//! Read-only previews of small bot-shared text snapshots.

use std::cell::Cell;

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::super::app::{App, Overlay};
use super::ctrl;

pub struct FilePreviewSheet {
    pub id: String,
    pub name: String,
    pub content: Option<Result<String, String>>,
    pub directory: Option<Value>,
    pub scroll: usize,
    pub max_scroll: Cell<usize>,
    pub page: Cell<usize>,
    preference_key: String,
    pub query: String,
    pub searching: bool,
    pub source: bool,
    pub wrap: bool,
    request: PreviewRequest,
}

#[derive(Clone, Debug)]
enum PreviewRequest {
    Directory { bot: String, path: String, hidden: bool },
    Workspace { bot: String, path: String },
    Shared { entry: String, file: crate::chat::files::SharedFile },
    Upload { bot: String, upload: String, size: u64 },
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

    pub(in crate::tui) fn open_workspace_browser(&mut self, bot: &str) {
        if self.bots.get(bot).is_none_or(|bot| bot.group) {
            self.flash("A group has no workspace; open one of its bots instead");
            return;
        }
        self.push_file_preview(
            "Workspace files".into(),
            PreviewRequest::Directory { bot: bot.to_owned(), path: ".".into(), hidden: false },
        );
    }

    pub(in crate::tui) fn open_workspace_preview(&mut self, bot: &str, path: String) {
        self.push_file_preview(path.clone(), PreviewRequest::Workspace { bot: bot.to_owned(), path });
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
        self.push_file_preview(file.name.clone(), PreviewRequest::Shared { entry: entry.to_owned(), file });
    }

    pub(in crate::tui) fn open_upload_preview(&mut self, bot: &str, data: &Value) {
        let Some((upload, name, size)) = first_upload(data) else {
            self.flash("No text attachment up to 1 MiB to preview; use f to download");
            return;
        };
        self.push_file_preview(
            name.to_owned(),
            PreviewRequest::Upload { bot: bot.to_owned(), upload: upload.to_owned(), size },
        );
    }

    fn request_preview(&self, request: &PreviewRequest, id: &str) {
        match request {
            PreviewRequest::Directory { bot, path, hidden } => {
                self.client.preview_directory(bot.clone(), path.clone(), *hidden, id.to_owned(), self.tx.clone());
            }
            PreviewRequest::Workspace { bot, path } => {
                self.client.preview_workspace(bot.clone(), path.clone(), id.to_owned(), self.tx.clone());
            }
            PreviewRequest::Shared { entry, file } => {
                self.client.preview_file(entry.clone(), file.clone(), id.to_owned(), self.tx.clone());
            }
            PreviewRequest::Upload { bot, upload, size } => {
                self.client.preview_upload(bot.clone(), upload.clone(), *size, id.to_owned(), self.tx.clone());
            }
        }
    }

    fn push_file_preview(&mut self, name: String, request: PreviewRequest) {
        let id = uuid::Uuid::new_v4().to_string();
        self.request_preview(&request, &id);
        let key = crate::chat::files::digest_hex(&Sha256::digest(format!("{}:{request:?}", self.url).as_bytes()));
        let preference = preview_preferences().get(&key).copied().unwrap_or([false, true]);
        self.overlays.push(Overlay::FilePreview(FilePreviewSheet {
            id,
            name,
            content: None,
            directory: None,
            scroll: 0,
            max_scroll: Cell::default(),
            page: Cell::default(),
            query: String::new(),
            searching: false,
            preference_key: key,
            source: preference[0],
            wrap: preference[1],
            request,
        }));
    }

    pub(in crate::tui) fn file_preview_key(&mut self, mut view: FilePreviewSheet, key: KeyEvent) -> Option<Overlay> {
        if let PreviewRequest::Directory { bot, path, hidden } = view.request.clone() {
            if matches!(key.code, KeyCode::Char('H' | 'g')) {
                let root = key.code == KeyCode::Char('g');
                let path = if root { ".".to_owned() } else { path };
                view.name.clone_from(&path);
                view.request = PreviewRequest::Directory { bot, path, hidden: if root { hidden } else { !hidden } };
                view.id = uuid::Uuid::new_v4().to_string();
                view.directory = None;
                view.content = None;
                view.scroll = 0;
                self.request_preview(&view.request, &view.id);
                return Some(Overlay::FilePreview(view));
            }
            if matches!(key.code, KeyCode::Enter | KeyCode::Char('o'))
                && let Some(item) = view
                    .directory
                    .as_ref()
                    .and_then(|v| v["entries"].as_array())
                    .and_then(|rows| rows.get(view.scroll))
                    .cloned()
            {
                let path = item["path"].as_str().unwrap_or_default().to_owned();
                if item["kind"] == "directory" {
                    view.request = PreviewRequest::Directory { bot, path: path.clone(), hidden };
                    view.name = path;
                    view.id = uuid::Uuid::new_v4().to_string();
                    view.directory = None;
                    view.content = None;
                    view.scroll = 0;
                    self.request_preview(&view.request, &view.id);
                } else if item["kind"] == "file"
                    && supported(item["name"].as_str().unwrap_or_default(), item["size"].as_u64().unwrap_or(u64::MAX))
                {
                    self.overlays.push(Overlay::FilePreview(view));
                    self.open_workspace_preview(&bot, path);
                    return None;
                } else {
                    self.flash("Ask the bot to share this file for preview");
                }
                return Some(Overlay::FilePreview(view));
            }
        }
        if view.searching {
            match key.code {
                KeyCode::Esc => {
                    view.searching = false;
                    view.query.clear();
                }
                KeyCode::Enter => view.searching = false,
                KeyCode::Backspace => {
                    view.query.pop();
                }
                KeyCode::Char(c) if !ctrl(key) && view.query.chars().count() < 200 => view.query.push(c),
                _ => {}
            }
            if let Some(Ok(text)) = &view.content {
                let query = view.query.to_lowercase();
                view.scroll = if query.is_empty() {
                    0
                } else {
                    text.lines().position(|line| line.to_lowercase().contains(&query)).unwrap_or(0)
                };
            }
            return Some(Overlay::FilePreview(view));
        }
        let half = (view.page.get() / 2).max(1);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return None,
            KeyCode::Char('/') if view.directory.is_none() => {
                view.searching = true;
                view.query.clear();
                view.source = true;
                view.wrap = false;
            }
            KeyCode::Char('n') if !view.query.is_empty() => {
                if let Some(Ok(text)) = &view.content {
                    let query = view.query.to_lowercase();
                    let matches: Vec<usize> = text
                        .lines()
                        .enumerate()
                        .filter_map(|(index, line)| line.to_lowercase().contains(&query).then_some(index))
                        .collect();
                    if let Some(next) = matches.iter().find(|index| **index > view.scroll).or_else(|| matches.first()) {
                        view.scroll = *next;
                    }
                    self.flash(&format!("{} matching lines", matches.len()));
                }
            }
            KeyCode::Char('v') => {
                view.query.clear();
                view.source = !view.source;
                view.scroll = 0;
                if let Err(error) = save_preview_preferences(&view) {
                    self.flash(&format!("View changed; preference could not be saved: {error}"));
                }
            }
            KeyCode::Char('w') => {
                view.query.clear();
                view.wrap = !view.wrap;
                view.scroll = 0;
                if let Err(error) = save_preview_preferences(&view) {
                    self.flash(&format!("View changed; preference could not be saved: {error}"));
                }
            }
            KeyCode::Char('c') => {
                if let Some(directory) = &view.directory {
                    let path = directory["entries"]
                        .as_array()
                        .and_then(|rows| rows.get(view.scroll))
                        .and_then(|entry| entry["path"].as_str())
                        .unwrap_or(".");
                    super::super::app::copy(path);
                } else if let Some(Ok(text)) = &view.content {
                    super::super::app::copy(text);
                }
            }
            KeyCode::Char('r') if view.content.is_some() => {
                view.id = uuid::Uuid::new_v4().to_string();
                view.content = None;
                view.directory = None;
                view.scroll = 0;
                self.request_preview(&view.request, &view.id);
            }
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

const PREVIEW_PREFERENCES: &str = "tui-text-preview-preferences";

fn preview_preferences() -> std::collections::BTreeMap<String, [bool; 2]> {
    super::super::choice::read(PREVIEW_PREFERENCES)
        .filter(|value| value.len() <= 65536)
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default()
}

fn save_preview_preferences(view: &FilePreviewSheet) -> std::io::Result<()> {
    let mut saved = preview_preferences();
    if !saved.contains_key(&view.preference_key) && saved.len() >= 100 {
        saved.pop_first();
    }
    saved.insert(view.preference_key.clone(), [view.source, view.wrap]);
    let value = serde_json::to_string(&saved).map_err(std::io::Error::other)?;
    super::super::choice::write(PREVIEW_PREFERENCES, &value)
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
            plays_in_chat: false,
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
