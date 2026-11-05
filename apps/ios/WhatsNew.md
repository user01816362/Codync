<!--
App Store "What's New" for the iPhone app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per App Store language (zh-Hant, en-US),
listing what iPhone users notice, without emoji (the App Store rejects them; asc-submit.py drops
any left). tools/asc-submit.py reads the released version's section;
a version without one gets a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.39.0
### zh-Hant
- 可以編輯已送出的訊息，讓機器人回答修改後的版本；用訊息下方的箭頭查看前後版本和各自得到的回答。
- 可以清除對話，讓機器人從頭開始；它的記憶、檔案和資料庫都會保留。

### en-US
- Edit a message you sent and the bot answers the changed version; the arrows under it walk through its versions and the answers they got.
- Clear a chat to start the bot's conversation over; its memory, files and Library stay.
