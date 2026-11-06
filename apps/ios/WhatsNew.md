<!--
App Store "What's New" for the iPhone app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per App Store language (zh-Hant, en-US),
listing what iPhone users notice, without emoji (the App Store rejects them; asc-submit.py drops
any left). tools/asc-submit.py reads the released version's section;
a version without one gets a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.40.0
### zh-Hant
- 在「隨時隨地連線」設定中設定 Tailscale 或 Cloudflare Tunnel，不必登入 Codync 帳號；掃描 QR Code 配對即可。
- iPhone app 不再傳送使用分析或當機報告。

### en-US
- Set up Tailscale or a named Cloudflare Tunnel in Reach from anywhere. No Codync account is needed; pair by scanning the QR code.
- The iPhone app no longer sends usage analytics or crash reports.

## 2.39.1
### zh-Hant
- 現在會在你從這支 iPhone 送出第五則訊息後，邀請你到 App Store 評分，一年最多兩次。
- 推理強度滑桿新增漸層點陣，讓不同強度更容易辨識。

### en-US
- Codync now invites you to rate it on the App Store after your fifth message from this iPhone, at most twice a year.
- The reasoning effort slider now uses a dotted gradient to make levels easier to distinguish.
