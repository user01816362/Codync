//! Read-only previews of small bot-shared text snapshots.

use std::cell::Cell;

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;

use super::super::app::{App, Overlay};
use super::ctrl;

pub struct FilePreviewSheet {
    pub id: String,
    pub name: String,
    pub content: Option<Result<String, String>>,
    pub scroll: usize,
    pub max_scroll: Cell<usize>,
    pub page: Cell<usize>,
}

fn supported(file: &crate::chat::files::SharedFile) -> bool {
    let Some((_, ext)) = file.name.rsplit_once('.') else { return false };
    file.size <= 1024 * 1024
        && "md markdown txt log json jsonl yaml yml toml xml csv tsv html css js jsx ts tsx py rs go swift kt kts java c h cpp hpp sh bash zsh sql diff patch"
            .split_whitespace().any(|known| ext.eq_ignore_ascii_case(known))
}

impl App {
    pub(in crate::tui) fn open_file_preview(&mut self, entry: &str, data: &Value) {
        let Some(raw) = data["files"].as_array().and_then(|files| files.first()) else { return };
        let Ok(file) = serde_json::from_value::<crate::chat::files::SharedFile>(raw.clone()) else {
            self.flash("Invalid file metadata");
            return;
        };
        if !supported(&file) {
            self.flash("Preview supports text files up to 1 MiB; use f to download");
            return;
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.client.preview_file(entry.to_owned(), file.clone(), id.clone(), self.tx.clone());
        self.overlays.push(Overlay::FilePreview(FilePreviewSheet {
            id, name: file.name, content: None, scroll: 0, max_scroll: Cell::default(), page: Cell::default(),
        }));
    }

    pub(in crate::tui) fn file_preview_key(mut view: FilePreviewSheet, key: KeyEvent) -> Option<Overlay> {
        let half = (view.page.get() / 2).max(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return None,
            KeyCode::Down | KeyCode::Char('j') => view.scroll = (view.scroll + 1).min(view.max_scroll.get()),
            KeyCode::Up | KeyCode::Char('k') => view.scroll = view.scroll.saturating_sub(1),
            KeyCode::PageDown => view.scroll = (view.scroll + half).min(view.max_scroll.get()),
            KeyCode::PageUp => view.scroll = view.scroll.saturating_sub(half),
            KeyCode::Char('d') if ctrl(key) => view.scroll = (view.scroll + half).min(view.max_scroll.get()),
            KeyCode::Char('u') if ctrl(key) => view.scroll = view.scroll.saturating_sub(half),
            _ => {}
        }
        Some(Overlay::FilePreview(view))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_small_text_files_are_previewable() {
        let mut file = crate::chat::files::SharedFile {
            id: "id".into(), name: "README.MD".into(), size: 10, sha256: "a".repeat(64),
        };
        assert!(supported(&file));
        file.name = "image.png".into();
        assert!(!supported(&file));
        file.name = "md".into();
        assert!(!supported(&file));
        file.name = "big.txt".into();
        file.size = 1024 * 1024 + 1;
        assert!(!supported(&file));
    }
}
