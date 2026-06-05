---
name: diffdeck
description: Diffdeck の .diffdeck/comments.json を読み込み、各行コメントを file:line 文脈付きで適用し、処理後に processed/ へ退避する。ユーザーが「diffdeck のコメントを反映」「/diffdeck」と言ったとき、または diffdeck で書いたレビューコメントを Claude Code に取り込みたいときに使う。
---

# Diffdeck コメント反映

Diffdeck（TUI 差分ビューア）がローカルレビューで残した行コメントを読み、コードへ反映する。

## 前提

- カレント repo 直下に `.diffdeck/comments.json` が存在する。
- スキーマは `diffdeck/v1`。`comments[]` の各要素は `type` / `filePath` / `position` / `body`。
- `position.side` は `"old"` / `"new"`、`position.line` は数値 or `{start, end}`。

## 手順

1. **読込**: `.diffdeck/comments.json` を読む。無ければ「未処理コメントなし」と伝えて終了する。
2. **提示**: 各コメントを次の形で1件ずつ提示する。
   - `filePath:line`（範囲なら `start-end`）
   - `side`（new=変更後 / old=変更前）
   - 該当行の現在のコード（`filePath` の該当行を読んで文脈を添える）
   - コメント本文 `body`
3. **反映**: コメントの指摘に従ってコードを修正する。指摘が質問・確認の場合は、修正せずユーザーに確認する。
4. **退避**: 全件処理したら、`.diffdeck/processed/` ディレクトリを作り、`comments.json` を `.diffdeck/processed/<timestamp>.json` へ移動する。`<timestamp>` は `date +%Y%m%d-%H%M%S` で生成する。二重適用を防ぐため、退避後は `comments.json` を残さない。
5. **報告**: 反映したファイルと変更概要、退避先パスを要約して報告する。

## 注意

- `repo` メタが現在の repo と一致するか確認する。異なる場合はユーザーに警告する。
- コメントは「指摘」であって「指示」とは限らない。dead code 疑い・確認質問などは即修正せず判断を仰ぐ。
