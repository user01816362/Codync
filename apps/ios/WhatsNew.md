<!--
App Store "What's New" for the iPhone app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per App Store language (zh-Hant, en-US),
listing what iPhone users notice, without emoji (the App Store rejects them; asc-submit.py drops
any left). tools/asc-submit.py reads the released version's section;
a version without one gets a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.43.1
### zh-Hant
- 預設可選擇不登入帳號，使用 Wi-Fi、Tailscale、Cloudflare Tunnel 或私人中繼；也可選擇登入以使用帳號與託管電腦功能。
- 修正延遲的群組回覆未通知提及成員的問題，保留可讀名稱與閱讀紀錄。
- 改善 OpenCode 舊版本啟動相容性及模型拒絕後的狀態處理。
- 自訂連接器不能再使用已安裝連接器的名稱。

### en-US
- Continue without an account using Wi-Fi, Tailscale, Cloudflare Tunnel or a private relay, or optionally sign in for account and hosted-computer features.
- Late group replies now notify the members they mention, retaining readable names and read receipts.
- Improved older OpenCode startup compatibility and handling of model refusals.
- Custom connectors now reject names already used by installed connectors.

## zh-Hant
- 新功能 Codync Cloud（Beta）：登入 Codync 帳號後，可以在你自己的 Boat（歐洲）或 InstaBox（美國）帳號上開雲端電腦，你的電腦沒開，bot 也能工作。Codync 不收費，機器由供應商依它的方案向你的帳號收費。在「管理電腦」的 Codync Cloud 區塊按 +。
- 雲端電腦閒置時會睡著，傳訊息給它的 bot 就會醒來。每個帳號最多三台，各自命名。
- 還沒有 InstaBox 帳號？它的免費方案有一台機器，在 InstaBox 的 key 頁面點「Get a free sandbox」。

### en-US
- New: Codync Cloud (beta). Signed in to a Codync account, start a cloud computer on your own Boat (EU) or InstaBox (US) account, so your bots work while your computer is off. Codync charges nothing; the provider bills your account under its own plan. In Manage computers, choose + in the Codync Cloud section.
- A cloud computer sleeps while idle and wakes when you message one of its bots. Up to three per account, each with its own name.
- No InstaBox account yet? Its free plan includes one machine: choose Get a free sandbox on InstaBox's key sheet.

## 2.42.1
### zh-Hant
- 文字預覽新增搜尋與檢視偏好；資料庫可載入更早的檔案並定位原訊息，支援引用檔案、工作區與成品中的段落。
- 群組設定新增本機自訂圖片，以及可暫停、繼續、指派負責人與查看歷程的任務。佇列支援暫停與排序，主機儲存空間可查看檔案大小及壓縮資料庫。
- 對話輸入區顯示智慧體回報的上下文用量；公式段落保留文字樣式並支援手機字級縮放。
- 群組收件匣可選擇自動核准新請求，並指定負責回覆的成員。
- 群組編輯新增清楚標示的欄位及群組收件匣：選擇負責成員後核准外部請求，再將部門回覆傳回原對話。成員可向其他已加入的群組傳訊。
- 選取訊息文字後引用時，會顯示原作者與引用段落的回覆列，不再將引用內容插入草稿。文字檔預覽新增原始碼、複製、換行與重試；資料庫可篩選檔案並直接預覽文字檔。
- bot 詳情「智慧體」區塊的「壓縮對話」預設改為「自動」：交給 Claude Code 或 Codex 在對話快滿時自己壓縮。想讓 Codync 提早壓縮，可以改選提早、接近上限或指定數量（從 100K、200K… 裡挑，或自己輸入）。已經是「提早」的 bot 會保留原設定，想交給智慧體自己壓縮可以改選「自動」。

### en-US
- Search and remember text preview controls, browse older Library files and jump to their messages, and quote verified file/workspace or artifact passages.
- Group settings add local custom images and owned tasks with pause/resume, reassignment and history. Queue controls add pause and ordering; host storage shows file totals and database compaction.
- Reported context is visible beside chat input; bounded formula paragraphs preserve inline styles and phone text scaling.
- Group inbox settings can automatically approve new requests and assign a member to handle replies.
- Group editing has labeled fields and an inbox: approve outside requests, choose a handler and route the department reply back. Members can message other joined groups.
- Selected message passages now use the reply bar with their author and source. Text previews add Read/Source, copy, wrap and retry; the Library can filter files and open supported text previews.
- Summarize long chats, in a bot's Agent section, now defaults to Auto: Claude Code or Codex summarizes a chat on its own once it fills up. Pick Early, Near the limit or a size (100K, 200K and so on, or one you type) to have Codync summarize sooner. Bots already on Early keep it; pick Auto to leave it to the agent.

## 2.42.0
### zh-Hant
- bot 詳情的「智慧體」區塊新增「壓縮對話」：選擇 Codync 什麼時候把長對話壓縮成摘要。可以選提早（預設）、接近上限、不主動壓縮，或指定數量：從 100K、200K… 裡挑，或自己輸入，最多到模型裝得下的上限。適用於 Claude Code 和 Codex 的 bot。

### en-US
- New Summarize long chats setting in a bot's Agent section: choose when Codync summarizes a long chat. Pick Early (default), Near the limit, Only when full, or a size: 100K, 200K and so on, or one you type, up to what the model holds. For Claude Code and Codex bots.

## 2.41.0
### zh-Hant
- 不登入也能用了：在歡迎畫面選擇「不登入繼續」，手機掃電腦的 QR 碼就能配對，透過 Wi-Fi 或 Tailscale 連線。之後登入，配對會留在手機上；到「管理電腦」按「加入帳號」，電腦確認後就能從任何地方連線。
- 電腦可以不加入帳號。配對時電腦會詢問；不加入的話，手機只能用 Wi-Fi 或 Tailscale 連線，之後也能在「管理電腦」加入帳號。
- 遠端畫面會照連線設定換路：自動是先直連再走 Cloudflare，Cloudflare 優先則相反。
- 群組裡輸入 @ 會列出成員，一排一個，和 / 指令選單一樣。

### en-US
- Use Codync without an account: choose Continue without an account on the welcome screen, then scan your computer's QR code. It connects over Wi-Fi or Tailscale. Sign in later and your computers stay paired; Add to account in Manage computers brings each into your account once the computer confirms.
- A computer can stay out of your account. It asks when you pair; without it, your phone connects only over Wi-Fi or Tailscale, and you can add it later in Manage computers.
- Remote screen now follows the Connection setting: Auto tries direct first, then Cloudflare, and Cloudflare first does the reverse.
- Typing @ in a group lists its members one per row, like the / command list.

## 2.40.3
### zh-Hant
- Markdown 預設顯示易讀的預覽，可在右上角切換原始碼，並修正空白預覽和長路徑顯示。
- 可在設定中隱藏協作活動、顯示精簡摘要或預設展開，並選取訊息文字加入引用草稿。
- 對話支援可展開的報告、比較、時間軸、長條圖及文字回答表單，並顯示來源連結。
- 程式碼、Mermaid 圖表及數學公式可離線預覽，保留原始文字。
- 改善群組 bot 忙碌後的提及接續，並顯示傳送失敗或達到回覆上限的提示。

### en-US
- Markdown opens as a readable preview with a small Code/Preview toggle; fixed blank previews and long task paths.
- Choose Hidden, Compact or Expanded collaboration activity, and quote selected message text into your draft.
- Read expandable reports, comparisons, timelines, bar charts and text-answer forms with source links.
- Preview highlighted code, Mermaid diagrams and math offline while keeping the original source.
- Improved mention handoffs after busy bots reply, with visible delivery failures and room limits.

## 2.40.3
### zh-Hant
- bot 現在可以讓 dev server 持續在電腦上執行，不會因為 bot 閒置或重新啟動而中斷。port 會顯示在聊天標題下方；點 bot 的名稱，就能看記錄、重新啟動或停止；手機連得到電腦時，也能直接開啟。
- 用量頁面可以自己選要看哪些服務並調整順序：預設只顯示 bot 正在用的，也可以打開或隱藏其他服務，或拖曳排列。另外支援顯示 Google Antigravity 和 GitHub Copilot 的額度。
- 修正 bot 更新時，選單偶爾點了沒反應的問題。
- 修正 iOS 27.2 之後的部分測試版上，登入後 app 當掉，之後每次開啟都立刻關閉（重新安裝也一樣）的問題。

### en-US
- Bots can now keep a dev server running on your computer, even when the bot goes idle or restarts. Its port shows under the chat title; tap the bot's name to read its logs, restart or stop it, and to open it when your phone can reach it.
- Choose and arrange which agents show in Usage: by default it lists the ones your bots run, and you can turn others on, hide them or drag them into order. Usage now also reads Google Antigravity's and GitHub Copilot's limits.
- Fixed menus sometimes ignoring a tap while a bot was updating.
- Fixed a crash on some betas of iOS 27.2 and later that closed the app after sign-in and at every launch afterwards, even after reinstalling.

## 2.40.0
### zh-Hant
- 在「隨時隨地連線」設定中設定 Tailscale 或 Cloudflare Tunnel，不必登入 Codync 帳號；掃描 QR Code 配對即可。
- iPhone app 不再傳送使用分析或當機報告。

### en-US
- Set up Tailscale or a named Cloudflare Tunnel in Reach from anywhere. No Codync account is needed; pair by scanning the QR code.
- The iPhone app no longer sends usage analytics or crash reports.

## 2.39.1
### zh-Hant
- 開啟 bot 分享或你傳送的 Markdown 和程式碼檔案，直接在對話中預覽。
- 程式碼區塊現在可以直接複製，並顯示程式語言。
- 開啟協作活動，查看 bot 之間的請求、回覆和進度。
- 現在會在你從這支 iPhone 送出第五則訊息後，邀請你到 App Store 評分，一年最多兩次。
- 推理強度滑桿新增漸層點陣，讓不同強度更容易辨識。

### en-US
- Preview Markdown and code files shared by bots or sent by you in your conversation.
- Copy code blocks directly and see their language.
- Open collaboration activity to see requests, replies and progress between bots.
- Codync now invites you to rate it on the App Store after your fifth message from this iPhone, at most twice a year.
- The reasoning effort slider now uses a dotted gradient to make levels easier to distinguish.
