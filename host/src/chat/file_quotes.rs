//! Verify literal excerpts against bounded saved file snapshots, never client-provided authors or paths.
use crate::{chat::{files, uploads}, store::{Entry, Store}};
use anyhow::{Context as _, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

pub fn annotate(store: &Store, entry: &Entry, meta: &Value, shared: bool, text: &str, reply: &mut Value) -> Result<()> {
    let size = meta["size"].as_u64().context("unknown file size")?;
    ensure!(size <= 1024 * 1024, "file excerpts are limited to text files up to 1 MiB");
    let mut bytes = Vec::new();
    if shared {
        let file: files::SharedFile = serde_json::from_value(meta.clone())?;
        let owner = entry.data["author"].as_str().unwrap_or(&entry.bot_id);
        let root = files::root(owner);
        while (bytes.len() as u64) < size {
            let chunk = files::read(&root, &file, bytes.len() as u64)?;
            ensure!(!chunk.is_empty() && bytes.len() + chunk.len() <= 1024 * 1024, "file is incomplete");
            bytes.extend(chunk);
        }
        ensure!(format!("{:x}", Sha256::digest(&bytes)) == file.sha256, "file integrity check failed");
    } else {
        let cfg = store.bot(&entry.bot_id)?.context("source conversation is unavailable")?.config;
        let (loaded, actual) = uploads::read(&uploads::root(&cfg), meta["id"].as_str().context("unknown upload")?, 0, 1024 * 1024)?;
        ensure!(actual == size, "upload size changed");
        bytes = loaded;
    }
    ensure!(bytes.len() as u64 == size, "file size changed");
    let source = std::str::from_utf8(&bytes).context("file is not UTF-8 text")?;
    let at = source.find(text).context("select a literal passage in Source to quote this file accurately")?;
    reply["fileName"] = meta["name"].clone();
    if source.rfind(text) == Some(at) {
        let start = source[..at].bytes().filter(|byte| *byte == b'\n').count() + 1;
        let end = start + text.trim_end_matches('\n').bytes().filter(|byte| *byte == b'\n').count();
        reply["startLine"] = json!(start);
        reply["endLine"] = json!(end);
    }
    Ok(())
}
