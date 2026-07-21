---
type: issues
title: 課題
description: 課題集
resource: "https://github.com/co-karuna/knowledge-base/blob/main/issues.md"
tags: [issues]
timestamp: 2026-06-29T08:04:55Z
---
# issues

* 時系列の追跡はファイルヘッダのtimestampだけで十分か？
  * ナレッジのテキスト情報をDBに入れて、ヘッダは別カラムで持たせるとソートしやすいのでは？
* リンク切れにどう対応するか？
* 手動と自動を区別する判断基準は？
  * index.md のような重要なファイルは自動で上書きされてもいいのか？
* ネストの深さは上限を設けるべきか？
  * 関係性は深さではなく、リンクで表現するべきなので、深くしないようにする。が優勢
* 権限管理はどうするか？
  * githubのCODEOWNERSで管理する
