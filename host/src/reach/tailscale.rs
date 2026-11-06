//! Tailscale on this computer: whether its app and CLI are here, and what address the
//! tailnet gives it. The host already advertises a Tailscale address in every pairing
//! link and `hello`; this is what the settings screens show beside it.

use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// The download page for the current platform.
pub const DOWNLOAD: &str = "https://tailscale.com/download";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// A GUI app or CLI is installed on this computer.
    pub installed: bool,
    /// The daemon is running and signed in.
    pub running: bool,
    /// The host can open the native app for sign-in (currently the macOS app).
    pub can_open: bool,
    /// The `MagicDNS` name, without the trailing dot (`mac.example.ts.net`).
    pub dns_name: Option<String>,
    /// The first IPv4 address on the tailnet (`100.x.y.z`).
    pub ipv4: Option<String>,
    /// Where to install it.
    pub download: String,
}

/// The `tailscale` command: the CLI, or the one inside the Mac app.
pub fn binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CODYNC_TAILSCALE").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(path);
    }
    if let Some(path) = on_path(if cfg!(windows) { "where" } else { "which" }, "tailscale") {
        return Some(path);
    }
    if cfg!(windows) {
        let mut paths = vec![
            PathBuf::from(r"C:\Program Files\Tailscale\tailscale.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Tailscale\tailscale.exe"),
        ];
        for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = std::env::var_os(variable) {
                paths.push(PathBuf::from(root).join("Tailscale").join("tailscale.exe"));
            }
        }
        if let Some(path) = paths.into_iter().find(|path| path.is_file()) {
            return Some(path);
        }
    }
    let paths: &[&str] = if cfg!(target_os = "macos") {
        &["/opt/homebrew/bin/tailscale", "/usr/local/bin/tailscale"]
    } else {
        &["/usr/bin/tailscale", "/usr/local/bin/tailscale"]
    };
    paths.iter().map(PathBuf::from).find(|path| path.is_file()).or_else(mac_app_binary)
}

pub fn status() -> Status {
    let binary = binary();
    let can_open = mac_app_installed();
    let installed = binary.is_some() || can_open;
    let Some(binary) = binary else {
        return Status { installed, can_open, download: DOWNLOAD.into(), ..Status::default() };
    };
    let output = Command::new(binary).env("TAILSCALE_BE_CLI", "1").args(["status", "--json"]).output();
    let Ok(output) = output else {
        return Status { installed: true, can_open, download: DOWNLOAD.into(), ..Status::default() };
    };
    if !output.status.success() {
        // Installed but the daemon is stopped or signed out: that is not "not installed".
        return Status { installed: true, can_open, download: DOWNLOAD.into(), ..Status::default() };
    }
    let mut status =
        parse(&output.stdout).unwrap_or(Status { installed: true, download: DOWNLOAD.into(), ..Status::default() });
    status.can_open = can_open;
    status
}

/// Opens the Mac app so a person can sign in. On other systems Tailscale's own
/// `tailscale up` prints the sign-in link to its terminal.
pub fn open_login() -> Result<(), String> {
    if cfg!(target_os = "macos") && mac_app_installed() {
        let status = Command::new("open")
            .args(["-a", "Tailscale"])
            .status()
            .map_err(|error| format!("Couldn't open Tailscale: {error}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("Couldn't open Tailscale (exit status {status})."));
    }
    Err("Open Tailscale on this computer and sign in, then come back.".into())
}

fn mac_app_installed() -> bool {
    mac_app_binary().is_some()
}

fn mac_app_binary() -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let mut apps = vec![PathBuf::from("/Applications/Tailscale.app")];
    if let Some(home) = dirs::home_dir() {
        apps.push(home.join("Applications/Tailscale.app"));
    }
    apps.into_iter()
        .flat_map(|app| [app.join("Contents/MacOS/Tailscale"), app.join("Contents/MacOS/tailscale")])
        .find(|path| path.is_file())
}

/// Parses `tailscale status --json`. `Self.DNSName` and `Self.TailscaleIPs` are enough;
/// `BackendState` says whether the daemon is running.
fn parse(raw: &[u8]) -> Option<Status> {
    let value: Value = serde_json::from_slice(raw).ok()?;
    let state = value["BackendState"].as_str().unwrap_or_default();
    let me = &value["Self"];
    let dns = me["DNSName"].as_str().map(|name| name.trim_end_matches('.').to_owned()).filter(|name| !name.is_empty());
    let ipv4 = me["TailscaleIPs"]
        .as_array()
        .and_then(|ips| ips.iter().filter_map(Value::as_str).find(|ip| ip.parse::<std::net::Ipv4Addr>().is_ok()))
        .map(str::to_owned);
    Some(Status {
        installed: true,
        running: state == "Running",
        dns_name: dns,
        ipv4,
        can_open: false,
        download: DOWNLOAD.into(),
    })
}

fn on_path(which: &str, name: &str) -> Option<PathBuf> {
    let output = Command::new(which).arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).lines().next()?.trim().to_owned();
    (!path.is_empty()).then(|| PathBuf::from(path)).filter(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_name_and_address() {
        let json = br#"{
            "BackendState": "Running",
            "Self": {
                "DNSName": "mac.example.ts.net.",
                "TailscaleIPs": ["fd7a:115c:a1e0::1", "100.101.102.103"]
            }
        }"#;
        let status = parse(json).unwrap();
        assert!(status.running);
        assert_eq!(status.dns_name.as_deref(), Some("mac.example.ts.net"));
        assert_eq!(status.ipv4.as_deref(), Some("100.101.102.103"));
    }

    #[test]
    fn a_stopped_daemon_is_still_installed() {
        let status = parse(br#"{"BackendState":"Stopped","Self":{}}"#).unwrap();
        assert!(!status.running);
        assert_eq!(status.dns_name, None);
    }
}
