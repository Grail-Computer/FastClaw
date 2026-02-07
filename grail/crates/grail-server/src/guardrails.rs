use anyhow::Context;
use regex::Regex;

use crate::models::GuardrailRule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    RequireApproval,
    Deny,
}

pub fn decision_from_action(action: &str) -> Decision {
    match action {
        "allow" => Decision::Allow,
        "deny" => Decision::Deny,
        _ => Decision::RequireApproval,
    }
}

pub fn rule_matches(rule: &GuardrailRule, text: &str) -> anyhow::Result<bool> {
    if !rule.enabled {
        return Ok(false);
    }
    match rule.pattern_kind.as_str() {
        "exact" => Ok(text.trim() == rule.pattern.trim()),
        "substring" => Ok(text.contains(rule.pattern.trim())),
        "regex" => {
            let re = Regex::new(rule.pattern.trim()).context("compile guardrail regex")?;
            Ok(re.is_match(text))
        }
        other => anyhow::bail!("unknown pattern_kind: {other}"),
    }
}

pub fn validate_rule(rule: &GuardrailRule) -> anyhow::Result<()> {
    anyhow::ensure!(!rule.id.trim().is_empty(), "guardrail id is required");
    anyhow::ensure!(!rule.name.trim().is_empty(), "guardrail name is required");
    anyhow::ensure!(!rule.kind.trim().is_empty(), "guardrail kind is required");
    anyhow::ensure!(
        !rule.pattern_kind.trim().is_empty(),
        "guardrail pattern_kind is required"
    );
    anyhow::ensure!(
        !rule.pattern.trim().is_empty(),
        "guardrail pattern is required"
    );
    anyhow::ensure!(
        !rule.action.trim().is_empty(),
        "guardrail action is required"
    );

    // Validate the pattern eagerly.
    if rule.pattern_kind == "regex" {
        let _ = Regex::new(rule.pattern.trim()).context("compile guardrail regex")?;
    }
    Ok(())
}

pub async fn evaluate_command_guardrails(
    rules: &[GuardrailRule],
    command: &str,
) -> anyhow::Result<(Decision, Option<GuardrailRule>)> {
    // Rules should already be ordered by priority ASC.
    for r in rules {
        if !r.enabled {
            continue;
        }
        if rule_matches(r, command)? {
            return Ok((decision_from_action(r.action.as_str()), Some(r.clone())));
        }
    }
    Ok((Decision::Allow, None))
}
