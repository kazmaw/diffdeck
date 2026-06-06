---
name: diffdeck
description: Launch diffdeck (a terminal diff viewer) in a separate terminal window for the user to review and leave line comments, then apply those comments to the code and archive them. Use when the user says "diffdeck", "/diffdeck", "review with diffdeck", "diffdeck でレビュー", "diffdeck のコメントを反映", or otherwise wants to leave or apply local review comments via diffdeck.
---

# Diffdeck review & apply

diffdeck is a terminal (TUI) diff viewer. The user reviews a diff in it and writes
line/range comments, which diffdeck saves to `.diffdeck/comments.json` (difit-compatible).
This skill has two phases:

1. **Launch** diffdeck in a *separate* terminal window so the user can review.
2. **Apply** the comments they wrote and archive them.

A TUI needs its own tty, so diffdeck must run in a separate window the user controls.
**Never run diffdeck inside your own (the agent's) terminal** — it cannot be operated there.

## Prerequisites

- Must be inside a Git-managed directory.
- Decide the `<diffdeck-cmd>`:
  - If `command -v diffdeck` succeeds, use `diffdeck`.
  - Otherwise, from the project root use `cargo run --release --quiet --` (a release build
    makes startup instant; the first run may compile).
- Comments file: `.diffdeck/comments.json`, schema `diffdeck/v1`. Top-level fields are
  `schema` / `repo` / `scope` / `comments[]`. Each comment has `type` / `filePath` /
  `position` / `body`; `position.side` is `"old"` / `"new"`; `position.line` is a number or
  `{start, end}`. `scope` (e.g. `working` / `staged` / `ref` / `range`) records which diff
  scope the comments were written against — informational; mention it when reporting.

## Phase 1: Launch (skip if comments already exist)

If `.diffdeck/comments.json` already exists and is non-empty, the user has reviewed
manually — **skip to Phase 2**.

Otherwise:

1. **Pick the scope** from the user's request/context (mirrors diffdeck's CLI):
   - no arg → working tree
   - `staged` → staged changes
   - `<ref>` → that ref vs its parent
   - `<target> <base>` (add `--merge-base` for PR-base direction) → range
   If ambiguous, ask with `AskUserQuestion` before launching.

2. **Open a new terminal window** running diffdeck in the repo root, detecting the
   environment (use the first that applies):
   - Inside tmux (`$TMUX` set):
     `tmux new-window -c "$PWD" '<diffdeck-cmd> <scope>'`
   - macOS + Terminal.app:
     `osascript -e 'tell application "Terminal" to do script "cd '"$PWD"' && <diffdeck-cmd> <scope>"'`
   - macOS + iTerm2: use the iTerm2 AppleScript equivalent (`create window with default profile`,
     then `write text`).
   - Linux GUI: `gnome-terminal -- bash -lc 'cd "$PWD" && <diffdeck-cmd> <scope>'`
     (or `x-terminal-emulator`).
   - Headless / no window manager / cannot open a window: print the **exact** command and
     ask the user to run it themselves in another terminal.

3. **Tell the user** (in their language) that diffdeck opened in another window, to review,
   press `w` to save, and to say when they're done.

4. **Wait for the user's signal** that they finished. Do not aggressively poll; simply wait
   for them to say done. You may optionally check whether `.diffdeck/comments.json` appeared.
   If the user shut diffdeck without saving, treat it as "no comments" and stop.

## Phase 2: Apply

1. **Read** `.diffdeck/comments.json`. If missing or empty, tell the user there are no
   comments and stop.

2. **Validate**: compare the JSON `repo` field with `git rev-parse --show-toplevel`; warn the
   user and confirm before proceeding if they differ. If `schema` is not `diffdeck/v1`, or the
   file is malformed, report it and ask for instructions rather than guessing.

3. **Present** each comment, one at a time:
   - `filePath:line` (range as `start-end`)
   - `side` (new = post-change / old = pre-change)
   - the current code at that line (read `filePath` for context)
   - the comment `body`
   - For `side: "new"`, `line` is the new-file line number — read it directly in the working tree.
   - For `side: "old"`, `line` is a pre-change line number that may not match the current file
     (the line may be deleted/moved). Confirm context via `git diff`, or ask the user when
     ambiguous. Do not edit the wrong line.

4. **Apply** the fixes. Comments are observations, not blind orders: when a comment is a
   question or a "is this needed?" doubt, confirm with the user instead of editing.

5. **Archive**: create `.diffdeck/processed/`, then move `comments.json` to
   `.diffdeck/processed/<timestamp>.json` where `<timestamp>` is `date +%Y%m%d-%H%M%S`.
   Do not leave `comments.json` behind (prevents double-application).

6. **Report** the files changed, a change summary (mention the `scope`), and the archive path.

## Notes

- Write comment bodies and your report in the language the user is using.
- Never copy secrets, tokens, passwords, or keys from the diff into command-line arguments.
