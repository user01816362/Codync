//! Reaching this computer away from home without a Codync account: a named
//! Cloudflare Tunnel, or Tailscale's own private network. Both are ordinary direct
//! HTTPS/HTTP addresses to the phone channel; neither needs the Codync cloud.
//!
//! The tunnel listener is a second HTTP server on `127.0.0.1` that serves only
//! `/channel` and `/health`. `cloudflared` connects to it, so the tunnel never
//! exposes this computer's local API, terminals, OAuth callbacks or bot webhooks.

pub mod cloudflare;
pub mod tailscale;

use crate::hub::Hub;
use crate::store::Store;
use anyhow::{Context, Result};
use serde::Serialize;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;

/// The loopback port `cloudflared` points at; 0 until [`start`] binds it.
pub static PUBLIC_PORT: AtomicU16 = AtomicU16::new(0);

/// The hostname `cloudflared` serves, once it is configured. Kept here because
/// [`crate::service::addresses`] is synchronous and has no `Store`.
static PUBLIC_URL: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub tailscale: tailscale::Status,
    pub cloudflare: cloudflare::Status,
}

pub fn status(store: &Store) -> Status {
    Status { tailscale: tailscale::status(), cloudflare: cloudflare::status(store) }
}

/// Reads the saved tunnel hostname at startup, before the first pairing link.
pub fn init(store: &Store) {
    let on = store.kv_get("reach_cloudflare_enabled").as_deref() != Some("false");
    let url = on.then(|| store.kv_get("reach_cloudflare_hostname")).flatten().map(|host| format!("https://{host}"));
    set_public_url(url.as_deref());
}

pub fn public_url() -> Option<String> {
    PUBLIC_URL.read().ok().and_then(|url| url.clone())
}

pub(crate) fn set_public_url(url: Option<&str>) {
    if let Ok(mut current) = PUBLIC_URL.write() {
        *current = url.map(str::to_owned);
    }
}

/// Starts the loopback channel listener and supervises a saved tunnel.
pub async fn start(hub: Arc<Hub>) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .context("binding the local Cloudflare Tunnel listener")?;
    let port = listener.local_addr()?.port();
    PUBLIC_PORT.store(port, Ordering::Relaxed);
    let app = crate::api::public_router(hub.clone()).into_make_service_with_connect_info::<std::net::SocketAddr>();
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            tracing::warn!(%error, "the Cloudflare Tunnel listener stopped");
        }
    });
    let supervisor = hub.clone();
    tokio::spawn(async move {
        let mut retry = tokio::time::interval(Duration::from_secs(60));
        loop {
            retry.tick().await;
            if !cloudflare::enabled(&supervisor.store) {
                continue;
            }
            let needs_unlock = supervisor.store.secret_key.locked().is_none();
            if needs_unlock && let Err(error) = crate::market::vault::unlock(supervisor.clone()).await {
                cloudflare::set_error(error.to_string());
                tracing::warn!(%error, "couldn't unlock the Cloudflare Tunnel credentials");
                continue;
            }
            let hub = supervisor.clone();
            if let Err(error) = tokio::task::spawn_blocking(move || {
                if cloudflare::enabled(&hub.store) && !cloudflare::status(&hub.store).running {
                    cloudflare::start(&hub.store)
                } else {
                    Ok(())
                }
            })
            .await
            .unwrap_or_else(|error| Err(error.to_string()))
            {
                cloudflare::set_error(error.clone());
                tracing::warn!(%error, "the Cloudflare Tunnel isn't running");
            }
        }
    });
    Ok(())
}

/// Stops the supervised tunnel before the host exits.
pub fn stop() {
    cloudflare::stop();
}

/// Tells connected apps to read the status again.
pub fn changed(hub: &Hub) {
    let _ = hub.events.send(serde_json::json!({"type": "reach"}));
}

/// Opens the Tailscale app on this computer so its user can sign in there.
pub fn open_tailscale() -> Result<()> {
    tailscale::open_login().map_err(|error| anyhow::anyhow!(error))
}
