# diffdeck

> **⚠️ This project is no longer maintained (development stopped 2026-06-06).**
>
> What exists still works, but there will be no further maintenance. The notes below record
> what it aimed for and why it was shelved.

## Why development stopped

### What it aimed for (the ideal)

The goal was to make three parties — the human, the TUI, and the LLM (Claude Code) — turn as
one smooth local loop:

1. The agent produces a diff.
2. The human leaves line comments in a terminal TUI.
3. The LLM picks those comments up and applies the fixes directly.

The ideal was a review that completes entirely inside the shell — no browser — where the
comments themselves become the instructions to the agent.

### The problems we actually hit

The biggest wall was that **the LLM ↔ TUI integration never worked well.** Against the ideal,
these problems were structural and stuck:

- **The TUI and the LLM cannot share one terminal.**
  - A TUI owns a real tty, so it cannot run inside Claude Code's own pane.
  - That forces it to spawn into a separate window, which is fragile and depends on the host
    environment (tmux / Terminal.app / iTerm2 / Linux GUI).
  - The agent cannot consistently launch, drive, and observe the TUI, so ownership of the loop
    ends up in limbo.

- **The hand-off became a one-way trip through a file.**
  - The LLM cannot see the TUI screen; it can only receive results through
    `.diffdeck/comments.json`.
  - So it never became an interactive, real-time loop — the manual coordination of
    "launch → review → save → apply" remained to the end.
  - It never reached the ideal's smoothness, where a comment is the instruction.

- **It could not out-compete existing web-based reviewers (e.g. difit).**
  - The core differentiator was "starts instantly in the terminal," but with the LLM
    integration staying weak, that advantage was cancelled out.
  - The conclusion: "it stays in the terminal" alone was too weak a reason to switch tools.

In short, **the TUI form and the ideal of an agent driving the whole process were
structurally mismatched.** With no clear way to absorb that in the design, development is
being shelved.

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
