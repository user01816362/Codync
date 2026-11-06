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

fn supported(name: &str, size: u64) -> bool {
    let Some((_, ext)) = name.rsplit_once('.') else { return false };
    size <= 1024 * 1024
        && "md markdown txt log json jsonl yaml yml toml xml csv tsv html css js jsx ts tsx py rs go swift kt kts java c h cpp hpp sh bash zsh sql diff patch"
            .split_whitespace().any(|known| ext.eq_ignore_ascii_case(known))
}

fn first_upload(data: &Value) -> Option<(&str, &str, u64)> {
    data["attachments"].as_array()?.iter().find_map(|file| {
        let (upload, name, size) = (file["id"].as_str()?, file["name"].as_str()?, file["size"].as_u64()?);
        supported(name, size).then_some((upload, name, size))
    })
}

fn decode_path(value: &str) -> Option<String> {
    let input = value.as_bytes();
    let mut output = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' {
            let digits = std::str::from_utf8(input.get(i + 1..i + 3)?).ok()?;
            output.push(u8::from_str_radix(digits, 16).ok()?);
            i += 3;
        } else {
            output.push(input[i]);
            i += 1;
        }
    }
    if output.contains(&0) {
        return None;
    }
    String::from_utf8(output).ok()
}

fn local_target(target: &str) -> Option<String> {
    if target.is_empty() || target.starts_with('#') || target.starts_with("//") || target.contains(['?', '#']) {
        return None;
    }
    if target.starts_with("file:") {
        return url::Url::parse(target).ok()?.to_file_path().ok()?.to_str().map(str::to_owned);
    }
    if target.contains(':') || target.chars().any(char::is_whitespace) {
        return None;
    }
    decode_path(target)
}

impl App {
    /// The first explicit Markdown link to a local path, opened only after the user selects the message and presses o.
    pub(in crate::tui) fn workspace_link(text: &str) -> Option<String> {
        let mut fence: Option<(u8, usize)> = None;
        for line in text.lines() {
            let trimmed = line.trim_start_matches(' ');
            let indent = line.len() - trimmed.len();
            if indent <= 3
                && let Some(marker @ (b'`' | b'~')) = trimmed.bytes().next()
            {
                let count = trimmed.bytes().take_while(|b| *b == marker).count();
                if count >= 3 {
                    match fence {
                        None => fence = Some((marker, count)),
                        Some((open, length)) if marker == open && count >= length => fence = None,
                        _ => {}
                    }
                    continue;
                }
            }
            if fence.is_some() || indent >= 4 {
                continue;
            }
            let bytes = line.as_bytes();
            let (mut index, mut ticks) = (0, 0);
            while index < bytes.len() {
                match bytes[index] {
                    b'\\' => index += 2,
                    b'`' => {
                        let count = bytes[index..].iter().take_while(|b| **b == b'`').count();
                        if ticks == 0 {
                            ticks = count;
                        } else if ticks == count {
                            ticks = 0;
                        }
                        index += count;
                    }
                    b'[' if ticks == 0 => {
                        let Some(end) = line[index + 1..].find("](") else { break };
                        let start = index + 1 + end + 2;
                        if let Some(close) = line[start..].find(')') {
                            if let Some(path) = local_target(&line[start..start + close]) {
                                return Some(path);
                            }
                            index = start + close + 1;
                        } else {
                            break;
                        }
                    }
                    _ => index += 1,
                }
            }
        }
        None
    }

    pub(in crate::tui) fn open_workspace_preview(&mut self, bot: &str, path: String) {
        let id = uuid::Uuid::new_v4().to_string();
        self.client.preview_workspace(bot.to_owned(), path.clone(), id.clone(), self.tx.clone());
        self.overlays.push(Overlay::FilePreview(FilePreviewSheet {
            id,
            name: path,
            content: None,
            scroll: 0,
            max_scroll: Cell::default(),
            page: Cell::default(),
        }));
    }

    pub(in crate::tui) fn open_file_preview(&mut self, entry: &str, data: &Value) {
        let Some(file) = data["files"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|raw| serde_json::from_value::<crate::chat::files::SharedFile>(raw.clone()).ok())
            .find(|file| supported(&file.name, file.size))
        else {
            self.flash("No text file up to 1 MiB to preview; use f to download");
            return;
        };
        let id = uuid::Uuid::new_v4().to_string();
        self.client.preview_file(entry.to_owned(), file.clone(), id.clone(), self.tx.clone());
        self.overlays.push(Overlay::FilePreview(FilePreviewSheet {
            id,
            name: file.name,
            content: None,
            scroll: 0,
            max_scroll: Cell::default(),
            page: Cell::default(),
        }));
    }

    pub(in crate::tui) fn open_upload_preview(&mut self, bot: &str, data: &Value) {
        let Some((upload, name, size)) = first_upload(data) else {
            self.flash("No text attachment up to 1 MiB to preview; use f to download");
            return;
        };
        let id = uuid::Uuid::new_v4().to_string();
        self.client.preview_upload(bot.to_owned(), upload.to_owned(), size, id.clone(), self.tx.clone());
        self.overlays.push(Overlay::FilePreview(FilePreviewSheet {
            id,
            name: name.to_owned(),
            content: None,
            scroll: 0,
            max_scroll: Cell::default(),
            page: Cell::default(),
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
            id: "id".into(),
            name: "README.MD".into(),
            size: 10,
            sha256: "a".repeat(64),
        };
        assert!(supported(&file.name, file.size));
        file.name = "image.png".into();
        assert!(!supported(&file.name, file.size));
        file.name = "md".into();
        assert!(!supported(&file.name, file.size));
        file.name = "big.txt".into();
        file.size = 1024 * 1024 + 1;
        assert!(!supported(&file.name, file.size));
    }

    #[test]
    fn preview_skips_non_text_and_invalid_attachments() {
        let message = serde_json::json!({"attachments": [
            {"id": "image", "name": "photo.png", "size": 3},
            {"name": "invalid.md", "size": 1},
            {"id": "text", "name": "notes.md", "size": 5}
        ]});
        assert_eq!(first_upload(&message), Some(("text", "notes.md", 5)));
    }

    #[test]
    fn workspace_preview_finds_only_explicit_local_links() {
        assert_eq!(App::workspace_link("[source](src/main.rs)"), Some("src/main.rs".into()));
        assert_eq!(App::workspace_link("[web](https://example.com/x)"), None);
        assert_eq!(App::workspace_link("[section](#heading)"), None);
        assert_eq!(App::workspace_link("plain src/main.rs"), None);
        assert_eq!(App::workspace_link("not a link ](secret.txt)"), None);
        assert_eq!(App::workspace_link("`[code](secret.txt)`"), None);
        assert_eq!(App::workspace_link("```md\n[code](secret.txt)\n```"), None);
        assert_eq!(App::workspace_link("[notes](my%20file.md)"), Some("my file.md".into()));
        assert_eq!(App::workspace_link("[bad](%00.md)"), None);
    }
}
