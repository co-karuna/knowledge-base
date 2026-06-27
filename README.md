# knowledge base
会社の知識をため込んでいくためのリポジトリ

## 部門
* finance : 財務
* accounting : 会計
* sales : 営業
* marketing : マーケティング
* human : 人事
* legal : 法務
* study : 研究

## プロジェクト
projects直下にプロジェクト名のフォルダを作成し、その中にファイルを作成する。

## 用語
metrics : 指標
runbooks : 運用手順書
policies : ポリシー
storategy : 戦略
tables : テーブルのスキーマ(SQLを書かせるため)
issues : 課題

row : 生データ
enriched : Markdown加工データ。ADRや要約ドキュメントなど

### メダリオン・アーキテクチャ
メダリオン・アーキテクチャは、メダリオンの知識を管理するためのアーキテクチャです。
生データ→クレンジング・加工したMarkdownファイル→実運用ドキュメント
のレイヤーごとにナレッジを保存する

## Open Knowledge Format
https://github.com/GoogleCloudPlatform/knowledge-catalog
type は必須。それ以外は任意

### example
```
---
type: runbook
title: 経費精算の手続きルール
description: 毎月の経費精算の提出期限と、レシート提出時の注意点についてのガイドライン。
resource: "https://github.com/your-org/knowledge-base/accounting/runbooks/expense-reimbursement.md"
tags: [accounting, expense, gold]
timestamp: 2026-06-25T10:00:00Z
---
```

なぜ仕様上 resource が存在するのか？（役割）
OKFファイルが「Gitリポジトリ」から離れて、別のシステム（例：Google CloudのKnowledge Catalogや、AIのベクトルデータベースなど）に読み込まれた（インポートされた）ときに、そのファイルの「実体がどこにあるか」を指し示す迷子札になるからです。
