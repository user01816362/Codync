//! A named Cloudflare Tunnel for this computer (`cloudflared`): sign in to the
//! Cloudflare account in a browser, create a tunnel and its DNS record, and run it
//! against a loopback listener that serves only the phone channel.
//!
//! This is a locally-managed tunnel, as Cloudflare documents it: `tunnel login`
//! writes the account certificate, `tunnel create` writes the tunnel credentials,
//! `tunnel route dns` points the hostname at it, and `tunnel run --config` serves it.
//! The quick `trycloudflare.com` tunnel is deliberately not used: its address changes
//! on every run and it isn't meant for this.

use super::PUBLIC_PORT;
use crate::LockExt;
use crate::market::vault;
use crate::store::Store;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const DOWNLOAD: &str = "https://developers.cloudflare.com/tunnel/downloads/";

/// Non-secret tunnel metadata lives in the host store; credentials are encrypted by the OS vault.
const HOSTNAME: &str = "reach_cloudflare_hostname";
const TUNNEL: &str = "reach_cloudflare_tunnel";
const UUID: &str = "reach_cloudflare_uuid";
const CREDS: &str = "reach_cloudflare_credentials";
/// Older unfinished Reach builds saved this path in the database. Migrate it once, if present.
const LEGACY_CREDS_PATH: &str = "reach_cloudflare_creds";
/// kv: `false` when the person stopped the tunnel here; unset means it starts with the host.
const ENABLED: &str = "reach_cloudflare_enabled";

static OPERATION: Mutex<()> = Mutex::new(());
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

static RUNTIME: LazyLock<Mutex<Runtime>> = LazyLock::new(|| Mutex::new(Runtime::default()));

#[derive(Default)]
struct Runtime {
    child: Option<Child>,
    credential_file: Option<PathBuf>,
    login_pending: bool,
    login_url: Option<String>,
    last_error: Option<String>,
}

// A status snapshot from `cloudflared`, not a decision: each bool says what an external
// tool reported, and the screens read them together.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// `cloudflared` is on this computer.
    pub installed: bool,
    /// A Cloudflare account certificate is on this computer.
    pub logged_in: bool,
    /// A tunnel and hostname are saved.
    pub configured: bool,
    /// It starts with the host and is allowed to run.
    pub enabled: bool,
    pub tunnel: Option<String>,
    pub hostname: Option<String>,
    pub url: Option<String>,
    pub running: bool,
    /// A browser sign-in is waiting; open `login_url` if the browser didn't.
    pub login_pending: bool,
    pub login_url: Option<String>,
    pub last_error: Option<String>,
    pub download: String,
}

/// The `cloudflared` command: an explicit path, the PATH, or a usual install place.
pub fn binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CODYNC_CLOUDFLARED").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(path);
    }
    let paths: &[&str] = if cfg!(target_os = "macos") {
        &["/opt/homebrew/bin/cloudflared", "/usr/local/bin/cloudflared", "/opt/local/bin/cloudflared"]
    } else if cfg!(windows) {
        &[r"C:\Program Files\cloudflared\cloudflared.exe", r"C:\Program Files (x86)\cloudflared\cloudflared.exe"]
    } else {
        &["/usr/local/bin/cloudflared", "/usr/bin/cloudflared", "/snap/bin/cloudflared"]
    };
    paths.iter().map(PathBuf::from).find(|p| p.is_file()).or_else(|| {
        let lookup = if cfg!(windows) { "where" } else { "which" };
        let output = Command::new(lookup).arg("cloudflared").output().ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout).lines().next()?.trim().to_owned();
        (!path.is_empty()).then(|| PathBuf::from(path)).filter(|p| p.is_file())
    })
}

/// The account certificate `tunnel login` writes, or `TUNNEL_ORIGIN_CERT`.
fn origin_cert() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("TUNNEL_ORIGIN_CERT").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(path);
    }
    let managed = origin_cert_path();
    if managed.is_file() {
        return Some(managed);
    }
    let path = dirs::home_dir()?.join(".cloudflared").join("cert.pem");
    path.is_file().then_some(path)
}

fn origin_cert_path() -> PathBuf {
    dir().join("cert.pem")
}

fn dir() -> PathBuf {
    crate::service::data_dir().join("cloudflare")
}

fn log_path() -> PathBuf {
    dir().join("cloudflared.log")
}

fn config_path() -> PathBuf {
    dir().join("config.yml")
}

/// Whether the tunnel should run (configured and not switched off).
pub fn enabled(store: &Store) -> bool {
    store.kv_get(HOSTNAME).is_some() && store.kv_get(ENABLED).as_deref() != Some("false")
}

pub fn status(store: &Store) -> Status {
    refresh_running();
    let runtime = RUNTIME.locked();
    let hostname = store.kv_get(HOSTNAME);
    let configured = hostname.is_some() && store.kv_get(TUNNEL).is_some() && store.kv_get(UUID).is_some();
    Status {
        installed: binary().is_some(),
        logged_in: origin_cert().is_some(),
        configured,
        enabled: configured && store.kv_get(ENABLED).as_deref() != Some("false"),
        tunnel: store.kv_get(TUNNEL),
        url: hostname.as_deref().map(|host| format!("https://{host}")),
        hostname,
        running: runtime.child.is_some(),
        login_pending: runtime.login_pending,
        login_url: runtime.login_url.clone(),
        last_error: runtime.last_error.clone(),
        download: DOWNLOAD.into(),
    }
}

pub fn set_error(error: String) {
    RUNTIME.locked().last_error = Some(error);
}

/// Starts a browser sign-in. Returns once the URL is known (or the process exited).
pub fn login() -> Result<(), String> {
    let binary = binary().ok_or("Install cloudflared on this computer first.")?;
    if origin_cert().is_some() {
        return Ok(());
    }
    {
        let mut runtime = RUNTIME.locked();
        if runtime.login_pending {
            return Ok(());
        }
        runtime.last_error = None;
        runtime.login_pending = true;
        runtime.login_url = None;
    }
    let mut child =
        Command::new(binary).args(["tunnel", "login"]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(
            |error| {
                let mut runtime = RUNTIME.locked();
                runtime.login_pending = false;
                format!("Couldn't start cloudflared: {error}")
            },
        )?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    if let Some(out) = stdout {
        read_for_url(out);
    }
    if let Some(err) = stderr {
        read_for_url(err);
    }
    std::thread::spawn(move || {
        let exit = child.wait();
        if exit.as_ref().is_ok_and(|status| status.success()) {
            if let Some(cert) = origin_cert() {
                secure_file(&cert);
            }
        }
        let mut runtime = RUNTIME.locked();
        runtime.login_pending = false;
        if !exit.is_ok_and(|status| status.success()) && origin_cert().is_none() {
            runtime.last_error = Some("Cloudflare sign-in didn't finish. Try again.".into());
        }
    });
    // cloudflared prints the URL within a moment; the settings screen shows it, and
    // the browser usually opened already.
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if RUNTIME.locked().login_url.is_some() || !RUNTIME.locked().login_pending {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

/// Creates a tunnel (once), points `hostname` at it, saves it and starts it.
pub fn create(store: &Store, hostname: &str) -> Result<Status, String> {
    let _operation = OPERATION.locked();
    let hostname = valid_hostname(hostname)?;
    let binary = binary().ok_or("Install cloudflared on this computer first.")?;
    let cert = origin_cert().ok_or("Sign in to Cloudflare on this computer first.")?;
    private_dir()?;

    let (name, uuid, credentials) =
        match (store.kv_get(TUNNEL), store.kv_get(UUID), saved_credentials(store)?) {
            (Some(name), Some(uuid), Some(credentials)) => (name, uuid, credentials),
            (Some(_), Some(_), None) => return Err(
                "The saved Cloudflare Tunnel credentials are missing. Forget this tunnel here and create a new one."
                    .into(),
            ),
            _ => create_tunnel(&binary, &cert)?,
        };

    // Keep what was created even if the DNS route is refused: a second try reuses it
    // instead of leaving an orphan tunnel in the account.
    store.kv_set(TUNNEL, &name).map_err(|error| error.to_string())?;
    store.kv_set(UUID, &uuid).map_err(|error| error.to_string())?;
    vault::write(store, CREDS, &credentials).map_err(|error| error.to_string())?;
    route_dns(&binary, &cert, &name, &hostname)?;
    store.kv_set(HOSTNAME, &hostname).map_err(|error| error.to_string())?;
    store.kv_set(ENABLED, "true").map_err(|error| error.to_string())?;
    super::set_public_url(Some(&format!("https://{hostname}")));
    start_inner(store)?;
    Ok(status(store))
}

pub fn set_enabled(store: &Store, on: bool) -> Result<Status, String> {
    let _operation = OPERATION.locked();
    if on {
        if store.kv_get(HOSTNAME).is_none() {
            return Err("Set up a Cloudflare Tunnel hostname first.".into());
        }
        store.kv_set(ENABLED, "true").map_err(|error| error.to_string())?;
        start_inner(store)?;
        super::set_public_url(store.kv_get(HOSTNAME).as_deref().map(|host| format!("https://{host}")).as_deref());
    } else {
        store.kv_set(ENABLED, "false").map_err(|error| error.to_string())?;
        stop_inner();
        super::set_public_url(None);
    }
    Ok(status(store))
}

/// Stops the tunnel and forgets its hostname here. The Cloudflare tunnel and DNS
/// record stay in the account, so another computer can use them and no traffic is
/// deleted behind the person's back.
pub fn forget(store: &Store) -> Result<Status, String> {
    let _operation = OPERATION.locked();
    stop_inner();
    let legacy_path = store.kv_get(LEGACY_CREDS_PATH);
    for key in [HOSTNAME, TUNNEL, UUID, CREDS, LEGACY_CREDS_PATH, ENABLED] {
        store.kv_delete(key).map_err(|error| error.to_string())?;
    }
    super::set_public_url(None);
    let _ = fs::remove_file(config_path());
    if let Some(path) = legacy_path {
        let _ = fs::remove_file(path);
    }
    Ok(status(store))
}

/// Runs the saved tunnel against the loopback channel listener.
pub fn start(store: &Store) -> Result<(), String> {
    let _operation = OPERATION.locked();
    if SHUTTING_DOWN.load(Ordering::Relaxed) || !enabled(store) {
        return Ok(());
    }
    start_inner(store)
}

fn start_inner(store: &Store) -> Result<(), String> {
    let binary = binary().ok_or("Install cloudflared on this computer first.")?;
    let hostname = store.kv_get(HOSTNAME).ok_or("Set up a Cloudflare Tunnel hostname first.")?;
    let uuid = store.kv_get(UUID).ok_or("The Cloudflare Tunnel isn't set up yet.")?;
    let credentials = saved_credentials(store)?.ok_or("The Cloudflare Tunnel isn't set up yet.")?;
    let port = PUBLIC_PORT.load(Ordering::Relaxed);
    if port == 0 {
        return Err("The phone listener isn't ready yet.".into());
    }
    stop_inner();
    private_dir()?;
    let creds = runtime_credentials_path();
    let started = (|| -> Result<Child, String> {
        write_private(&creds, credentials.as_bytes())
            .map_err(|error| format!("Couldn't prepare the Cloudflare Tunnel credentials: {error}"))?;
        write_private(config_path().as_path(), config(&uuid, &creds, &hostname).as_bytes())
            .map_err(|error| format!("Couldn't write the tunnel config: {error}"))?;

        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path())
            .map_err(|error| format!("Couldn't open the tunnel log: {error}"))?;
        let log_err = log.try_clone().map_err(|error| format!("Couldn't open the tunnel log: {error}"))?;
        let config = config_path().to_string_lossy().into_owned();
        let mut child = Command::new(binary)
            .args(["tunnel", "--config", &config, "--no-autoupdate", "run", &uuid])
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err))
            .spawn()
            .map_err(|error| format!("Couldn't start cloudflared: {error}"))?;
        std::thread::sleep(Duration::from_millis(150));
        match child.try_wait() {
            Ok(Some(exit)) => {
                let tail = log_tail();
                let message = if tail.is_empty() {
                    format!("cloudflared stopped ({exit}).")
                } else {
                    format!("cloudflared stopped: {tail}")
                };
                Err(message)
            }
            Ok(None) => Ok(child),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(format!("Couldn't check cloudflared: {error}"))
            }
        }
    })();
    let child = match started {
        Ok(child) => child,
        Err(error) => {
            let _ = fs::remove_file(&creds);
            RUNTIME.locked().last_error = Some(error.clone());
            return Err(error);
        }
    };
    let mut runtime = RUNTIME.locked();
    runtime.child = Some(child);
    runtime.credential_file = Some(creds);
    runtime.last_error = None;
    Ok(())
}

pub fn stop() {
    let _operation = OPERATION.locked();
    SHUTTING_DOWN.store(true, Ordering::Relaxed);
    stop_inner();
}

fn stop_inner() {
    let (child, credentials) = {
        let mut runtime = RUNTIME.locked();
        (runtime.child.take(), runtime.credential_file.take())
    };
    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }
    if let Some(path) = credentials {
        let _ = fs::remove_file(path);
    }
}

/// The process may have exited between calls: reap it and keep why it did.
fn refresh_running() {
    let mut runtime = RUNTIME.locked();
    let Some(child) = runtime.child.as_mut() else { return };
    match child.try_wait() {
        Ok(Some(exit)) => {
            runtime.child = None;
            if !exit.success() {
                let tail = log_tail();
                runtime.last_error = Some(if tail.is_empty() {
                    format!("cloudflared stopped ({exit}).")
                } else {
                    format!("cloudflared stopped: {tail}")
                });
            }
        }
        Ok(None) => {}
        Err(error) => {
            runtime.child = None;
            runtime.last_error = Some(format!("Couldn't check cloudflared: {error}"));
        }
    }
}

fn create_tunnel(binary: &Path, cert: &Path) -> Result<(String, String, String), String> {
    let name = format!("codync-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    private_dir()?;
    let creds = dir().join(format!("create-{}.json", uuid::Uuid::new_v4()));
    let result = (|| {
        let output = Command::new(binary)
            .args([
                "tunnel",
                "--origincert",
                &cert.to_string_lossy(),
                "create",
                "--credentials-file",
                &creds.to_string_lossy(),
                &name,
            ])
            .output()
            .map_err(|error| format!("Couldn't run cloudflared tunnel create: {error}"))?;
        if !output.status.success() {
            return Err(command_error("Cloudflare didn't create the tunnel", &output.stderr, &output.stdout));
        }
        secure_file(&creds);
        let raw =
            fs::read_to_string(&creds).map_err(|error| format!("Couldn't read the tunnel credentials: {error}"))?;
        let uuid = credentials_id(&raw)
            .ok_or_else(|| "Cloudflare created the tunnel but its credentials file has no tunnel id.".to_owned())?;
        Ok((name, uuid, raw))
    })();
    match fs::remove_file(&creds) {
        Ok(()) => {}
        Err(error) if result.is_ok() => return Err(format!("Couldn't remove temporary tunnel credentials: {error}")),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            tracing::warn!(path = %creds.display(), %error, "couldn't remove temporary Cloudflare credentials")
        }
        Err(_) => {}
    }
    result
}

fn route_dns(binary: &Path, cert: &Path, name: &str, hostname: &str) -> Result<(), String> {
    let output = Command::new(binary)
        .args(["tunnel", "--origincert", &cert.to_string_lossy(), "route", "dns", name, hostname])
        .output()
        .map_err(|error| format!("Couldn't run cloudflared tunnel route dns: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(command_error(
        &format!(
            "Cloudflare didn't point {hostname} at the tunnel. Check that the domain is in this Cloudflare account and the hostname isn't used by another record"
        ),
        &output.stderr,
        &output.stdout,
    ))
}

fn config(uuid: &str, creds: &Path, hostname: &str) -> String {
    let credentials = yaml_string(&creds.to_string_lossy());
    format!(
        "tunnel: {uuid}\ncredentials-file: {credentials}\ningress:\n  - hostname: {hostname}\n    service: http://127.0.0.1:{}\n  - service: http_status:404\n",
        PUBLIC_PORT.load(Ordering::Relaxed)
    )
}

/// Quotes a path as a YAML single-quoted scalar; apostrophes double inside YAML strings.
fn yaml_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// The tunnel id in `cloudflared`'s credentials JSON.
fn credentials_id(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    ["TunnelID", "tunnelID", "tunnelId", "id"].iter().find_map(|key| value[*key].as_str()).map(str::to_owned)
}

fn saved_credentials(store: &Store) -> Result<Option<String>, String> {
    if let Some(credentials) = vault::read(store, CREDS).map_err(|error| error.to_string())? {
        return Ok((!credentials.is_empty()).then_some(credentials));
    }
    let Some(path) = store.kv_get(LEGACY_CREDS_PATH) else { return Ok(None) };
    let credentials = fs::read_to_string(&path)
        .map_err(|error| format!("The saved Cloudflare Tunnel credentials are unavailable: {error}"))?;
    vault::write(store, CREDS, &credentials).map_err(|error| error.to_string())?;
    store.kv_delete(LEGACY_CREDS_PATH).map_err(|error| error.to_string())?;
    let _ = fs::remove_file(path);
    Ok(Some(credentials))
}

fn runtime_credentials_path() -> PathBuf {
    dir().join("runtime-credentials.json")
}

fn private_dir() -> Result<(), String> {
    fs::create_dir_all(dir()).map_err(|error| format!("Couldn't create {}: {error}", dir().display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir(), fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Couldn't secure {}: {error}", dir().display()))?;
    }
    Ok(())
}

fn secure_file(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(0o600)) {
            tracing::warn!(path = %path.display(), %error, "couldn't restrict Cloudflare credential file permissions");
        }
    }
}

fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn read_for_url(pipe: impl Read + Send + 'static) {
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            if let Some(url) = url_in(&line) {
                let mut runtime = RUNTIME.locked();
                if runtime.login_url.is_none() {
                    runtime.login_url = Some(url);
                }
            }
        }
    });
}

fn url_in(line: &str) -> Option<String> {
    let start = line.find("https://")?;
    let rest = &line[start..];
    let end =
        rest.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '>' || c == '<').unwrap_or(rest.len());
    (end > "https://".len()).then(|| rest[..end].to_owned())
}

fn command_error(what: &str, stderr: &[u8], stdout: &[u8]) -> String {
    let detail =
        String::from_utf8_lossy(stderr).trim().lines().last().map(str::to_owned).filter(|s| !s.is_empty()).or_else(
            || String::from_utf8_lossy(stdout).trim().lines().last().map(str::to_owned).filter(|s| !s.is_empty()),
        );
    match detail {
        Some(detail) => format!("{what}: {detail}"),
        None => format!("{what}."),
    }
}

fn log_tail() -> String {
    fs::read_to_string(log_path())
        .ok()
        .and_then(|text| text.lines().rev().find(|line| !line.trim().is_empty()).map(str::trim).map(str::to_owned))
        .unwrap_or_default()
}

/// `mac.example.com`, lower case, no scheme, port or path.
pub fn valid_hostname(raw: &str) -> Result<String, String> {
    let host = raw.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return Err("Enter the hostname Cloudflare should serve, such as codync.example.com.".into());
    }
    if host.contains("://") || host.contains('/') || host.contains('?') || host.contains('#') || host.contains(':') {
        return Err("Enter just the hostname, without https://, a port or a path.".into());
    }
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Err("Enter a hostname from your Cloudflare domain, not an IP address.".into());
    }
    if host.ends_with("trycloudflare.com") {
        return Err(
            "That is a temporary Quick Tunnel address. Use a hostname from a domain in your Cloudflare account.".into(),
        );
    }
    if host.split('.').count() < 2 {
        return Err("Enter the full hostname, such as codync.example.com.".into());
    }
    let labels_ok = host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    });
    if !labels_ok {
        return Err("That hostname has a part Cloudflare won't accept. Use letters, numbers and hyphens.".into());
    }
    Ok(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostnames_are_lower_case_and_plain() {
        assert_eq!(valid_hostname(" Codync.Example.COM. ").unwrap(), "codync.example.com");
        for bad in
            ["", "example", "https://example.com", "example.com/path", "example.com:443", "1.2.3.4", "*.example.com"]
        {
            assert!(valid_hostname(bad).is_err(), "{bad}");
        }
        assert!(valid_hostname("x.trycloudflare.com").is_err());
    }

    #[test]
    fn config_only_serves_the_tunnel_hostname_and_a_404() {
        let text = config("uuid-1", Path::new("/tmp/t.json"), "codync.example.com");
        assert!(text.contains("tunnel: uuid-1"));
        assert!(text.contains("- hostname: codync.example.com"));
        assert!(text.contains("service: http://127.0.0.1:"));
        assert!(text.trim_end().ends_with("- service: http_status:404"));
    }

    #[test]
    fn reads_the_login_url_from_a_line() {
        assert_eq!(
            url_in("Please open https://dash.cloudflare.com/argotunnel?callback=abc and log in").as_deref(),
            Some("https://dash.cloudflare.com/argotunnel?callback=abc")
        );
        assert_eq!(url_in("no link here"), None);
    }
}
