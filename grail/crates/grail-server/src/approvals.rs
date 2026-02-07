use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Context;
use serde::Deserialize;
use serde_json::json;
use tracing::{info, warn};

use crate::db;
use crate::guardrails::{evaluate_command_guardrails, validate_rule, Decision};
use crate::models::{Approval, CronJob, GuardrailRule, PermissionsMode, Settings, Task};
use crate::slack::SlackClient;
use crate::telegram::TelegramClient;
use crate::AppState;

const APPROVAL_TIMEOUT_SECS: u64 = 15 * 60;

pub async fn handle_command_execution_request(
    state: &AppState,
    settings: &Settings,
    cwd: &Path,
    task: &Task,
    params: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    // Respect the global permissions switch first.
    if settings.permissions_mode != PermissionsMode::Full {
        return Ok(json!({ "decision": "decline" }));
    }

    // Require commands to run under our configured cwd (avoid touching app code).
    // Be strict: reject any cwd that contains `..` to avoid path traversal via lexical paths.
    let raw = params.get("cwd").and_then(|v| v.as_str()).unwrap_or("");
    let mut cmd_cwd = if raw.trim().is_empty() {
        cwd.to_path_buf()
    } else {
        PathBuf::from(raw.trim())
    };
    if !cmd_cwd.is_absolute() {
        cmd_cwd = cwd.join(cmd_cwd);
    }
    let Some(cmd_cwd) = clean_path_no_parent(&cmd_cwd) else {
        return Ok(json!({ "decision": "decline" }));
    };
    if !cmd_cwd.starts_with(cwd) {
        return Ok(json!({ "decision": "decline" }));
    }

    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if command.is_empty() {
        return Ok(json!({ "decision": "decline" }));
    }

    match settings.command_approval_mode.as_str() {
        "auto" => return Ok(json!({ "decision": "accept" })),
        "always_ask" => {}
        _ => {
            // guardrails (default)
            let rules = db::list_guardrail_rules(&state.pool, Some("command"), 500).await?;
            let (decision, matched) = evaluate_command_guardrails(&rules, &command).await?;
            match decision {
                Decision::Allow => return Ok(json!({ "decision": "accept" })),
                Decision::Deny => {
                    warn!(
                        command = %command,
                        matched_rule = matched.as_ref().map(|r| r.id.as_str()).unwrap_or(""),
                        "command denied by guardrail"
                    );
                    return Ok(json!({ "decision": "decline" }));
                }
                Decision::RequireApproval => {}
            }
        }
    }

    // Need human approval.
    let approval_id = random_id("appr");
    let now = chrono::Utc::now().timestamp();

    let details = json!({
        "command": command,
        "cwd": cmd_cwd.to_string_lossy(),
        "reason": params.get("reason").cloned().unwrap_or(json!(null)),
    });

    let approval = Approval {
        id: approval_id.clone(),
        kind: "command_execution".to_string(),
        status: "pending".to_string(),
        decision: None,
        workspace_id: Some(task.workspace_id.clone()),
        channel_id: Some(task.channel_id.clone()),
        thread_ts: Some(task.thread_ts.clone()),
        requested_by_user_id: Some(task.requested_by_user_id.clone()),
        details_json: details.to_string(),
        created_at: now,
        updated_at: now,
        resolved_at: None,
    };
    db::insert_approval(&state.pool, &approval).await?;

    let approve_hint = if task.provider == "slack" {
        format!("@{} approve {}", settings.agent_name, approval_id)
    } else {
        format!("approve {}", approval_id)
    };
    let always_hint = if task.provider == "slack" {
        format!("@{} always {}", settings.agent_name, approval_id)
    } else {
        format!("always {}", approval_id)
    };
    let deny_hint = if task.provider == "slack" {
        format!("@{} deny {}", settings.agent_name, approval_id)
    } else {
        format!("deny {}", approval_id)
    };

    let mut msg = String::new();
    msg.push_str("*Approval required*\n");
    msg.push_str(&format!(
        "Proposed command in `{}`:\n```\n{}\n```\n",
        cmd_cwd.to_string_lossy(),
        crate::secrets::redact_secrets(
            details
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .0
    ));
    if let Some(reason) = params.get("reason").and_then(|v| v.as_str()) {
        if !reason.trim().is_empty() {
            msg.push_str(&format!("Reason: {reason}\n"));
        }
    }
    msg.push_str("Reply:\n");
    msg.push_str(&format!("- `{}` (once)\n", approve_hint));
    msg.push_str(&format!("- `{}` (remember)\n", always_hint));
    msg.push_str(&format!("- `{}`\n", deny_hint));

    match task.provider.as_str() {
        "slack" => {
            if let Ok(Some(token)) = crate::secrets::load_slack_bot_token_opt(state).await {
                let slack = SlackClient::new(state.http.clone(), token);
                let blocks = json!([
                    { "type": "section", "text": { "type": "mrkdwn", "text": msg.trim() } },
                    { "type": "actions", "elements": [
                        { "type": "button", "text": { "type": "plain_text", "text": "Approve" }, "action_id": "grail_approve", "value": approval_id.clone() },
                        { "type": "button", "text": { "type": "plain_text", "text": "Always" }, "style": "primary", "action_id": "grail_always", "value": approval_id.clone() },
                        { "type": "button", "text": { "type": "plain_text", "text": "Deny" }, "style": "danger", "action_id": "grail_deny", "value": approval_id.clone() }
                    ] }
                ]);

                if let Err(err) = slack
                    .post_message_rich(
                        &task.channel_id,
                        thread_opt(&task.thread_ts),
                        msg.trim(),
                        blocks,
                    )
                    .await
                {
                    warn!(error = %err, "failed to post rich approval message; falling back to plain text");
                    let _ = slack
                        .post_message(&task.channel_id, thread_opt(&task.thread_ts), msg.trim())
                        .await;
                }
            } else {
                warn!("cannot request approval: SLACK_BOT_TOKEN missing");
            }
        }
        "telegram" => {
            if let Ok(Some(token)) = crate::secrets::load_telegram_bot_token_opt(state).await {
                let tg = TelegramClient::new(state.http.clone(), token);
                let reply_to = task.thread_ts.parse::<i64>().ok();
                let _ = tg
                    .send_message(&task.channel_id, reply_to, msg.trim())
                    .await;
            } else {
                warn!("cannot request approval: TELEGRAM_BOT_TOKEN missing");
            }
        }
        _ => {}
    }

    let deadline = Instant::now() + Duration::from_secs(APPROVAL_TIMEOUT_SECS);
    loop {
        if Instant::now() >= deadline {
            db::expire_approval(&state.pool, &approval_id).await?;
            return Ok(json!({ "decision": "decline" }));
        }

        let Some(a) = db::get_approval(&state.pool, &approval_id).await? else {
            // Shouldn't happen, but fail closed.
            return Ok(json!({ "decision": "decline" }));
        };

        match a.status.as_str() {
            "approved" => {
                let decision = a.decision.unwrap_or_else(|| "approve".to_string());
                if decision == "always" {
                    // Persist an allow rule for this exact command.
                    let now = chrono::Utc::now().timestamp();
                    let rule = GuardrailRule {
                        id: random_id("gr"),
                        name: format!("approved: {}", truncate(&command, 48)),
                        kind: "command".to_string(),
                        pattern_kind: "exact".to_string(),
                        pattern: command.clone(),
                        action: "allow".to_string(),
                        priority: 1,
                        enabled: true,
                        created_at: now,
                        updated_at: now,
                    };
                    if let Err(err) = validate_rule(&rule) {
                        warn!(error = %err, "failed to validate generated allow rule");
                    } else if let Err(err) = db::insert_guardrail_rule(&state.pool, &rule).await {
                        warn!(error = %err, "failed to persist allow rule from approval");
                    }
                }

                info!(approval_id = %approval_id, "approval granted");
                return Ok(json!({ "decision": "accept" }));
            }
            "denied" => {
                info!(approval_id = %approval_id, "approval denied");
                return Ok(json!({ "decision": "decline" }));
            }
            "expired" => return Ok(json!({ "decision": "decline" })),
            _ => {}
        }

        tokio::time::sleep(Duration::from_millis(750)).await;
    }
}

pub async fn handle_approval_command(
    state: &AppState,
    action: &str,
    approval_id: &str,
) -> anyhow::Result<Option<String>> {
    let decision = match action {
        "approve" => ("approved", "approve"),
        "always" => ("approved", "always"),
        "deny" => ("denied", "deny"),
        "cancel" => ("denied", "deny"),
        _ => return Ok(Some("Unknown approval action.".to_string())),
    };

    let changed = db::resolve_approval(&state.pool, approval_id, decision.0, decision.1).await?;
    if !changed {
        return Ok(Some(
            "Approval not found, already handled, or expired.".to_string(),
        ));
    }

    // Apply side effects for approved non-command approvals.
    if decision.0 == "approved" {
        if let Some(a) = db::get_approval(&state.pool, approval_id).await? {
            apply_approval_side_effects(state, &a).await?;
        }
    }

    Ok(Some(format!("Recorded: {action} {approval_id}")))
}

async fn apply_approval_side_effects(state: &AppState, approval: &Approval) -> anyhow::Result<()> {
    match approval.kind.as_str() {
        "guardrail_rule_add" => {
            let proposed: ProposedGuardrailRule =
                serde_json::from_str(&approval.details_json).context("parse guardrail proposal")?;
            let now = chrono::Utc::now().timestamp();
            let rule = GuardrailRule {
                id: proposed.id.unwrap_or_else(|| random_id("gr")),
                name: proposed.name,
                kind: proposed.kind,
                pattern_kind: proposed.pattern_kind,
                pattern: proposed.pattern,
                action: proposed.action,
                priority: proposed.priority.unwrap_or(100),
                enabled: proposed.enabled.unwrap_or(true),
                created_at: now,
                updated_at: now,
            };
            validate_rule(&rule)?;
            db::insert_guardrail_rule(&state.pool, &rule).await?;
        }
        "cron_job_add" => {
            let proposed: ProposedCronJob =
                serde_json::from_str(&approval.details_json).context("parse cron proposal")?;
            let now = chrono::Utc::now().timestamp();
            let job = CronJob {
                id: proposed.id.unwrap_or_else(|| random_id("cron")),
                name: proposed.name,
                enabled: proposed.enabled.unwrap_or(true),
                mode: proposed.mode.unwrap_or_else(|| "agent".to_string()),
                schedule_kind: proposed.schedule_kind,
                every_seconds: proposed.every_seconds,
                cron_expr: proposed.cron_expr,
                at_ts: proposed.at_ts,
                workspace_id: proposed.workspace_id,
                channel_id: proposed.channel_id,
                thread_ts: proposed.thread_ts.unwrap_or_default(),
                prompt_text: proposed.prompt_text,
                next_run_at: proposed.next_run_at,
                last_run_at: None,
                last_status: None,
                last_error: None,
                created_at: now,
                updated_at: now,
            };
            db::insert_cron_job(&state.pool, &job).await?;
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ProposedGuardrailRule {
    #[serde(default)]
    id: Option<String>,
    name: String,
    kind: String,
    pattern_kind: String,
    pattern: String,
    action: String,
    #[serde(default)]
    priority: Option<i64>,
    #[serde(default)]
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ProposedCronJob {
    #[serde(default)]
    id: Option<String>,
    name: String,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    mode: Option<String>,
    schedule_kind: String,
    #[serde(default)]
    every_seconds: Option<i64>,
    #[serde(default)]
    cron_expr: Option<String>,
    #[serde(default)]
    at_ts: Option<i64>,
    workspace_id: String,
    channel_id: String,
    #[serde(default)]
    thread_ts: Option<String>,
    prompt_text: String,
    #[serde(default)]
    next_run_at: Option<i64>,
}

fn thread_opt(thread_ts: &str) -> Option<&str> {
    let t = thread_ts.trim();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

fn clean_path_no_parent(p: &Path) -> Option<PathBuf> {
    use std::path::Component;

    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Prefix(pre) => out.push(pre.as_os_str()),
            Component::RootDir => out.push(c.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => return None,
            Component::Normal(seg) => out.push(seg),
        }
    }
    Some(out)
}

fn random_id(prefix: &str) -> String {
    let mut bytes = [0u8; 16];
    let mut rng = rand::rng();
    rand::RngCore::fill_bytes(&mut rng, &mut bytes);
    format!("{}_{}", prefix, hex::encode(bytes))
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.trim().replace('\n', " ");
    if s.len() <= max {
        s
    } else {
        format!(
            "{}…",
            s.chars().take(max.saturating_sub(1)).collect::<String>()
        )
    }
}
