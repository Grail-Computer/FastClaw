use askama::Template;

use crate::models::Task;

#[derive(Template)]
#[template(path = "status.html")]
pub struct StatusTemplate {
    pub active: &'static str,
    pub slack_signing_secret_set: bool,
    pub slack_bot_token_set: bool,
    pub queue_depth: i64,
    pub permissions_mode: String,
}

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    pub active: &'static str,
    pub context_last_n: i64,
    pub permissions_mode: String,
}

#[derive(Template)]
#[template(path = "tasks.html")]
pub struct TasksTemplate {
    pub active: &'static str,
    pub tasks: Vec<TaskRow>,
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: i64,
    pub status: String,
    pub channel_id: String,
    pub thread_ts: String,
    pub prompt_text: String,
    pub created_at: String,
}

impl From<Task> for TaskRow {
    fn from(t: Task) -> Self {
        Self {
            id: t.id,
            status: t.status,
            channel_id: t.channel_id,
            thread_ts: t.thread_ts,
            prompt_text: t.prompt_text,
            created_at: format!("{}", t.created_at),
        }
    }
}

