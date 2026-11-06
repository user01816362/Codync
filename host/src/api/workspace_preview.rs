//! User-requested, bounded text previews inside a bot's configured workspace.
use super::str_arg;
use crate::hub::Hub;
use anyhow::{Context as _, Result, anyhow, ensure};
use serde_json::{Value, json};
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

const MAX_PREVIEW: u64 = 1024 * 1024;

pub async fn read(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let bot = hub.store.bot(str_arg(body, "botId")?)?.ok_or_else(|| anyhow!("unknown bot"))?;
    ensure!(!bot.deleted && !bot.config.is_group(), "unknown bot");
    let root = PathBuf::from(bot.config.cwd);
    let path = str_arg(body, "path")?.to_owned();
    ensure!(path.len() <= 4096 && !path.is_empty(), "invalid path");
    let (name, text) = tokio::task::spawn_blocking(move || read_text(&root, &path)).await??;
    Ok(json!({"name": name, "text": text}))
}

fn read_text(root: &Path, requested: &str) -> Result<(String, String)> {
    let root = root.canonicalize().context("workspace is unavailable")?;
    let requested = Path::new(requested);
    let relative = if requested.is_absolute() {
        requested.strip_prefix(&root).context("file is outside this workspace")?
    } else {
        requested
    };
    ensure!(
        relative.components().all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
        "invalid workspace path"
    );
    let name =
        relative.file_name().and_then(|value| value.to_str()).ok_or_else(|| anyhow!("invalid file name"))?.to_owned();
    let mut file = open_workspace_file(&root, relative)?;
    ensure!(file.metadata()?.is_file(), "only regular files can be previewed");
    ensure!(file.metadata()?.len() <= MAX_PREVIEW, "file exceeds 1 MiB");
    let mut bytes = Vec::new();
    std::io::Read::take(&mut file, MAX_PREVIEW + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= MAX_PREVIEW && !bytes.contains(&0), "file is too large or is not text");
    let text = String::from_utf8(bytes).context("file is not UTF-8 text")?;
    ensure!(text.chars().all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t')), "file contains binary controls");
    Ok((name, text))
}

#[cfg(unix)]
fn open_workspace_file(root: &Path, relative: &Path) -> Result<std::fs::File> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd as _, FromRawFd as _};
    use std::os::unix::ffi::OsStrExt as _;
    let mut directory = std::fs::File::open(root)?;
    let mut parts = relative
        .components()
        .filter_map(|part| match part {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .peekable();
    while let Some(part) = parts.next() {
        let name = CString::new(part.as_bytes()).context("invalid path")?;
        let mut flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
        if parts.peek().is_some() {
            flags |= libc::O_DIRECTORY;
        }
        // SAFETY: the directory descriptor is live and name is a NUL-terminated CString.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error()).context("opening workspace file");
        }
        // SAFETY: openat returned a new owned descriptor.
        directory = unsafe { std::fs::File::from_raw_fd(fd) };
    }
    Ok(directory)
}

#[cfg(not(unix))]
fn open_workspace_file(_root: &Path, _relative: &Path) -> Result<std::fs::File> {
    anyhow::bail!("workspace previews are not available on this host")
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    #[test]
    fn previews_only_bounded_utf8_regular_files_inside_workspace() {
        let root = std::env::temp_dir().join(format!("codync-preview-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/notes.md"), "hello").unwrap();
        std::fs::write(root.join("binary"), [0]).unwrap();
        std::os::unix::fs::symlink("/etc", root.join("escape")).unwrap();
        assert_eq!(read_text(&root, "src/notes.md").unwrap().1, "hello");
        assert!(read_text(&root, "../outside").is_err());
        assert!(read_text(&root, "escape/passwd").is_err());
        assert!(read_text(&root, "binary").is_err());
        assert!(read_text(&root, "src").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
