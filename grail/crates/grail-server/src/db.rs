use std::path::Path;

use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::models::{PermissionsMode, Settings, Task};

pub async fn init_sqlite(db_path: &Path) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .with_context(|| format!("connect sqlite at {}", db_path.display()))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("run migrations")?;

    Ok(pool)
}

pub async fn get_settings(pool: &SqlitePool) -> anyhow::Result<Settings> {
    let row = sqlx::query(
        r#"
        SELECT
          context_last_n,
          model,
          reasoning_effort,
          reasoning_summary,
          permissions_mode,
          allow_slack_mcp,
          allow_context_writes,
          updated_at
        FROM settings
        WHERE id = 1
        "#,
    )
    .fetch_one(pool)
    .await
    .context("select settings")?;

    Ok(Settings {
        context_last_n: row.get::<i64, _>("context_last_n"),
        model: row.get::<Option<String>, _>("model"),
        reasoning_effort: row.get::<Option<String>, _>("reasoning_effort"),
        reasoning_summary: row.get::<Option<String>, _>("reasoning_summary"),
        permissions_mode: PermissionsMode::from_db_str(row.get::<String, _>("permissions_mode").as_str()),
        allow_slack_mcp: row.get::<i64, _>("allow_slack_mcp") != 0,
        allow_context_writes: row.get::<i64, _>("allow_context_writes") != 0,
        updated_at: row.get::<i64, _>("updated_at"),
    })
}

pub async fn update_settings(
    pool: &SqlitePool,
    context_last_n: i64,
    permissions_mode: PermissionsMode,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE settings
        SET context_last_n = ?1,
            permissions_mode = ?2,
            updated_at = unixepoch()
        WHERE id = 1
        "#,
    )
    .bind(context_last_n)
    .bind(permissions_mode.as_db_str())
    .execute(pool)
    .await
    .context("update settings")?;
    Ok(())
}

pub async fn try_mark_event_processed(
    pool: &SqlitePool,
    workspace_id: &str,
    event_id: &str,
) -> anyhow::Result<bool> {
    let res = sqlx::query(
        r#"
        INSERT INTO processed_events (workspace_id, event_id, processed_at)
        VALUES (?1, ?2, unixepoch())
        ON CONFLICT(workspace_id, event_id) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(event_id)
    .execute(pool)
    .await
    .context("insert processed event")?;

    Ok(res.rows_affected() == 1)
}

pub async fn enqueue_task(
    pool: &SqlitePool,
    workspace_id: &str,
    channel_id: &str,
    thread_ts: &str,
    event_ts: &str,
    requested_by_user_id: &str,
    prompt_text: &str,
) -> anyhow::Result<i64> {
    let res = sqlx::query(
        r#"
        INSERT INTO tasks (
          status,
          workspace_id,
          channel_id,
          thread_ts,
          event_ts,
          requested_by_user_id,
          prompt_text,
          created_at
        )
        VALUES ('queued', ?1, ?2, ?3, ?4, ?5, ?6, unixepoch())
        "#,
    )
    .bind(workspace_id)
    .bind(channel_id)
    .bind(thread_ts)
    .bind(event_ts)
    .bind(requested_by_user_id)
    .bind(prompt_text)
    .execute(pool)
    .await
    .context("insert task")?;

    Ok(res.last_insert_rowid())
}

pub async fn claim_next_task(pool: &SqlitePool) -> anyhow::Result<Option<Task>> {
    let mut tx = pool.begin().await.context("begin tx")?;

    let row_opt = sqlx::query(
        r#"
        SELECT
          id,
          status,
          workspace_id,
          channel_id,
          thread_ts,
          event_ts,
          requested_by_user_id,
          prompt_text,
          result_text,
          error_text,
          created_at,
          started_at,
          finished_at
        FROM tasks
        WHERE status = 'queued'
        ORDER BY created_at ASC, id ASC
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut *tx)
    .await
    .context("select next task")?;

    let Some(row) = row_opt else {
        tx.commit().await.context("commit tx")?;
        return Ok(None);
    };

    let id = row.get::<i64, _>("id");
    let updated = sqlx::query(
        r#"
        UPDATE tasks
        SET status = 'running',
            started_at = unixepoch()
        WHERE id = ?1
          AND status = 'queued'
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .context("mark task running")?;

    if updated.rows_affected() != 1 {
        tx.commit().await.context("commit tx")?;
        return Ok(None);
    }

    tx.commit().await.context("commit tx")?;

    Ok(Some(Task {
        id,
        status: "running".to_string(),
        workspace_id: row.get::<String, _>("workspace_id"),
        channel_id: row.get::<String, _>("channel_id"),
        thread_ts: row.get::<String, _>("thread_ts"),
        event_ts: row.get::<String, _>("event_ts"),
        requested_by_user_id: row.get::<String, _>("requested_by_user_id"),
        prompt_text: row.get::<String, _>("prompt_text"),
        result_text: row.get::<Option<String>, _>("result_text"),
        error_text: row.get::<Option<String>, _>("error_text"),
        created_at: row.get::<i64, _>("created_at"),
        started_at: Some(chrono::Utc::now().timestamp()),
        finished_at: row.get::<Option<i64>, _>("finished_at"),
    }))
}

pub async fn complete_task_success(
    pool: &SqlitePool,
    task_id: i64,
    result_text: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE tasks
        SET status = 'succeeded',
            result_text = ?2,
            finished_at = unixepoch()
        WHERE id = ?1
        "#,
    )
    .bind(task_id)
    .bind(result_text)
    .execute(pool)
    .await
    .context("complete task success")?;
    Ok(())
}

pub async fn complete_task_failure(
    pool: &SqlitePool,
    task_id: i64,
    error_text: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE tasks
        SET status = 'failed',
            error_text = ?2,
            finished_at = unixepoch()
        WHERE id = ?1
        "#,
    )
    .bind(task_id)
    .bind(error_text)
    .execute(pool)
    .await
    .context("complete task failure")?;
    Ok(())
}

pub async fn list_recent_tasks(pool: &SqlitePool, limit: i64) -> anyhow::Result<Vec<Task>> {
    let rows = sqlx::query(
        r#"
        SELECT
          id,
          status,
          workspace_id,
          channel_id,
          thread_ts,
          event_ts,
          requested_by_user_id,
          prompt_text,
          result_text,
          error_text,
          created_at,
          started_at,
          finished_at
        FROM tasks
        ORDER BY created_at DESC, id DESC
        LIMIT ?1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("list tasks")?;

    Ok(rows
        .into_iter()
        .map(|row| Task {
            id: row.get::<i64, _>("id"),
            status: row.get::<String, _>("status"),
            workspace_id: row.get::<String, _>("workspace_id"),
            channel_id: row.get::<String, _>("channel_id"),
            thread_ts: row.get::<String, _>("thread_ts"),
            event_ts: row.get::<String, _>("event_ts"),
            requested_by_user_id: row.get::<String, _>("requested_by_user_id"),
            prompt_text: row.get::<String, _>("prompt_text"),
            result_text: row.get::<Option<String>, _>("result_text"),
            error_text: row.get::<Option<String>, _>("error_text"),
            created_at: row.get::<i64, _>("created_at"),
            started_at: row.get::<Option<i64>, _>("started_at"),
            finished_at: row.get::<Option<i64>, _>("finished_at"),
        })
        .collect())
}

