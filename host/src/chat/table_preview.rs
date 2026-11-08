//! Bounded, read-only delimited data formatting. The original source is never modified or evaluated.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::fmt::Write as _;

pub fn format(name: &str, text: &str) -> Result<Value> {
    ensure!(text.len() <= 64 * 1024, "table previews are limited to 64 KiB; use Source for the full file");
    let delimiter = match name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase().as_str() {
        "csv" => ',',
        "tsv" => '\t',
        _ => anyhow::bail!("only CSV and TSV have table previews"),
    };
    let mut input = text.trim_start_matches('\u{feff}').chars().peekable();
    let (mut cell, mut row, mut rows) = (String::new(), Vec::new(), Vec::new());
    let (mut quoted, mut closed, mut truncated, mut cell_len) = (false, false, false, 0);
    while let Some(c) = input.next() {
        if quoted {
            if c == '"' {
                if input.peek() == Some(&'"') {
                    input.next();
                    append(&mut cell, '"', &mut cell_len, &mut truncated);
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                append(&mut cell, c, &mut cell_len, &mut truncated);
            }
            continue;
        }
        if c == delimiter || c == '\n' || c == '\r' {
            if row.len() < 20 {
                row.push(std::mem::take(&mut cell));
            } else {
                cell.clear();
                truncated = true;
            }
            cell_len = 0;
            closed = false;
            if c != delimiter {
                if c == '\r' && input.peek() == Some(&'\n') {
                    input.next();
                }
                rows.push(std::mem::take(&mut row));
                if rows.len() == 100 {
                    truncated |= input.peek().is_some();
                    break;
                }
            }
        } else if c == '"' && cell.is_empty() && !closed {
            quoted = true;
        } else {
            ensure!(!closed || c.is_whitespace(), "invalid characters after a quoted field; use Source");
            ensure!(c != '"', "invalid quote in an unquoted field; use Source");
            if !closed {
                append(&mut cell, c, &mut cell_len, &mut truncated);
            }
        }
    }
    ensure!(!quoted, "unterminated quoted field; use Source");
    if rows.len() < 100 && (!row.is_empty() || !cell.is_empty() || closed) {
        if row.len() < 20 {
            row.push(cell);
        } else {
            truncated = true;
        }
        rows.push(row);
    }
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut markdown = String::new();
    if columns > 0 {
        writeln!(markdown, "| {} |", (1..=columns).map(|n| n.to_string()).collect::<Vec<_>>().join(" | "))?;
        writeln!(markdown, "| {} |", vec!["---"; columns].join(" | "))?;
        for row in rows {
            let cells = (0..columns).map(|i| escape(row.get(i).map_or("", String::as_str))).collect::<Vec<_>>();
            writeln!(markdown, "| {} |", cells.join(" | "))?;
        }
    }
    if markdown.is_empty() {
        markdown = "No rows".into();
    }
    Ok(json!({"markdown": markdown, "truncated": truncated}))
}

fn append(cell: &mut String, c: char, len: &mut usize, truncated: &mut bool) {
    if *len < 256 {
        cell.push(c);
        *len += 1;
    } else {
        *truncated = true;
    }
}

fn escape(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if c == '\n' || c == '\r' {
            out.push_str(" ↵ ");
        } else if !c.is_control() {
            if "\\`*_{}[]()#+-.!|>~".contains(c) {
                out.push('\\');
            }
            out.push(c);
        }
    }
    out
}
