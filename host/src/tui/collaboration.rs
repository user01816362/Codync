//! Device-only presentation of collaboration; never changes bot admission or execution.
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, Ordering};

const FILE: &str = "tui-collaboration-activity";
static MODE: LazyLock<AtomicU8> = LazyLock::new(|| {
    let mode = super::choice::read(FILE).and_then(|value| value.trim().parse::<u8>().ok()).filter(|mode| *mode < 3);
    AtomicU8::new(mode.unwrap_or(1))
});

pub(in crate::tui) fn mode() -> u8 {
    MODE.load(Ordering::Relaxed)
}

pub(in crate::tui) fn cycle() -> &'static str {
    let next = (mode() + 1) % 3;
    if let Err(error) = super::choice::write(FILE, &next.to_string()) {
        tracing::warn!(%error, "couldn't save collaboration display preference");
    } else {
        MODE.store(next, Ordering::Relaxed);
    }
    match mode() {
        0 => "Collaboration activity: Hidden",
        2 => "Collaboration activity: Expanded",
        _ => "Collaboration activity: Compact",
    }
}
