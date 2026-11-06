//! Explicit agent-reported research activity, using the existing live plan transport.
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub(super) fn tool() -> Value {
    json!({
        "name": "research_progress",
        "description": "Update the live research checklist in working details. Report only steps you actually plan or perform; completed means you performed the step, not independent verification of its findings. Use send_report for findings and explicit citations. Does not send a final reply.",
        "inputSchema": {"type": "object", "additionalProperties": false, "properties": {
            "steps": {"type": "array", "minItems": 1, "maxItems": 20, "items": {
                "type": "object", "additionalProperties": false,
                "properties": {"content": {"type": "string", "minLength": 1, "maxLength": 300},
                    "status": {"type": "string", "enum": ["pending", "in_progress", "completed"]}},
                "required": ["content", "status"]
            }}
        }, "required": ["steps"]},
        "annotations": {"readOnlyHint": false, "idempotentHint": true}
    })
}

pub(super) fn validate(args: &Value) -> Result<Value> {
    ensure!(serde_json::to_vec(args)?.len() <= 16 * 1024, "research progress exceeds 16 KiB");
    let steps = args["steps"].as_array().ok_or_else(|| anyhow::anyhow!("steps is required"))?;
    ensure!(!steps.is_empty() && steps.len() <= 20, "research requires 1–20 steps");
    let mut entries = Vec::with_capacity(steps.len());
    for step in steps {
        let text = step["content"].as_str().unwrap_or_default();
        let status = step["status"].as_str().unwrap_or_default();
        ensure!(
            !text.trim().is_empty() && text.len() <= 300 && !text.chars().any(char::is_control),
            "invalid research step"
        );
        ensure!(matches!(status, "pending" | "in_progress" | "completed"), "invalid research status");
        entries.push(json!({"content": text, "status": status, "priority": "medium"}));
    }
    Ok(json!(entries))
}
