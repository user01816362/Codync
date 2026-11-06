//! Bounded, read-only reports authored by an agent and rendered as portable Markdown.
//!
//! This deliberately uses the normal message path: old clients and the terminal see the same
//! information. It is not A2UI and does not execute model-supplied markup or actions.
use anyhow::{Context as _, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;

const MAX_BYTES: usize = 32 * 1024;
const MAX_SECTIONS: usize = 24;
const MAX_ITEMS: usize = 30;
const MAX_SOURCES: usize = 30;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    version: u8,
    title: String,
    sections: Vec<Section>,
    #[serde(default)]
    sources: Vec<Source>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    title: String,
    url: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    label: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Section {
    kind: String,
    heading: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    items: Vec<Item>,
    /// One-based indexes into `Report.sources`, supplied explicitly by the agent.
    #[serde(default)]
    citations: Vec<usize>,
}

pub(super) fn tool() -> Value {
    json!({
        "name": "send_report",
        "description": "Send a compact structured report as a normal chat message. Use only for content that benefits from sections, comparisons, timelines, data tables or cited sources. Cite only sources you actually used. Lists and data are read-only; do not use this tool for forms or actions.",
        "inputSchema": {
            "type": "object", "additionalProperties": false,
            "properties": {
                "version": {"type": "integer", "const": 1},
                "title": {"type": "string", "minLength": 1, "maxLength": 200},
                "sections": {"type": "array", "minItems": 1, "maxItems": MAX_SECTIONS, "items": {
                    "type": "object", "additionalProperties": false,
                    "properties": {
                        "kind": {"type": "string", "enum": ["text", "comparison", "timeline", "data"]},
                        "heading": {"type": "string", "minLength": 1, "maxLength": 200},
                        "text": {"type": "string", "maxLength": 6000},
                        "items": {"type": "array", "maxItems": MAX_ITEMS, "items": {"type": "object", "additionalProperties": false, "properties": {"label": {"type": "string"}, "value": {"type": "string"}}, "required": ["label", "value"]}},
                        "citations": {"type": "array", "maxItems": MAX_SOURCES, "items": {"type": "integer", "minimum": 1}}
                    }, "required": ["kind", "heading"]
                }},
                "sources": {"type": "array", "maxItems": MAX_SOURCES, "items": {"type": "object", "additionalProperties": false, "properties": {"title": {"type": "string"}, "url": {"type": "string", "description": "A source actually used; http or https only."}}, "required": ["title", "url"]}}
            }, "required": ["version", "title", "sections"]
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": false}
    })
}

/// Validate the entire report before delivery; never execute HTML or follow source URLs.
pub(super) fn render(args: &Value) -> Result<String> {
    ensure!(serde_json::to_vec(args)?.len() <= MAX_BYTES, "report exceeds 32 KiB");
    let report: Report = serde_json::from_value(args.clone()).context("invalid report")?;
    ensure!(report.version == 1, "unsupported report version");
    ensure!(!report.sections.is_empty() && report.sections.len() <= MAX_SECTIONS, "invalid section count");
    ensure!(report.sources.len() <= MAX_SOURCES, "too many sources");
    let mut source_urls = Vec::new();
    for source in &report.sources {
        check_text(&source.title, 200)?;
        ensure!(source.url.len() <= 2048, "source URL is too long");
        let url = url::Url::parse(&source.url).context("invalid source URL")?;
        ensure!(matches!(url.scheme(), "http" | "https") && url.host_str().is_some(), "source URL must be http or https");
        ensure!(url.username().is_empty() && url.password().is_none(), "source URL must not contain credentials");
        // Parentheses delimit links in the desktop's Markdown parser. Percent-encode them
        // in the destination while retaining the same URL for browsers and native clients.
        source_urls.push(url.as_str().replace('(', "%28").replace(')', "%29"));
    }
    check_text(&report.title, 200)?;
    let mut result = format!("# {}\n", escape(&report.title));
    for section in &report.sections {
        check_text(&section.heading, 200)?;
        ensure!(section.text.len() <= 6000 && !section.text.chars().any(|c| c.is_control() && c != '\n' && c != '\t'), "invalid section text");
        ensure!(section.items.len() <= MAX_ITEMS && section.citations.len() <= MAX_SOURCES, "too many section items or citations");
        let mut cited = HashSet::new();
        for &citation in &section.citations {
            ensure!(citation > 0 && citation <= report.sources.len(), "citation references a missing source");
            ensure!(cited.insert(citation), "duplicate citation");
        }
        result.push_str(&format!("\n## {}\n\n", escape(&section.heading)));
        if !section.text.trim().is_empty() {
            result.push_str(&escape(&section.text));
            result.push('\n');
        }
        match section.kind.as_str() {
            "text" => ensure!(section.items.is_empty() && !section.text.trim().is_empty(), "text sections require text and no items"),
            "comparison" | "timeline" | "data" => {
                ensure!(!section.items.is_empty(), "section requires items");
                for item in &section.items {
                    check_text(&item.label, 500)?;
                    check_text(&item.value, 2000)?;
                    result.push_str(&format!("\n- **{}:** {}", escape(&item.label), escape(&item.value)));
                }
                result.push('\n');
            }
            _ => bail!("unsupported section type"),
        }
        if !section.citations.is_empty() {
            result.push_str("\nSources: ");
            for (index, citation) in section.citations.iter().enumerate() {
                if index > 0 { result.push_str(", "); }
                result.push_str(&format!("[{}]({})", citation, source_urls[citation - 1]));
            }
            result.push('\n');
        }
    }
    if !report.sources.is_empty() {
        result.push_str("\n## Sources\n");
        for (index, source) in report.sources.iter().enumerate() {
            result.push_str(&format!("\n{}. [{}]({})", index + 1, escape(&source.title), source_urls[index]));
        }
    }
    ensure!(result.len() <= MAX_BYTES, "rendered report exceeds 32 KiB");
    Ok(result)
}

fn check_text(value: &str, max: usize) -> Result<()> {
    ensure!(!value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control), "invalid report text");
    Ok(())
}

/// Values are plain text, not agent-provided Markdown or HTML. Escape syntax across clients.
fn escape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '<' | '>' | '(' | ')' | '#' | '+' | '-' | '.' | '!' | '|') {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_only_explicit_sources_and_supported_sections() {
        let input = json!({"version": 1, "title": "Research", "sources": [{"title": "Official", "url": "https://example.com/docs"}], "sections": [
            {"kind": "text", "heading": "Finding", "text": "Result <script>", "citations": [1]},
            {"kind": "comparison", "heading": "Options", "items": [{"label": "A", "value": "Good"}]},
            {"kind": "timeline", "heading": "Steps", "items": [{"label": "Today", "value": "Done"}]},
            {"kind": "data", "heading": "Data", "items": [{"label": "Count", "value": "42"}]}
        ]});
        let text = render(&input).unwrap();
        assert!(text.contains("\\<script\\>"));
        assert!(text.contains("[1](https://example.com/docs)"));
        assert!(text.contains("**Count:** 42"));
    }

    #[test]
    fn rejects_unknown_kinds_invalid_citations_and_unsafe_urls() {
        let mut input = json!({"version": 1, "title": "Report", "sections": [{"kind": "text", "heading": "Finding", "text": "Hello"}]});
        input["version"] = json!(2);
        assert!(render(&input).is_err());
        input["version"] = json!(1);
        input["sections"][0]["kind"] = json!("form");
        assert!(render(&input).is_err());
        input["sections"][0]["kind"] = json!("text");
        input["sections"][0]["citations"] = json!([1]);
        assert!(render(&input).is_err());
        input["sources"] = json!([{ "title": "Unsafe", "url": "javascript:alert(1)" }]);
        assert!(render(&input).is_err());
        input["sources"] = json!([{ "title": "Unsafe", "url": "https://user:pass@example.com/" }]);
        assert!(render(&input).is_err());
    }

    #[test]
    fn rejects_oversize_and_unexpected_fields_before_delivery() {
        let mut input = json!({"version": 1, "title": "Report", "sections": [{"kind": "text", "heading": "Finding", "text": "Hello"}]});
        input["sections"][0]["script"] = json!("alert(1)");
        assert!(render(&input).is_err());
        input["sections"][0].as_object_mut().unwrap().remove("script");
        input["sections"][0]["text"] = json!("x".repeat(MAX_BYTES));
        assert!(render(&input).is_err());
    }
}
