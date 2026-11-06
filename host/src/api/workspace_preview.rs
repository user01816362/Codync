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
    let bot_id = bot.config.id;
    let bots = hub.store.bots()?;
    let roots: Vec<(String, PathBuf)> = bots
        .into_iter()
        .filter(|row| !row.deleted && !row.config.is_group() && !row.config.cwd.is_empty())
        .map(|row| (row.config.id, PathBuf::from(row.config.cwd)))
        .collect();
    let path = str_arg(body, "path")?.to_owned();
    ensure!(path.len() <= 4096 && !path.is_empty(), "invalid path");
    tokio::task::spawn_blocking(move || {
        let (owner, root) = workspace(&roots, &bot_id, Path::new(&path))?;
        let (name, text) = read_text(root, &path)?;
        Ok(json!({"botId": owner, "name": name, "text": text}))
    })
    .await?
}

/// Absolute links can be forwarded between bots. Resolve only registered workspaces,
/// never a filesystem-wide search or a guess by basename. Relative links retain their author.
fn workspace<'a>(roots: &'a [(String, PathBuf)], author: &str, path: &Path) -> Result<&'a (String, PathBuf)> {
    if path.is_absolute() {
        roots
            .iter()
            .filter(|(_, root)| path.starts_with(root))
            .max_by_key(|(_, root)| root.components().count())
            .ok_or_else(|| {
                anyhow!("This file is outside the available bot workspaces. Ask the bot to share the file instead.")
            })
    } else {
        roots.iter().find(|(id, _)| id == author).ok_or_else(|| anyhow!("This bot's workspace is unavailable."))
    }
}

fn read_text(root: &Path, requested: &str) -> Result<(String, String)> {
    let configured = root;
    let root = configured.canonicalize().context("workspace is unavailable")?;
    let requested = Path::new(requested);
    let relative = if requested.is_absolute() {
        requested
            .strip_prefix(configured)
            .or_else(|_| requested.strip_prefix(&root))
            .context("file is outside this workspace")?
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

#[cfg(windows)]
fn open_workspace_file(root: &Path, relative: &Path) -> Result<std::fs::File> {
    use std::os::windows::ffi::OsStringExt as _;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW;

    // Reject alternate data streams and Windows device names before opening anything.
    for part in relative.components() {
        let Component::Normal(value) = part else { continue };
        let value = value.to_string_lossy();
        let base = value.split('.').next().unwrap_or_default().trim_end_matches([' ', '.']).to_ascii_uppercase();
        ensure!(
            !value.contains(':')
                && !matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
                && !(base.len() == 4
                    && (base.starts_with("COM") || base.starts_with("LPT"))
                    && matches!(base.as_bytes()[3], b'1'..=b'9'))
                && !matches!(base.as_str(), "COM¹" | "COM²" | "COM³" | "LPT¹" | "LPT²" | "LPT³"),
            "invalid workspace path"
        );
    }
    let file = std::fs::File::open(root.join(relative)).context("opening workspace file")?;
    ensure!(file.metadata()?.is_file(), "only regular files can be previewed");
    let mut buffer = vec![0u16; 32768];
    // SAFETY: the file owns a live Windows handle and the buffer has the stated capacity.
    let len = unsafe { GetFinalPathNameByHandleW(file.as_raw_handle().cast(), buffer.as_mut_ptr(), 32768, 0) };
    ensure!(len > 0 && (len as usize) < buffer.len(), "cannot resolve workspace file");
    buffer.truncate(len as usize);
    let resolved = PathBuf::from(std::ffi::OsString::from_wide(&buffer));
    ensure!(resolved.starts_with(root), "file is outside this workspace");
    Ok(file)
}

#[cfg(not(any(unix, windows)))]
fn open_workspace_file(_root: &Path, _relative: &Path) -> Result<std::fs::File> {
    anyhow::bail!("workspace previews are not available on this host")
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    #[test]
    fn forwarded_absolute_links_resolve_only_registered_workspaces() {
        let base = std::env::temp_dir().join(format!("codync-forwarded-preview-{}", uuid::Uuid::new_v4()));
        let author = base.join("author");
        let owner = base.join("owner");
        std::fs::create_dir_all(&author).unwrap();
        std::fs::create_dir_all(&owner).unwrap();
        std::fs::write(owner.join("report.md"), "# Shared report").unwrap();
        let roots = vec![("a".to_owned(), author), ("b".to_owned(), owner.clone())];
        let linked = owner.join("report.md");
        let (id, root) = workspace(&roots, "a", &linked).unwrap();
        assert_eq!(id, "b");
        assert_eq!(read_text(root, linked.to_str().unwrap()).unwrap().1, "# Shared report");
        assert_eq!(workspace(&roots, "a", Path::new("report.md")).unwrap().0, "a");
        assert!(workspace(&roots, "a", &base.join("outside.md")).is_err());
        std::fs::write(base.join("outside.md"), "outside").unwrap();
        std::os::unix::fs::symlink(base.join("outside.md"), owner.join("escape.md")).unwrap();
        assert!(read_text(root, "escape.md").is_err());
        std::fs::remove_dir_all(base).unwrap();
    }

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

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn workspace_preview_rejects_device_names_and_parent_paths() {
        let root = std::env::temp_dir().join(format!("codync-preview-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("note.md"), "hello").unwrap();
        assert_eq!(read_text(&root, "note.md").unwrap().1, "hello");
        assert!(read_text(&root, "../outside").is_err());
        assert!(read_text(&root, "CON").is_err());
        assert!(read_text(&root, "note.md:stream").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
