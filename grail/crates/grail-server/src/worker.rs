use std::time::Duration;

use tracing::{info, warn};

use crate::db;
use crate::AppState;

pub async fn worker_loop(state: AppState) {
    loop {
        match db::claim_next_task(&state.pool).await {
            Ok(Some(task)) => {
                let task_id = task.id;
                let result = process_task(&state, &task).await;
                match result {
                    Ok(text) => {
                        if let Err(err) = db::complete_task_success(&state.pool, task_id, &text).await
                        {
                            warn!(error = %err, task_id, "failed to mark task succeeded");
                        }
                    }
                    Err(err) => {
                        let msg = format!("{err:#}");
                        warn!(error = %msg, task_id, "task failed");
                        let _ = db::complete_task_failure(&state.pool, task_id, &msg).await;
                    }
                }
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_millis(750)).await;
            }
            Err(err) => {
                warn!(error = %err, "worker loop db error");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

async fn process_task(state: &AppState, task: &crate::models::Task) -> anyhow::Result<String> {
    let settings = db::get_settings(&state.pool).await?;

    let Some(slack) = state.slack.as_ref() else {
        anyhow::bail!("SLACK_BOT_TOKEN is not configured");
    };

    // Stub: fetch context and echo. Codex integration comes next.
    let ctx = if task.thread_ts != task.event_ts {
        slack.fetch_thread_replies(
            &task.channel_id,
            &task.thread_ts,
            &task.event_ts,
            settings.context_last_n,
        )
        .await?
    } else {
        slack.fetch_channel_history(&task.channel_id, &task.event_ts, settings.context_last_n)
            .await?
    };

    let mut summary = String::new();
    summary.push_str("Working on it.\n\n");
    summary.push_str(&format!("Request: {}\n", task.prompt_text.trim()));
    summary.push_str(&format!(
        "Mode: {}\n",
        settings.permissions_mode.as_db_str()
    ));
    summary.push_str(&format!("Context messages: {}\n\n", ctx.len()));

    for m in ctx.into_iter().take(20) {
        let who = m.user.as_deref().unwrap_or("unknown");
        let text = m.text.unwrap_or_default().replace('\n', " ");
        summary.push_str(&format!("- {who}: {text}\n"));
    }

    // Reply in thread.
    slack.post_message(&task.channel_id, &task.thread_ts, &summary)
        .await?;

    info!(task_id = task.id, "replied to slack");
    Ok(summary)
}
