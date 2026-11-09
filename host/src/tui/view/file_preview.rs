//! A bounded file preview, rendered as Markdown or plain source.

use super::super::manage::FilePreviewSheet;
use super::super::md;
use super::buffer::{centered, frame_box, hline, put, put_line, u};
use super::style::theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
};

pub(super) fn file_preview(buf: &mut Buffer, area: Rect, view: &FilePreviewSheet) {
    let t = theme();
    let r = centered(area, 84, area.height.saturating_sub(4));
    let inner = frame_box(buf, r, t.text, t.panel, Some((&view.name, t.text)));
    if inner.height < 3 {
        return;
    }
    hline(buf, inner.x, inner.bottom() - 2, inner.width, t.dim.patch(t.panel));
    put(
        buf,
        inner.x,
        inner.bottom() - 1,
        inner.width,
        " ↑↓ scroll · / find · n next · v read/source · w wrap · c copy · r retry · esc",
        t.dim.patch(t.panel),
    );
    if view.searching || !view.query.is_empty() {
        put(
            buf,
            inner.x,
            inner.bottom() - 1,
            inner.width,
            &format!("Find: {} · enter finish · n next · esc cancel", view.query),
            t.text.patch(t.panel),
        );
    }
    let body = Rect::new(inner.x, inner.y, inner.width, inner.height - 2);
    let width = usize::from(body.width);
    if let Some(directory) = &view.directory {
        let rows = directory["entries"].as_array().map(Vec::as_slice).unwrap_or_default();
        let height = usize::from(body.height);
        view.max_scroll.set(rows.len().saturating_sub(1));
        view.page.set(height);
        let first = view.scroll.saturating_sub(height.saturating_sub(1));
        for (offset, item) in rows.iter().enumerate().skip(first).take(height) {
            let mark = if offset == view.scroll { ">" } else { " " };
            let suffix = if item["kind"] == "directory" { "/" } else { "" };
            let label = format!("{mark} {}{suffix}", item["name"].as_str().unwrap_or_default());
            put(buf, body.x, body.y + u(offset - first), body.width, &label, t.text.patch(t.panel));
        }
        if rows.is_empty() {
            put(buf, body.x, body.y, body.width, "This folder is empty.", t.dim);
        }
        put(
            buf,
            inner.x,
            inner.bottom() - 1,
            inner.width,
            if directory["truncated"] == true {
                "Limited listing · ↵ open · g root · H hidden · r retry · esc close"
            } else {
                "↑↓ select · ↵ open · g root · H hidden · r retry · esc close"
            },
            t.dim.patch(t.panel),
        );
        return;
    }
    let lines = match &view.content {
        None => vec![Line::from(Span::styled(" Loading…", t.dim))],
        Some(Err(error)) => vec![Line::from(Span::styled(format!(" {error}"), t.red))],
        Some(Ok(text))
            if !view.source
                && (view.name.to_lowercase().ends_with(".md") || view.name.to_lowercase().ends_with(".markdown")) =>
        {
            md::render(text, width, 1)
        }
        Some(Ok(text))
            if !view.source
                && (view.name.to_lowercase().ends_with(".csv") || view.name.to_lowercase().ends_with(".tsv")) =>
        {
            match crate::chat::table_preview::format(&view.name, text) {
                Ok(table) => {
                    let mut lines = md::render(table["markdown"].as_str().unwrap_or_default(), width, 1);
                    if table["truncated"] == true {
                        lines.push(Line::raw("Preview truncated; v shows the full source."));
                    }
                    lines
                }
                Err(error) => vec![Line::raw(format!("{error}; v shows the full source."))],
            }
        }
        Some(Ok(text)) if !view.query.is_empty() => {
            let query = view.query.to_lowercase();
            text.lines()
                .enumerate()
                .map(|(index, line)| {
                    let style = if line.to_lowercase().contains(&query) {
                        t.text.add_modifier(ratatui::style::Modifier::REVERSED)
                    } else {
                        t.text
                    };
                    Line::styled(format!("{}: {line}", index + 1), style)
                })
                .collect()
        }
        Some(Ok(text)) if !view.wrap => text.lines().map(|line| Line::raw(line.to_owned())).collect(),
        Some(Ok(text)) => text
            .lines()
            .flat_map(|line| md::wrap(&[Span::raw(line.to_owned())], width, &[Span::raw(" ")], &[Span::raw(" ")]))
            .collect(),
    };
    let height = usize::from(body.height);
    let max = lines.len().saturating_sub(height);
    view.max_scroll.set(max);
    view.page.set(height);
    for (i, line) in lines.iter().skip(view.scroll.min(max)).take(height).enumerate() {
        put_line(buf, body.x, body.y + u(i), body.width, line);
    }
}
