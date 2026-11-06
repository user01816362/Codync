package com.codync.android

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.codync.android.core.CloudflareReach
import com.codync.android.core.ReachStatus
import com.codync.android.core.TailscaleOnPhone
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import com.kyant.shapes.RoundedRectangle

/**
 * Reaching this computer away from home without a Codync account (iOS `ReachSheet`): Tailscale's
 * private network, or a named Cloudflare Tunnel. Both serve only the end-to-end phone channel.
 */
@Composable internal fun ReachSheet(store: AppStore) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    var status by remember { mutableStateOf<ReachStatus?>(null) }
    var hostname by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var confirmForget by remember { mutableStateOf(false) }
    var phoneTailscale by remember { mutableStateOf(TailscaleOnPhone.isConnected) }

    suspend fun read(timeout: Long = 20_000): ReachStatus =
        ReachStatus.decode(store.ready().call("reachStatus", timeoutMillis = timeout))

    fun refresh() {
        scope.launch {
            try { status = read() }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Couldn't reach the computer." }
        }
    }

    fun perform(what: String, method: String, body: JsonElement = JsonNull, timeout: Long = 20_000) {
        busy = true
        error = null
        scope.launch {
            try { status = ReachStatus.decode(store.ready().call(method, body, timeout)) }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Couldn't $what." }
            finally { busy = false }
        }
    }

    fun open(url: String?) {
        if (!url.isNullOrEmpty()) context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
    }

    val cloudflare = status?.cloudflare
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    DisposableEffect(lifecycle) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) phoneTailscale = TailscaleOnPhone.isConnected
        }
        lifecycle.addObserver(observer)
        onDispose { lifecycle.removeObserver(observer) }
    }

    LaunchedEffect(store) { refresh() }

    // A Cloudflare sign-in finishes in the browser, not through this app: read the status
    // again until cloudflared has written the account certificate.
    LaunchedEffect(cloudflare?.loginPending) {
        if (cloudflare?.loginPending != true) return@LaunchedEffect
        while (true) {
            delay(2_000)
            try { status = read() } catch (cancelled: CancellationException) { throw cancelled } catch (_: Exception) {}
            if (status?.cloudflare?.loginPending != true) break
        }
    }

    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Reach from anywhere") {
            if (busy) Spinner(16.dp)
            IconButton("Check again", "arrow.clockwise", enabled = !busy) { refresh() }
        }
        CardForm {
            CardSection("Tailscale",
                footer = "Install Tailscale on this computer and on this phone, then sign in to both. Codync connects straight over your tailnet; no Codync account is involved.") {
                row {
                    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(hostLine(status?.tailscale), style = Type.body, color = Palette.text)
                        Text(phoneLine(phoneTailscale), style = Type.footnote, color = if (phoneTailscale) Palette.secondary else Palette.warning)
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            if (status?.tailscale?.installed == false) {
                                SecondaryButton("Get Tailscale") { open(status?.tailscale?.download) }
                            } else if (status?.tailscale?.running == false && status?.tailscale?.canOpen == true) {
                                SecondaryButton("Open on this computer", enabled = !busy) {
                                    perform("open Tailscale", "reachTailscaleOpen")
                                }
                            }
                            if (!phoneTailscale) {
                                SecondaryButton("Get Tailscale for Android") { open("https://tailscale.com/docs/install/android") }
                            }
                        }
                    }
                }
            }

            CardSection("Cloudflare Tunnel",
                footer = "A named tunnel from a domain in your Cloudflare account. Only Codync's end-to-end encrypted phone channel is served through it, never the computer's files, terminals or local API.") {
                row {
                    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(cloudflareLine(cloudflare), style = Type.body,
                            color = if (cloudflare?.lastError.isNullOrEmpty()) Palette.text else Palette.warning)
                        cloudflareActions(
                            cloudflare = cloudflare,
                            busy = busy,
                            hostname = hostname,
                            onHostname = { hostname = it },
                            onInstall = { open(cloudflare?.download) },
                            onOpenLogin = { open(cloudflare?.loginUrl) },
                            onLogin = { perform("sign in", "reachCloudflareLogin") },
                            onCreate = {
                                val host = hostname.trim()
                                if (host.isNotEmpty()) perform("create the tunnel", "reachCloudflareCreate",
                                    buildJsonObject { put("hostname", host) }, timeout = 120_000)
                            },
                            onSetEnabled = { on -> perform("change the tunnel", "reachCloudflareSetEnabled",
                                buildJsonObject { put("enabled", on) }) },
                            onForget = { confirmForget = true },
                        )
                    }
                }
            }

            if (error != null) CardSection {
                row { Text(error ?: "", style = Type.footnote, color = Palette.danger) }
            }
        }
    }

    CodyncDialog("Forget this Cloudflare Tunnel here?", confirmForget, { confirmForget = false },
        "The tunnel stops and this computer forgets its hostname. The tunnel and DNS record stay in your Cloudflare account; delete them there if you no longer want them.") {
        listOf(DialogAction("Forget", destructive = true) { perform("forget the tunnel", "reachCloudflareForget") })
    }
}

/** The computer's Tailscale line, as the host sees it. */
@Composable private fun hostLine(tailscale: com.codync.android.core.TailscaleReach?): String = when {
    tailscale == null -> tr("Checking…")
    !tailscale.installed -> tr("Tailscale isn't installed on this computer.")
    !tailscale.running -> tr("Tailscale is installed on this computer, but not signed in or not running.")
    !tailscale.dnsName.isNullOrEmpty() -> tr("This computer is on your tailnet at {name}.").replace("{name}", tailscale.dnsName.orEmpty())
    !tailscale.ipv4.isNullOrEmpty() -> tr("This computer is on your tailnet at {ip}.").replace("{ip}", tailscale.ipv4.orEmpty())
    else -> tr("This computer is running on your tailnet.")
}

@Composable private fun phoneLine(on: Boolean): String =
    if (on) tr("This phone is on Tailscale.")
    else tr("This phone isn't on Tailscale yet. Install it to connect away from home without Cloudflare.")

@Composable private fun cloudflareLine(cloudflare: CloudflareReach?): String = when {
    cloudflare == null -> tr("Checking…")
    !cloudflare.installed -> tr("Install cloudflared on this computer first.")
    cloudflare.loginPending -> tr("Finish signing in in a browser on this device or the computer running Codync.")
    !cloudflare.lastError.isNullOrEmpty() -> cloudflare.lastError.orEmpty()
    !cloudflare.loggedIn -> tr("Sign in to the Cloudflare account that holds your domain.")
    !cloudflare.configured -> tr("Choose a hostname from a domain in that account.")
    cloudflare.running && !cloudflare.url.isNullOrEmpty() -> tr("Tunnel process running for {url}.").replace("{url}", cloudflare.url.orEmpty())
    else -> tr("Stopped. Turn it on to reach this computer from anywhere.")
}

@Composable private fun cloudflareActions(
    cloudflare: CloudflareReach?,
    busy: Boolean,
    hostname: String,
    onHostname: (String) -> Unit,
    onInstall: () -> Unit,
    onOpenLogin: () -> Unit,
    onLogin: () -> Unit,
    onCreate: () -> Unit,
    onSetEnabled: (Boolean) -> Unit,
    onForget: () -> Unit,
) {
    if (cloudflare == null) return
    if (!cloudflare.installed) {
        SecondaryButton("Install cloudflared") { onInstall() }
    } else if (cloudflare.loginPending) {
        SecondaryButton("Continue Cloudflare sign-in", enabled = !cloudflare.loginUrl.isNullOrEmpty()) { onOpenLogin() }
    } else if (!cloudflare.loggedIn) {
        SecondaryButton("Sign in to Cloudflare", enabled = !busy) { onLogin() }
    } else if (!cloudflare.configured) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Box(Modifier.weight(1f).background(Palette.surface, RoundedRectangle(10.dp)).padding(horizontal = 12.dp, vertical = 8.dp)) {
                PlainTextField(hostname, onHostname, Modifier.fillMaxWidth(), "codync.example.com", singleLine = true)
            }
            SecondaryButton("Create tunnel", enabled = !busy && hostname.isNotBlank()) { onCreate() }
        }
    } else {
        CodyncSwitch(cloudflare.enabled, { onSetEnabled(it) }, Modifier.fillMaxWidth(), enabled = !busy) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(cloudflare.hostname ?: tr("Cloudflare Tunnel"), style = Type.body, color = Palette.text)
                Text(if (cloudflare.running) tr("Tunnel running") else tr("Tunnel stopped"), style = Type.footnote, color = Palette.secondary)
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SecondaryButton("Restart tunnel", enabled = !busy) { onSetEnabled(true) }
            SecondaryButton("Forget this tunnel", enabled = !busy) { onForget() }
        }
    }
}
