# Codync, local fork

A fork of [leepokai/Codync](https://github.com/leepokai/Codync) that runs without an
account and without the hosted Codync cloud.

## What changed

| Area | Change |
|---|---|
| Host pairing | A **direct** (LAN, Tailscale, in-person) pairing is let in without a cloud-signed device certificate. A relay link still has to prove its account. |
| Host telemetry | Off, always. The settings switch and `codync-host analytics on` cannot turn it back on. |
| Codync account UI | Hidden in iPhone, Android and desktop apps when this fork has no Codync cloud configuration. Pair computers directly with a QR code. |
| iPhone analytics | Usage analytics and crash reporting have been removed from the iPhone app. |
| Host cloud | No built-in cloud address. The computer is local-only unless `CODYNC_CLOUD_URL` (or an address saved in Settings) names one. |
| Remote access | Wi-Fi, Tailscale and a named Cloudflare Tunnel are available without a Codync account. Cloudflare exposes only the phone channel and a minimal health check. |
| App configs | `apps/shared/Config/{main,dev}.plist` ship an empty Clerk key and cloud URL, so every client takes its existing signed-out path. |
| iPhone / Android | Pairing omits the certificate when the build has no cloud, instead of failing with "Connect to the internet to pair". |
| CI | `.github/workflows/fork-build.yml` builds an unsigned Apple Silicon macOS app on every push to `main`; the standalone `.github/workflows/ios-ipa.yml` builds an unsigned iPhone IPA. |

## Download and run the macOS app

1. Open **Releases** and download `codync-macos.zip` (or `codync-macos.dmg`) from the newest
   `vX.Y.Z` release. Every push to `main` publishes one; a build without a version bump replaces
   that version's assets.
2. Unzip it, then move `Codync.app` to `/Applications`.
3. The app is ad-hoc signed, not notarized. Clear the download quarantine once:

   ```sh
   xattr -dr com.apple.quarantine /Applications/Codync.app
   open /Applications/Codync.app
   ```

   If macOS still refuses the first launch, right-click the app → **Open**.

Requires an Apple Silicon Mac (arm64 only).

## Download the unsigned iPhone IPA

The standalone **iOS IPA (unsigned)** workflow runs after the macOS fork build succeeds and
publishes `codync-ios-unsigned.ipa` and its `.sha256` on the newest `vX.Y.Z` release. It can also
be dispatched by hand from the Actions tab. It is built with `CODE_SIGNING_ALLOWED=NO`: no Apple
account, provisioning profile, Xcode Cloud or App Store Connect is used.

iOS will not install it as-is. Re-sign it with your own Apple ID or team (Sideloadly, AltStore,
Xcode), then install it. A free Apple ID may not cover the app group, push and Live Activity
entitlements this app uses; a paid developer team's profile does. Without re-signing it only
works on a jailbroken device or with TrollStore.

## What still needs a server

- Push notifications while the app is closed (APNs/FCM need a server).
- The address book ("find my computers"), short pairing codes and cloud assistants.
- Relay/TURN remote screen across networks: use LAN or Tailscale instead.
- Signed iPhone and Android builds: the CI IPA is unsigned; sign it yourself with your own identity.
