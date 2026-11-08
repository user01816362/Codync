# Codync setup

Based on [leepokai/Codync](https://github.com/leepokai/Codync), this version runs without an
account and without the hosted Codync cloud.

## What changed

| Area | Change |
|---|---|
| Host pairing | Direct pairing (LAN, Tailscale or Cloudflare Tunnel) and a configured private relay support phones without an account. The computer asks before allowing a new phone; the hosted Codync relay still requires account proof. |
| Host telemetry | Off, always. The settings switch and `codync-host analytics on` cannot turn it back on. |
| Codync account UI | Hidden in iPhone, Android and desktop apps when this repository has no Codync cloud configuration. Pair computers directly with a QR code. |
| iPhone analytics | Usage analytics and crash reporting have been removed from the iPhone app. |
| Host cloud | No built-in cloud address. The computer is local-only unless `CODYNC_CLOUD_URL` (or an address saved in Settings) names one. |
| Remote access | Wi-Fi, Tailscale and a named Cloudflare Tunnel are available without a Codync account. Cloudflare exposes only the phone channel and a minimal health check. |
| App configs | `apps/shared/Config/{main,dev}.plist` ship an empty Clerk key and cloud URL, so every client takes its existing signed-out path. |
| iPhone / Android | Pairing omits the certificate when the build has no cloud, instead of failing with "Connect to the internet to pair". |
| CI | [Build workflow](.github/workflows/fork-build.yml) builds an unsigned Apple Silicon macOS app on every push to `main`; the standalone `.github/workflows/ios-ipa.yml` builds an unsigned iPhone IPA. |

## Chat and agent features added in this version

- Bounded structured reports: expandable sections, comparisons, timelines, bar charts,
  text-answer forms and source links, with a text fallback for the terminal and older clients.
- Explicit streamed replies and direct ACP image snapshots in supported own-chat turns.
- Offline code highlighting, Mermaid and math previews; readable Markdown/file previews.
- Collaboration visibility controls, bounded mention handoffs and delivery/read-status tools.
- Desktop selected-text quotes with author and source-message context in the reply bar.

See the [coverage audit](docs/features/rich-agent-responses.md) for limits and platform gaps.
Phone selected-text quotes still become draft text, image generation depends on the provider,
and seen acknowledgements do not prove comprehension. Real-device and accessibility checks
remain outstanding. **Hidden collaboration activity changes presentation, not bot execution.**

## Download and run the macOS app

1. Open **Releases** and download `codync-macos.zip` (or `codync-macos.dmg`) from the newest
   release. Releases use normal version tags such as `v2.41.1`. Later builds at the same version remain available in GitHub Actions artifacts.
2. Unzip it, then move `Codync.app` to `/Applications`.
3. The app is ad-hoc signed, not notarized. Clear the download quarantine once:

   ```sh
   xattr -dr com.apple.quarantine /Applications/Codync.app
   open /Applications/Codync.app
   ```

   If macOS still refuses the first launch, right-click the app → **Open**.

Requires an Apple Silicon Mac (arm64 only).

## Download the unsigned iPhone IPA

The standalone **iOS IPA (unsigned)** workflow runs after the macOS build succeeds and
attaches `codync-ios-unsigned.ipa` and its `.sha256` only to a release for that exact commit.
Otherwise download them from the workflow artifacts. It can also
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

## Upstream sync

Merge upstream `main` into this repository; do not use GitHub's **Discard commits** option.
Keep our commits and resolve overlapping changes explicitly. New upstream versions
are adopted during that merge. CI remains Apple-only, and iPhone analytics stay removed.

Version 2.41.1 reconciles two independent schema-8 migrations: upstream's signed-out-device
field and our cleared-chat tracking. Schema 9 checks for both additions, preserves
existing paired devices and chat clear marks, and handles databases from either repository.

## Updates

The ad-hoc macOS app checks releases from `user01816362/Codync`, when a newer app version is published. Settings → Updates and the menu bar offer a direct DMG
download when one is available. Automatic checks can be turned off.

These builds cannot use Electron's signed macOS installer. Replace the app in Applications
after downloading; automatic installation requires a consistent signing identity and a signed
update feed. The automatic-install setting is hidden for these packages. The iPhone IPA still
requires re-signing and installation with your sideloading tool.

Install a new build manually once to get this checker: existing installed apps cannot
acquire the fix through their broken updater. CI embeds its build ID before ad-hoc signing,
and publishes normal `v<version>` release tags without changing the app's version. Existing
tags and releases are preserved; later same-version builds stay in Actions artifacts.
Electron packaging also points at this repository, so a future signed feed cannot select upstream.

References: [Electron autoUpdater](https://www.electronjs.org/docs/latest/api/auto-updater),
[electron-builder auto updates](https://www.electron.build/v26/docs/features/auto-update/).
