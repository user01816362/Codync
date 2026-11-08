//! Read-only directory listings through the same confined open primitive as text previews.
use super::{str_arg, workspace_preview};
use crate::hub::Hub;
use anyhow::{Context as _, Result, anyhow, ensure};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;

pub async fn list(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let bot = hub.store.bot(str_arg(body, "botId")?)?.ok_or_else(|| anyhow!("unknown bot"))?;
    ensure!(!bot.deleted && !bot.config.is_group(), "unknown bot");
    let roots = workspace_preview::roots(hub)?;
    let bot = bot.config.id;
    let path = body["path"].as_str().unwrap_or(".").to_owned();
    ensure!(!path.is_empty() && path.len() <= 4096, "invalid workspace path");
    let hidden = body["showHidden"].as_bool().unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        let (owner, root) = workspace_preview::workspace(&roots, &bot, Path::new(&path))?;
        let (directory, relative) = workspace_preview::open(root, &path)?;
        ensure!(directory.metadata()?.is_dir(), "not a directory");
        let (mut entries, mut truncated) = entries(&directory)?;
        entries.retain(|entry| valid_name(&entry.name) && (hidden || !entry.name.starts_with('.')));
        entries.sort_by_cached_key(|entry| (entry.kind != "directory", entry.name.to_lowercase()));
        truncated |= entries.len() > 500;
        entries.truncate(500);
        let parent = if relative.components().any(|part| matches!(part, std::path::Component::Normal(_))) {
            Some(relative.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_string_lossy().into_owned())
        } else { None };
        let rows: Vec<Value> = entries.into_iter().map(|entry| json!({
            "name": entry.name, "path": relative.join(&entry.name).to_string_lossy(),
            "kind": entry.kind, "size": entry.size
        })).collect();
        Ok(json!({"botId": owner, "path": relative.to_string_lossy(), "parent": parent, "entries": rows, "truncated": truncated}))
    }).await?
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.chars().any(char::is_control)
        && Path::new(name).components().count() == 1
}

struct Item {
    name: String,
    kind: &'static str,
    size: u64,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn entries(directory: &std::fs::File) -> Result<(Vec<Item>, bool)> {
    use std::ffi::CStr;
    use std::os::fd::{AsRawFd as _, IntoRawFd as _};
    struct Directory(*mut libc::DIR);
    impl Drop for Directory {
        fn drop(&mut self) {
            // SAFETY: this guard owns the directory stream returned by fdopendir.
            unsafe { libc::closedir(self.0) };
        }
    }
    let fd = directory.try_clone()?.into_raw_fd();
    // SAFETY: fd is a new owned descriptor; successful fdopendir takes ownership.
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        let error = std::io::Error::last_os_error();
        // SAFETY: fdopendir did not take ownership on failure.
        unsafe { libc::close(fd) };
        return Err(error).context("listing workspace directory");
    }
    let stream = Directory(stream);
    let mut rows = Vec::new();
    for _ in 0..2000 {
        // SAFETY: errno is thread-local; this blocking task does not yield between reset and readdir.
        let (entry, error) = unsafe {
            #[cfg(target_os = "macos")]
            let errno = libc::__error();
            #[cfg(target_os = "linux")]
            let errno = libc::__errno_location();
            *errno = 0;
            let entry = libc::readdir(stream.0);
            (entry, *errno)
        };
        if entry.is_null() {
            ensure!(error == 0, "listing workspace directory: {}", std::io::Error::from_raw_os_error(error));
            return Ok((rows, false));
        }
        // SAFETY: readdir returns a live NUL-terminated name until the next call on this stream.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        let Ok(text) = name.to_str() else { continue };
        if !valid_name(text) { continue; }
        let mut info = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: the directory remains open, name is terminated, and info is writable.
        let status = unsafe { libc::fstatat(directory.as_raw_fd(), name.as_ptr(), info.as_mut_ptr(), libc::AT_SYMLINK_NOFOLLOW) };
        if status != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::NotFound { continue; }
            return Err(error).context("reading directory entry");
        }
        // SAFETY: successful fstatat initialized the entire stat structure.
        let info = unsafe { info.assume_init() };
        let kind = match info.st_mode & libc::S_IFMT {
            libc::S_IFLNK => "link",
            libc::S_IFDIR => "directory",
            libc::S_IFREG => "file",
            _ => "special",
        };
        rows.push(Item { name: text.to_owned(), kind, size: info.st_size.max(0) as u64 });
    }
    Ok((rows, true))
}

#[cfg(windows)]
fn entries(directory: &std::fs::File) -> Result<(Vec<Item>, bool)> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::ERROR_NO_MORE_FILES;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FULL_DIR_INFO,
        FileFullDirectoryInfo, FileFullDirectoryRestartInfo, GetFileInformationByHandleEx,
    };
    let mut buffer = vec![0u64; 8192];
    let capacity = buffer.len() * 8;
    let header = std::mem::offset_of!(FILE_FULL_DIR_INFO, FileName);
    let mut rows = Vec::new();
    let mut restart = true;
    let mut scanned = 0;
    loop {
        let class = if restart { FileFullDirectoryRestartInfo } else { FileFullDirectoryInfo };
        restart = false;
        // SAFETY: the directory handle is live, and the aligned buffer has the supplied capacity.
        let ok = unsafe { GetFileInformationByHandleEx(directory.as_raw_handle().cast(), class, buffer.as_mut_ptr().cast(), capacity as u32) };
        if ok == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) { return Ok((rows, false)); }
            return Err(error).context("listing workspace directory");
        }
        let mut offset = 0usize;
        loop {
            ensure!(offset + std::mem::size_of::<FILE_FULL_DIR_INFO>() <= capacity, "invalid directory record");
            // SAFETY: the kernel produced a bounded directory record; unaligned reads avoid layout assumptions.
            let info = unsafe { std::ptr::read_unaligned(buffer.as_ptr().cast::<u8>().add(offset).cast::<FILE_FULL_DIR_INFO>()) };
            let bytes = info.FileNameLength as usize;
            let end = offset.checked_add(header).and_then(|start| start.checked_add(bytes)).context("invalid file name")?;
            ensure!(bytes % 2 == 0 && (offset + header) % 2 == 0 && end <= capacity, "invalid directory name");
            // SAFETY: the UTF-16 name range is even-sized and within the live aligned buffer.
            let units = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>().add(offset + header).cast::<u16>(), bytes / 2) };
            scanned += 1;
            if scanned > 2000 { return Ok((rows, true)); }
            if let Ok(name) = String::from_utf16(units) {
                let kind = if info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 { "link" }
                    else if info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 { "directory" } else { "file" };
                rows.push(Item { name, kind, size: info.EndOfFile.max(0) as u64 });
                if rows.len() >= 2000 { return Ok((rows, true)); }
            }
            if info.NextEntryOffset == 0 { break; }
            ensure!(info.NextEntryOffset as usize >= header, "invalid directory offset");
            offset = offset.checked_add(info.NextEntryOffset as usize).context("invalid directory offset")?;
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn entries(_directory: &std::fs::File) -> Result<(Vec<Item>, bool)> {
    anyhow::bail!("workspace browsing is unavailable on this host")
}
