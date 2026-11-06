package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.decodeFromJsonElement

/**
 * Reaching this computer away from home without a Codync account (`reachStatus`): Tailscale's
 * private network, or a named Cloudflare Tunnel. Both serve only the end-to-end phone channel.
 */
@Serializable
data class ReachStatus(
    val tailscale: TailscaleReach = TailscaleReach(),
    val cloudflare: CloudflareReach = CloudflareReach(),
) {
    companion object {
        fun decode(value: JsonElement): ReachStatus = WireJson.decodeFromJsonElement(value)
    }
}

/** Tailscale on the computer, as its host sees it. The phone's own state is [TailscaleOnPhone]. */
@Serializable
data class TailscaleReach(
    val installed: Boolean = false,
    val running: Boolean = false,
    val canOpen: Boolean = false,
    /** The MagicDNS name, without the trailing dot (`mac.example.ts.net`). */
    val dnsName: String? = null,
    /** The first IPv4 address on the tailnet (`100.x.y.z`). */
    val ipv4: String? = null,
    val download: String = "https://tailscale.com/download",
)

/** A named Cloudflare Tunnel on the computer (`reachStatus`). */
@Serializable
data class CloudflareReach(
    val installed: Boolean = false,
    val loggedIn: Boolean = false,
    val configured: Boolean = false,
    val enabled: Boolean = false,
    val tunnel: String? = null,
    val hostname: String? = null,
    val url: String? = null,
    val running: Boolean = false,
    /** A browser sign-in is waiting; the link can be opened on this phone or the host computer. */
    val loginPending: Boolean = false,
    val loginUrl: String? = null,
    val lastError: String? = null,
    val download: String = "https://developers.cloudflare.com/tunnel/downloads/",
)

/** Tailscale gives every device an address in 100.64.0.0/10 on a VPN (`tun`) interface. */
object TailscaleOnPhone {
    val isConnected: Boolean
        get() = runCatching {
            java.net.NetworkInterface.getNetworkInterfaces().toList().any { iface ->
                val name = iface.name.lowercase()
                (name.startsWith("tun") || name.startsWith("utun") || name.contains("tailscale")) && iface.inetAddresses.toList().any { address ->
                    val bytes = address.address
                    address is java.net.Inet4Address && bytes.size == 4 &&
                        bytes[0].toInt() and 0xff == 100 && (bytes[1].toInt() and 0xff) in 64..127
                }
            }
        }.getOrDefault(false)
}
