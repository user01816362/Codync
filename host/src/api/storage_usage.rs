//! Bounded accounting for app-managed file roots. Compaction never removes a saved message or file.
use crate::{
    chat::{artifacts, files, uploads},
    hub::Hub,
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

pub async fn overview(hub: &Arc<Hub>, compact: bool, body: &Value) -> Result<Value> {
    if compact {
        ensure!(body["confirm"] == true, "confirm database compaction first");
    }
    let hub = hub.clone();
    tokio::task::spawn_blocking(move || {
        if compact {
            hub.store.compact_storage()?;
        }
        let mut result = hub.store.storage_usage()?;
        let mut shared = Vec::new();
        let mut uploaded = Vec::new();
        let mut rendered = Vec::new();
        for row in hub.store.bots()? {
            shared.push(files::root(&row.config.id));
            uploaded.push(uploads::root(&row.config));
            rendered.push(artifacts::root(&row.config.id));
        }
        result["sharedFiles"] = scan(shared);
        result["uploads"] = scan(uploaded);
        result["artifacts"] = scan(rendered);
        Ok(result)
    })
    .await?
}

fn scan(roots: Vec<PathBuf>) -> Value {
    let mut pending: Vec<(PathBuf, usize)> = roots.into_iter().map(|path| (path, 0)).collect();
    let (mut examined, mut count, mut bytes, mut unavailable) = (0_u64, 0_u64, 0_u64, 0_u64);
    let mut truncated = false;
    while let Some((path, depth)) = pending.pop() {
        if examined >= 10_000 {
            truncated = true;
            break;
        }
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            _ => {
                unavailable += 1;
                continue;
            }
        }
        let Ok(entries) = std::fs::read_dir(path) else {
            unavailable += 1;
            continue;
        };
        for entry in entries {
            examined += 1;
            if examined > 10_000 {
                truncated = true;
                break;
            }
            let Ok(entry) = entry else {
                unavailable += 1;
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                unavailable += 1;
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if depth < 8 {
                    pending.push((entry.path(), depth + 1));
                } else {
                    truncated = true;
                }
            } else if kind.is_file() {
                match entry.metadata() {
                    Ok(meta) => {
                        count += 1;
                        bytes = bytes.saturating_add(meta.len());
                    }
                    Err(_) => unavailable += 1,
                }
            }
        }
    }
    json!({"count":count,"bytes":bytes,"truncated":truncated,"unavailable":unavailable})
}
