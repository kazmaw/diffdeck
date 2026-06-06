# diffdeck

> **⚠️ このプロジェクトは開発を終了しました（2026-06-06）。**
>
> 動くものは残っていますが、これ以上のメンテナンスはしません。以下は「何を目指して、なぜ畳んだか」の記録です。

## 開発をやめた理由

### 目指していたもの（理想）

人間・TUI・LLM（Claude Code）の三者が一つのローカルループとして滑らかに回ることを狙っていました。

1. エージェントが diff を出す
2. 人間がターミナル上の TUI で行コメントを残す
3. LLM がそのコメントを拾って、そのまま修正に反映する

「ブラウザを開かずシェルの中だけでレビューが完結し、コメントがそのままエージェントへの指示になる」——これが理想でした。

### 実際にぶつかった問題

最大の壁は **LLM ↔ TUI の連携がうまくいかなかった** ことです。理想に対して、構造的に次の問題が残りました。

- **TUI と LLM が同じターミナルを共有できない**
  - TUI は実 tty を占有するため、Claude Code のペイン内では動かせません。
  - 結果として別ウィンドウへ spawn するしかなく、起動環境（tmux / Terminal.app / iTerm2 / Linux GUI）依存で壊れやすくなりました。
  - エージェントが「起動・操作・観測」を一貫して担えず、ループの主導権が宙に浮きました。

- **ハンドオフがファイル経由の一方通行になった**
  - LLM は TUI の画面を直接見られず、`.diffdeck/comments.json` を介してしか結果を受け取れません。
  - そのため対話的なリアルタイムループにならず、「起動 → レビュー → 保存 → 反映」という手動コーディネーションが最後まで残りました。
  - 理想の「コメントがそのまま指示になる」滑らかさには届きませんでした。

- **既存の web ベースレビューア（difit など）に対する優位を出しきれなかった**
  - 差別化の核は「ターミナルで即起動」でしたが、肝心の LLM 連携が弱いままで、その利点が相殺されました。
  - 「ターミナルで完結する」だけでは、ツールを乗り換える理由として弱かった、というのが結論です。

総じて、**TUI という形態と、エージェントが全工程を駆動するという理想が構造的に噛み合いませんでした。** ここを設計で吸収する見通しが立たなかったため、開発を畳みます。

---

A fast, in-terminal diff viewer that lets you leave line comments and hands them to
Claude Code as a difit-compatible JSON file.

diffdeck reads `git diff`, renders it in a terminal UI with syntax highlighting, lets you
attach line/range comments, and writes them to `.diffdeck/comments.json`. A companion
`/diffdeck` skill then applies those comments to your code and archives them. It is a single
Rust binary with no runtime services.

## Why

When an AI or CLI changes your code, you often want to skim the diff, jot a few "fix this"
or "why this?" notes on specific lines, and have an agent act on them. Web-based reviewers
work, but a terminal tool starts instantly and stays in your shell. diffdeck keeps the review
loop local: review in the terminal, save comments to a file, let Claude Code pick them up.

## Install

### npx / npm (no Rust needed)

```bash
# run without installing
npx diffdeck

# or install globally
npm i -g diffdeck
```

Prebuilt binaries are shipped as per-OS packages (`@diffdeck/cli-*`) and selected
automatically. Supported: macOS (arm64/x64), Linux (x64), Windows (x64). On an
unsupported platform the launcher tells you to use `cargo install diffdeck` instead.

### cargo (Rust users)

```bash
cargo install diffdeck

# or from a clone of this repo
cargo install --path .
```

### AI skill (Claude Code)

diffdeck ships a companion skill that lets Claude Code launch the reviewer and apply
your comments. Install it explicitly:

```bash
# installs to ~/.claude/skills/diffdeck/SKILL.md
diffdeck install-skill

# preview without writing
diffdeck install-skill --print

# install to a custom directory / overwrite
diffdeck install-skill --dir ./some/dir
diffdeck install-skill --force
```

## Usage

The CLI mirrors difit's argument muscle-memory. The diff scope is chosen by the arguments:

| Command | Scope |
|---|---|
| `diffdeck` | working tree (vs `HEAD`) |
| `diffdeck staged` | staged changes |
| `diffdeck <ref>` | `<ref>` vs its parent |
| `diffdeck <target> <base>` | range, `base → target` |
| `diffdeck <target> <base> --merge-base` | range from the merge base (PR-style) |

diffdeck needs a real terminal (tty). With no changes to show it prints `変更なし` and exits 0.

## Key bindings

| Key | Mode | Action |
|---|---|---|
| `j` / `k` | normal / range | Move the line cursor down / up |
| `J` / `K` | normal | Next / previous file |
| `c` | normal | Comment the current diff line |
| `V` | normal | Start a range selection (anchor at the current line) |
| `c` | range | Comment the selected range |
| `d` | normal | Delete the comment(s) on the current line |
| `w` | normal | Save comments and quit |
| `q` | normal | Quit (asks to confirm if there are unsaved comments) |
| type / `Backspace` | comment | Edit the comment text |
| `Enter` | comment | Save the comment |
| `Esc` | comment / range / confirm | Cancel |
| `y` | confirm-quit | Quit without saving |

The diff pane follows the cursor (viewport scrolling) and only highlights visible lines, so
large diffs stay responsive.

## Output: `.diffdeck/comments.json`

Comments are written to `.diffdeck/comments.json` in a difit-compatible shape
(schema `diffdeck/v1`). diffdeck loads any existing file on start and overwrites it on save,
so the file is always the full set of comments.

```json
{
  "schema": "diffdeck/v1",
  "repo": "/path/to/repo",
  "scope": "working",
  "comments": [
    {
      "type": "thread",
      "filePath": "src/auth.rs",
      "position": { "side": "new", "line": 16 },
      "body": "Is this warning still needed?"
    },
    {
      "type": "thread",
      "filePath": "src/ui.rs",
      "position": { "side": "new", "line": { "start": 36, "end": 39 } },
      "body": "Dead code?"
    }
  ]
}
```

- `position.side`: `"new"` for the post-change side, `"old"` for the deleted side.
- `position.line`: a single line number, or `{ "start", "end" }` for a range.

Add `.diffdeck/` to your `.gitignore`; diffdeck warns on startup if it is not ignored.

## The `/diffdeck` skill

The bundled skill at `.claude/skills/diffdeck/SKILL.md` drives the full loop with Claude Code:

1. **Launch** — opens diffdeck in a separate terminal window (tmux / Terminal.app / iTerm2 /
   Linux GUI; falls back to printing the command) so you can review.
2. **Apply** — after you save, reads `.diffdeck/comments.json`, presents each comment with
   `file:line` context, applies the fixes, and archives the file to
   `.diffdeck/processed/<timestamp>.json` to prevent double-application.

If you ran diffdeck yourself and a `comments.json` already exists, the skill skips the launch
step and goes straight to applying.

## Development

```bash
cargo test          # unit + integration tests
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo build --release
```

The codebase is split into single-responsibility modules: `model` (domain types), `cli`
(arg → `DiffSpec`), `diff_parse` (pure unified-diff parser), `git` (git execution),
`comments` (persistence), `highlight` (syntect), `ui::app` (terminal-independent state
machine), `ui::render` (ratatui drawing), and `run`/`main` (wiring and the terminal loop).

## Releasing (maintainers)

Releases are automated by `.github/workflows/release.yml` on a `v*` tag whose number
matches `Cargo.toml`:

1. Bump `version` in `Cargo.toml`, commit.
2. `git tag vX.Y.Z && git push origin vX.Y.Z`.

CI cross-builds, attaches binaries to the GitHub Release, publishes the npm packages
(main + `@diffdeck/cli-*`), and runs `cargo publish`. Required GitHub secrets:
`NPM_TOKEN` and `CARGO_REGISTRY_TOKEN`.
