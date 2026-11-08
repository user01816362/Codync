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
        " ↑↓ scroll · v read/source · w wrap · c copy · r retry · esc close",
        t.dim.patch(t.panel),
    );
    let body = Rect::new(inner.x, inner.y, inner.width, inner.height - 2);
    let width = usize::from(body.width);
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
