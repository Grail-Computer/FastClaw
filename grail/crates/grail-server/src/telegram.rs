use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TelegramClient {
    http: reqwest::Client,
    bot_token: String,
}

impl TelegramClient {
    pub fn new(http: reqwest::Client, bot_token: String) -> Self {
        Self { http, bot_token }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.bot_token, method)
    }

    pub async fn get_me(&self) -> anyhow::Result<TelegramUser> {
        let resp: TelegramApiResponse<TelegramUser> = self
            .http
            .get(self.api_url("getMe"))
            .send()
            .await
            .context("telegram getMe request")?
            .json()
            .await
            .context("telegram getMe decode")?;

        if !resp.ok {
            anyhow::bail!(
                "telegram getMe failed: {}",
                resp.description
                    .unwrap_or_else(|| "unknown_error".to_string())
            );
        }
        resp.result.context("telegram getMe missing result")
    }

    pub async fn send_message(
        &self,
        chat_id: &str,
        reply_to_message_id: Option<i64>,
        text: &str,
    ) -> anyhow::Result<Vec<i64>> {
        const MAX_CHARS: usize = 3900;

        #[derive(Serialize)]
        struct Req<'a> {
            chat_id: &'a str,
            text: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            reply_to_message_id: Option<i64>,
            // Avoid errors if the message we're replying to is deleted.
            allow_sending_without_reply: bool,
            disable_web_page_preview: bool,
        }

        let mut ids = Vec::new();
        for chunk in split_telegram_text(text, MAX_CHARS) {
            let resp: TelegramApiResponse<TelegramMessage> = self
                .http
                .post(self.api_url("sendMessage"))
                .json(&Req {
                    chat_id,
                    text: &chunk,
                    reply_to_message_id,
                    allow_sending_without_reply: true,
                    disable_web_page_preview: true,
                })
                .send()
                .await
                .context("telegram sendMessage request")?
                .json()
                .await
                .context("telegram sendMessage decode")?;

            if !resp.ok {
                anyhow::bail!(
                    "telegram sendMessage failed: {}",
                    resp.description
                        .unwrap_or_else(|| "unknown_error".to_string())
                );
            }
            if let Some(msg) = resp.result {
                ids.push(msg.message_id);
            }
        }
        Ok(ids)
    }
}

fn split_telegram_text(text: &str, max_chars: usize) -> Vec<String> {
    let t = text.trim();
    if t.is_empty() {
        return vec!["(empty)".to_string()];
    }
    if t.chars().count() <= max_chars {
        return vec![t.to_string()];
    }

    let mut out = Vec::new();
    let mut buf = String::new();
    for line in t.split_inclusive('\n') {
        if buf.chars().count() + line.chars().count() > max_chars && !buf.is_empty() {
            out.push(buf.trim().to_string());
            buf.clear();
        }
        if line.chars().count() > max_chars {
            // Hard split long lines.
            let mut current = String::new();
            for ch in line.chars() {
                current.push(ch);
                if current.chars().count() >= max_chars {
                    out.push(current.trim().to_string());
                    current.clear();
                }
            }
            if !current.trim().is_empty() {
                buf.push_str(&current);
            }
            continue;
        }
        buf.push_str(line);
    }
    if !buf.trim().is_empty() {
        out.push(buf.trim().to_string());
    }
    out
}

#[derive(Debug, Deserialize)]
pub struct TelegramApiResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramInboundMessage>,
    pub edited_message: Option<TelegramInboundMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramInboundMessage {
    pub message_id: i64,
    pub date: i64,
    pub chat: TelegramChat,
    pub from: Option<TelegramUser>,
    pub text: Option<String>,
    #[serde(default)]
    pub entities: Vec<TelegramEntity>,
    pub reply_to_message: Option<Box<TelegramReplyToMessage>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramReplyToMessage {
    pub message_id: i64,
    pub from: Option<TelegramUser>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String, // private | group | supergroup | channel
    pub title: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    #[serde(default)]
    pub is_bot: bool,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramEntity {
    #[serde(rename = "type")]
    pub kind: String, // mention | bot_command | ...
    pub offset: i64,
    pub length: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
}
