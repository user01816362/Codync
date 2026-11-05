<!--
Google Play release notes for the Android app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per Play language (zh-Hant, published as zh-TW;
en-US), listing what Android users notice, each under 500 characters (Play's limit;
tools/play-submit.py cuts longer notes at the last whole line). tools/play-submit.py reads the
released version's section; a version without one gets the PRs' "What's New: Android" bullets,
else a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.39.0
### zh-Hant
- 可以編輯已送出的訊息，讓機器人回答修改後的版本；用訊息下方的箭頭查看前後版本和各自得到的回答。
- 可以清除對話，讓機器人從頭開始；它的記憶、檔案和資料庫都會保留。

### en-US
- Edit a message you sent and the bot answers the changed version; the arrows under it walk through its versions and the answers they got.
- Clear a chat to start the bot's conversation over; its memory, files and Library stay.
