use anyhow::Context;

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
