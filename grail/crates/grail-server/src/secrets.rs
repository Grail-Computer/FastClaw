use anyhow::Context;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::db;
use crate::AppState;

fn normalize_nonempty(s: String) -> Option<String> {
    let v = s.trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

pub async fn load_openai_api_key_opt(state: &AppState) -> anyhow::Result<Option<String>> {
    if let Ok(v) = std::env::var("OPENAI_API_KEY") {
        if let Some(v) = normalize_nonempty(v) {
            return Ok(Some(v));
        }
    }

    let Some(crypto) = state.crypto.as_deref() else {
        return Ok(None);
    };
    let Some((nonce, ciphertext)) = db::read_secret(&state.pool, "openai_api_key").await? else {
        return Ok(None);
    };
    let plaintext = crypto.decrypt(b"openai_api_key", &nonce, &ciphertext)?;
    let s = String::from_utf8(plaintext).context("OPENAI_API_KEY not valid utf-8")?;
    Ok(normalize_nonempty(s))
}

pub async fn openai_api_key_configured(state: &AppState) -> anyhow::Result<bool> {
    Ok(load_openai_api_key_opt(state).await?.is_some())
}

pub async fn load_slack_bot_token_opt(state: &AppState) -> anyhow::Result<Option<String>> {
    if let Some(v) = state.config.slack_bot_token.as_deref() {
        if let Some(v) = normalize_nonempty(v.to_string()) {
            return Ok(Some(v));
        }
    }

    let Some(crypto) = state.crypto.as_deref() else {
        return Ok(None);
    };
    let Some((nonce, ciphertext)) = db::read_secret(&state.pool, "slack_bot_token").await? else {
        return Ok(None);
    };
    let plaintext = crypto.decrypt(b"slack_bot_token", &nonce, &ciphertext)?;
    let s = String::from_utf8(plaintext).context("SLACK_BOT_TOKEN not valid utf-8")?;
    Ok(normalize_nonempty(s))
}

pub async fn slack_bot_token_configured(state: &AppState) -> anyhow::Result<bool> {
    Ok(load_slack_bot_token_opt(state).await?.is_some())
}

pub async fn load_slack_signing_secret_opt(state: &AppState) -> anyhow::Result<Option<String>> {
    if let Some(v) = state.config.slack_signing_secret.as_deref() {
        if let Some(v) = normalize_nonempty(v.to_string()) {
            return Ok(Some(v));
        }
    }

    let Some(crypto) = state.crypto.as_deref() else {
        return Ok(None);
    };
    let Some((nonce, ciphertext)) = db::read_secret(&state.pool, "slack_signing_secret").await?
    else {
        return Ok(None);
    };
    let plaintext = crypto.decrypt(b"slack_signing_secret", &nonce, &ciphertext)?;
    let s = String::from_utf8(plaintext).context("SLACK_SIGNING_SECRET not valid utf-8")?;
    Ok(normalize_nonempty(s))
}

pub async fn slack_signing_secret_configured(state: &AppState) -> anyhow::Result<bool> {
    Ok(load_slack_signing_secret_opt(state).await?.is_some())
}

pub async fn load_telegram_bot_token_opt(state: &AppState) -> anyhow::Result<Option<String>> {
    if let Some(v) = state.config.telegram_bot_token.as_deref() {
        if let Some(v) = normalize_nonempty(v.to_string()) {
            return Ok(Some(v));
        }
    }

    let Some(crypto) = state.crypto.as_deref() else {
        return Ok(None);
    };
    let Some((nonce, ciphertext)) = db::read_secret(&state.pool, "telegram_bot_token").await?
    else {
        return Ok(None);
    };
    let plaintext = crypto.decrypt(b"telegram_bot_token", &nonce, &ciphertext)?;
    let s = String::from_utf8(plaintext).context("TELEGRAM_BOT_TOKEN not valid utf-8")?;
    Ok(normalize_nonempty(s))
}

pub async fn telegram_bot_token_configured(state: &AppState) -> anyhow::Result<bool> {
    Ok(load_telegram_bot_token_opt(state).await?.is_some())
}

pub async fn load_telegram_webhook_secret_opt(state: &AppState) -> anyhow::Result<Option<String>> {
    if let Some(v) = state.config.telegram_webhook_secret.as_deref() {
        if let Some(v) = normalize_nonempty(v.to_string()) {
            return Ok(Some(v));
        }
    }

    let Some(crypto) = state.crypto.as_deref() else {
        return Ok(None);
    };
    let Some((nonce, ciphertext)) = db::read_secret(&state.pool, "telegram_webhook_secret").await?
    else {
        return Ok(None);
    };
    let plaintext = crypto.decrypt(b"telegram_webhook_secret", &nonce, &ciphertext)?;
    let s = String::from_utf8(plaintext).context("TELEGRAM_WEBHOOK_SECRET not valid utf-8")?;
    Ok(normalize_nonempty(s))
}

pub async fn telegram_webhook_secret_configured(state: &AppState) -> anyhow::Result<bool> {
    Ok(load_telegram_webhook_secret_opt(state).await?.is_some())
}

pub async fn load_brave_search_api_key_opt(state: &AppState) -> anyhow::Result<Option<String>> {
    if let Ok(v) = std::env::var("BRAVE_SEARCH_API_KEY") {
        if let Some(v) = normalize_nonempty(v) {
            return Ok(Some(v));
        }
    }
    // Nanobot-compatible name.
    if let Ok(v) = std::env::var("BRAVE_API_KEY") {
        if let Some(v) = normalize_nonempty(v) {
            return Ok(Some(v));
        }
    }

    let Some(crypto) = state.crypto.as_deref() else {
        return Ok(None);
    };
    let Some((nonce, ciphertext)) = db::read_secret(&state.pool, "brave_search_api_key").await?
    else {
        return Ok(None);
    };
    let plaintext = crypto.decrypt(b"brave_search_api_key", &nonce, &ciphertext)?;
    let s = String::from_utf8(plaintext).context("BRAVE_SEARCH_API_KEY not valid utf-8")?;
    Ok(normalize_nonempty(s))
}

pub async fn brave_search_api_key_configured(state: &AppState) -> anyhow::Result<bool> {
    Ok(load_brave_search_api_key_opt(state).await?.is_some())
}

static SECRET_REDACTIONS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        // OpenAI API keys (including newer sk-proj- style).
        (
            Regex::new(r"\bsk-(?:proj-)?[A-Za-z0-9_-]{10,}\b").expect("regex"),
            "[REDACTED_OPENAI_KEY]",
        ),
        // Slack tokens.
        (
            Regex::new(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b").expect("regex"),
            "[REDACTED_SLACK_TOKEN]",
        ),
        (
            Regex::new(r"\bxapp-[A-Za-z0-9-]{10,}\b").expect("regex"),
            "[REDACTED_SLACK_APP_TOKEN]",
        ),
        // Telegram bot token.
        (
            Regex::new(r"\b\d{6,}:[A-Za-z0-9_-]{20,}\b").expect("regex"),
            "[REDACTED_TELEGRAM_TOKEN]",
        ),
        // Private keys.
        (
            Regex::new(
                r"(?s)-----BEGIN [A-Z ]+PRIVATE KEY-----.*?-----END [A-Z ]+PRIVATE KEY-----",
            )
            .expect("regex"),
            "-----BEGIN PRIVATE KEY-----\n[REDACTED_PRIVATE_KEY]\n-----END PRIVATE KEY-----",
        ),
    ]
});

/// Best-effort redaction to avoid leaking secrets into Slack/Telegram, memory, or context files.
pub fn redact_secrets(text: &str) -> (String, bool) {
    let mut out = text.to_string();
    let mut changed = false;
    for (re, repl) in SECRET_REDACTIONS.iter() {
        let next = re.replace_all(&out, *repl).to_string();
        if next != out {
            changed = true;
            out = next;
        }
    }
    (out, changed)
}
