pub fn normalize_cron_expr(expr: &str) -> anyhow::Result<String> {
    let parts: Vec<&str> = expr.split_whitespace().filter(|p| !p.is_empty()).collect();
    match parts.len() {
        // Standard 5-field cron: min hour dom month dow
        // Expand to 7 fields for the `cron` crate: sec min hour dom month dow year
        5 => Ok(format!("0 {} *", parts.join(" "))),
        // 6-field cron: sec min hour dom month dow
        6 => Ok(format!("{} *", parts.join(" "))),
        // Already in 7-field form
        7 => Ok(parts.join(" ")),
        _ => anyhow::bail!("cron expr must have 5, 6, or 7 fields"),
    }
}
