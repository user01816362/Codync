//! Bound each adapter frame before allocating an untrusted, arbitrarily long line.
use anyhow::{Result, ensure};
use tokio::io::{AsyncBufRead, AsyncBufReadExt as _, AsyncReadExt as _};

const MAX_FRAME: usize = 32 * 1024 * 1024;

pub(super) async fn next(reader: &mut (impl AsyncBufRead + Unpin)) -> Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    let count = (&mut *reader).take((MAX_FRAME + 1) as u64).read_until(b'\n', &mut bytes).await?;
    ensure!(count <= MAX_FRAME, "agent frame exceeds 32 MiB");
    Ok((count > 0).then_some(bytes))
}
