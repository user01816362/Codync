//! Reaching this computer without the Codync cloud: the Cloudflare Tunnel and
//! Tailscale settings a person opens from this computer or a paired phone.

use super::str_arg;
use crate::hub::Hub;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::ops::ControlFlow;
use std::sync::Arc;

pub(super) async fn call(hub: &Arc<Hub>, method: &str, b: Value) -> Result<ControlFlow<Value, Value>> {
    Ok(ControlFlow::Break(match method {
        "reachStatus" => {
            let hub = hub.clone();
            let status = tokio::task::spawn_blocking(move || crate::reach::status(&hub.store)).await?;
            serde_json::to_value(status)?
        }
        "reachTailscaleOpen" => {
            let opened = hub.clone();
            tokio::task::spawn_blocking(crate::reach::open_tailscale).await??;
            crate::reach::changed(hub);
            let status = tokio::task::spawn_blocking(move || crate::reach::status(&opened.store)).await?;
            serde_json::to_value(status)?
        }
        "reachCloudflareLogin" => {
            let work = hub.clone();
            tokio::task::spawn_blocking(crate::reach::cloudflare::login).await?.map_err(|error| anyhow!(error))?;
            crate::reach::changed(hub);
            let status = tokio::task::spawn_blocking(move || crate::reach::status(&work.store)).await?;
            serde_json::to_value(status)?
        }
        "reachCloudflareCreate" => {
            let hostname = str_arg(&b, "hostname")?.to_owned();
            crate::market::vault::unlock(hub.clone()).await?;
            let work = hub.clone();
            tokio::task::spawn_blocking(move || crate::reach::cloudflare::create(&work.store, &hostname))
                .await?
                .map_err(|error| anyhow!(error))?;
            crate::reach::changed(hub);
            let status = tokio::task::spawn_blocking({
                let hub = hub.clone();
                move || crate::reach::status(&hub.store)
            })
            .await?;
            serde_json::to_value(status)?
        }
        "reachCloudflareSetEnabled" => {
            let enabled = b["enabled"].as_bool().ok_or_else(|| anyhow!("`enabled` is required"))?;
            if enabled {
                crate::market::vault::unlock(hub.clone()).await?;
            }
            let work = hub.clone();
            tokio::task::spawn_blocking(move || crate::reach::cloudflare::set_enabled(&work.store, enabled))
                .await?
                .map_err(|error| anyhow!(error))?;
            crate::reach::changed(hub);
            let status = tokio::task::spawn_blocking({
                let hub = hub.clone();
                move || crate::reach::status(&hub.store)
            })
            .await?;
            serde_json::to_value(status)?
        }
        "reachCloudflareForget" => {
            let work = hub.clone();
            tokio::task::spawn_blocking(move || crate::reach::cloudflare::forget(&work.store))
                .await?
                .map_err(|error| anyhow!(error))?;
            crate::reach::changed(hub);
            let status = tokio::task::spawn_blocking({
                let hub = hub.clone();
                move || crate::reach::status(&hub.store)
            })
            .await?;
            serde_json::to_value(status)?
        }
        _ => return Ok(ControlFlow::Continue(b)),
    }))
}
