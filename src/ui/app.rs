//! 端末非依存の App 状態機械とキー操作。

use crate::comment::{Comment, LineRange, Side};
use crate::model::{FileDiff, LineKind};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    RangeSelect,
    Comment,
    ConfirmQuit,
}

/// 差分ペインに表示する 1 行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Header(String),
    Diff {
        kind: LineKind,
        old_no: Option<u32>,
        new_no: Option<u32>,
        content: String,
    },
    /// コメント対象行の直下に差し込むコメント本文。非ターゲット行。
    Comment {
        body: String,
    },
}

#[derive(Debug)]
pub struct App {
    pub files: Vec<FileDiff>,
    pub comments: Vec<Comment>,
    pub repo: String,
    pub scope: String,
    pub file_cursor: usize,
    pub line_cursor: usize,
    pub mode: Mode,
    pub input: String,
    pub range_anchor: Option<usize>,
    pub dirty: bool,
    pub should_quit: bool,
    pub save_requested: bool,
    /// 直近フレームの差分ペイン表示行数。ページ移動量の算出に使う。
    pub viewport_h: usize,
}

impl App {
    pub fn new(files: Vec<FileDiff>, comments: Vec<Comment>, repo: String, scope: String) -> Self {
        App {
            files,
            comments,
            repo,
            scope,
            file_cursor: 0,
            line_cursor: 0,
            mode: Mode::Normal,
            input: String::new(),
            range_anchor: None,
            dirty: false,
            should_quit: false,
            save_requested: false,
            viewport_h: 0,
        }
    }

    /// 現在ファイルの表示行を平坦化する。
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        let Some(file) = self.files.get(self.file_cursor) else {
            return rows;
        };
        let path = file.display_path().to_string();
        for hunk in &file.hunks {
            rows.push(Row::Header(format!(
                "@@ -{},{} +{},{} @@ {}",
                hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines, hunk.header
            )));
            for line in &hunk.lines {
                rows.push(Row::Diff {
                    kind: line.kind.clone(),
                    old_no: line.old_no,
                    new_no: line.new_no,
                    content: line.content.clone(),
                });
                // この行が指す (side, line_no) を求め、対象コメントを直下に差し込む。
                let target = match line.kind {
                    LineKind::Removed => line.old_no.map(|n| (Side::Old, n)),
                    _ => line.new_no.map(|n| (Side::New, n)),
                };
                if let Some((side, line_no)) = target {
                    for c in &self.comments {
                        if c.file_path != path || c.position.side != side {
                            continue;
                        }
                        // 単一行はその行、範囲は末尾行の直下に置く。
                        let anchor = match &c.position.line {
                            LineRange::Single(n) => *n,
                            LineRange::Range { end, .. } => *end,
                        };
                        if anchor == line_no {
                            rows.push(Row::Comment {
                                body: c.body.clone(),
                            });
                        }
                    }
                }
            }
        }
        rows
    }

    fn row_count(&self) -> usize {
        self.rows().len()
    }

    /// 行 index から (side, line_no) を導出する。Header 行は None。
    fn target_at(&self, idx: usize) -> Option<(Side, u32)> {
        match self.rows().get(idx)? {
            Row::Header(_) | Row::Comment { .. } => None,
            Row::Diff {
                kind,
                old_no,
                new_no,
                ..
            } => match kind {
                LineKind::Removed => old_no.map(|n| (Side::Old, n)),
                _ => new_no.map(|n| (Side::New, n)),
            },
        }
    }

    fn current_file_path(&self) -> Option<String> {
        self.files
            .get(self.file_cursor)
            .map(|f| f.display_path().to_string())
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Comment => self.on_key_comment(key),
            Mode::ConfirmQuit => self.on_key_confirm(key),
            Mode::Normal | Mode::RangeSelect => self.on_key_nav(key),
        }
    }

    /// 半ページ移動量（最小 1 行）。
    fn half_page(&self) -> usize {
        (self.viewport_h / 2).max(1)
    }

    /// 全ページ移動量（最小 1 行）。
    fn full_page(&self) -> usize {
        self.viewport_h.max(1)
    }

    /// カーソルを delta だけ動かし、0..=末尾にクランプする。
    fn move_cursor_by(&mut self, delta: isize) {
        let max = self.row_count().saturating_sub(1) as isize;
        let next = (self.line_cursor as isize + delta).clamp(0, max);
        self.line_cursor = next as usize;
    }

    fn on_key_nav(&mut self, key: KeyEvent) {
        // Ctrl 系はここで消費する（素の d=削除 と衝突させないため）。
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('d') => self.move_cursor_by(self.half_page() as isize),
                KeyCode::Char('u') => self.move_cursor_by(-(self.half_page() as isize)),
                KeyCode::Char('f') => self.move_cursor_by(self.full_page() as isize),
                KeyCode::Char('b') => self.move_cursor_by(-(self.full_page() as isize)),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::PageDown => self.move_cursor_by(self.full_page() as isize),
            KeyCode::PageUp => self.move_cursor_by(-(self.full_page() as isize)),
            KeyCode::Char('g') => self.line_cursor = 0,
            KeyCode::Char('G') => {
                self.line_cursor = self.row_count().saturating_sub(1);
            }
            KeyCode::Char('j') => {
                let max = self.row_count().saturating_sub(1);
                if self.line_cursor < max {
                    self.line_cursor += 1;
                }
            }
            KeyCode::Char('k') => {
                self.line_cursor = self.line_cursor.saturating_sub(1);
            }
            KeyCode::Char('J') => {
                if self.file_cursor + 1 < self.files.len() {
                    self.file_cursor += 1;
                    self.line_cursor = 0;
                    self.mode = Mode::Normal;
                    self.range_anchor = None;
                }
            }
            KeyCode::Char('K') => {
                if self.file_cursor > 0 {
                    self.file_cursor -= 1;
                    self.line_cursor = 0;
                    self.mode = Mode::Normal;
                    self.range_anchor = None;
                }
            }
            KeyCode::Char('V') => {
                self.mode = Mode::RangeSelect;
                self.range_anchor = Some(self.line_cursor);
            }
            KeyCode::Char('c') => {
                if self.target_at(self.line_cursor).is_some() {
                    self.input.clear();
                    self.mode = Mode::Comment;
                }
            }
            KeyCode::Char('d') => {
                self.delete_comment_at_cursor();
            }
            KeyCode::Char('w') => {
                self.save_requested = true;
                self.should_quit = true;
            }
            KeyCode::Char('q') => {
                if self.dirty {
                    self.mode = Mode::ConfirmQuit;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.range_anchor = None;
            }
            _ => {}
        }
    }

    fn on_key_comment(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => self.input.push(c),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Enter => self.commit_comment(),
            KeyCode::Esc => {
                self.input.clear();
                self.mode = Mode::Normal;
                self.range_anchor = None;
            }
            _ => {}
        }
    }

    fn on_key_confirm(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') => self.should_quit = true,
            _ => self.mode = Mode::Normal,
        }
    }

    fn commit_comment(&mut self) {
        let Some(path) = self.current_file_path() else {
            self.mode = Mode::Normal;
            return;
        };
        let Some((side, cursor_line)) = self.target_at(self.line_cursor) else {
            self.mode = Mode::Normal;
            return;
        };

        let line = if let Some(anchor) = self.range_anchor {
            // anchor〜cursor の対応行番号から範囲を作る
            let mut nums = Vec::new();
            let (lo, hi) = if anchor <= self.line_cursor {
                (anchor, self.line_cursor)
            } else {
                (self.line_cursor, anchor)
            };
            for i in lo..=hi {
                if let Some((s, n)) = self.target_at(i) {
                    if s == side {
                        nums.push(n);
                    }
                }
            }
            match (nums.iter().min().copied(), nums.iter().max().copied()) {
                (Some(start), Some(end)) if start != end => LineRange::Range { start, end },
                _ => LineRange::Single(cursor_line),
            }
        } else {
            LineRange::Single(cursor_line)
        };

        let body = std::mem::take(&mut self.input);
        self.comments.push(Comment::thread(path, side, line, body));
        self.dirty = true;
        self.mode = Mode::Normal;
        self.range_anchor = None;
    }

    fn delete_comment_at_cursor(&mut self) {
        let Some(path) = self.current_file_path() else {
            return;
        };
        let Some((side, line_no)) = self.target_at(self.line_cursor) else {
            return;
        };
        let before = self.comments.len();
        self.comments.retain(|c| {
            if c.file_path != path || c.position.side != side {
                return true;
            }
            let hit = match &c.position.line {
                LineRange::Single(n) => *n == line_no,
                LineRange::Range { start, end } => line_no >= *start && line_no <= *end,
            };
            !hit
        });
        if self.comments.len() != before {
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Hunk, Line};

    fn key(c: char) -> KeyEvent {
        KeyEvent::from(KeyCode::Char(c))
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    /// 1 ファイル・20 行(new_no 1..=20)の App。viewport_h を明示できる。
    fn tall(line_cursor: usize, viewport_h: usize) -> App {
        let lines: Vec<Line> = (0..20)
            .map(|i| Line {
                kind: LineKind::Added,
                old_no: None,
                new_no: Some(i as u32 + 1),
                content: format!("l{i}"),
            })
            .collect();
        let f = FileDiff {
            old_path: Some("f.txt".into()),
            new_path: Some("f.txt".into()),
            is_binary: false,
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 20,
                header: "h".into(),
                lines,
            }],
        };
        let mut a = App::new(vec![f], vec![], "/repo".into(), "working".into());
        a.line_cursor = line_cursor;
        a.viewport_h = viewport_h;
        a
    }

    fn file(path: &str) -> FileDiff {
        FileDiff {
            old_path: Some(path.into()),
            new_path: Some(path.into()),
            is_binary: false,
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 3,
                header: "ctx".into(),
                lines: vec![
                    Line {
                        kind: LineKind::Added,
                        old_no: None,
                        new_no: Some(2),
                        content: "a".into(),
                    },
                    Line {
                        kind: LineKind::Added,
                        old_no: None,
                        new_no: Some(3),
                        content: "b".into(),
                    },
                    Line {
                        kind: LineKind::Added,
                        old_no: None,
                        new_no: Some(4),
                        content: "c".into(),
                    },
                ],
            }],
        }
    }

    fn app() -> App {
        App::new(
            vec![file("src/a.rs"), file("src/b.rs")],
            vec![],
            "/repo".into(),
            "working".into(),
        )
    }

    #[test]
    fn rows_include_header_and_lines() {
        let a = app();
        let rows = a.rows();
        assert!(matches!(rows[0], Row::Header(_)));
        assert_eq!(rows.len(), 4); // 1 header + 3 lines
    }

    #[test]
    fn j_and_k_move_within_bounds() {
        let mut a = app();
        a.on_key(key('j'));
        assert_eq!(a.line_cursor, 1);
        // 末尾でクランプ
        for _ in 0..10 {
            a.on_key(key('j'));
        }
        assert_eq!(a.line_cursor, 3);
        for _ in 0..10 {
            a.on_key(key('k'));
        }
        assert_eq!(a.line_cursor, 0);
    }

    #[test]
    fn shift_j_switches_file_and_resets_line() {
        let mut a = app();
        a.on_key(key('j'));
        a.on_key(key('J'));
        assert_eq!(a.file_cursor, 1);
        assert_eq!(a.line_cursor, 0);
    }

    #[test]
    fn comment_on_added_line_uses_new_side() {
        let mut a = app();
        a.on_key(key('j')); // row 1 = first Added line (new_no 2)
        a.on_key(key('c'));
        assert_eq!(a.mode, Mode::Comment);
        for ch in "fix".chars() {
            a.on_key(key(ch));
        }
        a.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(a.comments.len(), 1);
        let c = &a.comments[0];
        assert_eq!(c.file_path, "src/a.rs");
        assert_eq!(c.position.side, Side::New);
        assert_eq!(c.position.line, LineRange::Single(2));
        assert_eq!(c.body, "fix");
        assert!(a.dirty);
        assert_eq!(a.mode, Mode::Normal);
    }

    #[test]
    fn range_select_then_comment_makes_range() {
        let mut a = app();
        a.on_key(key('j')); // row 1, new_no 2
        a.on_key(key('V')); // anchor at row 1
        a.on_key(key('j')); // row 2, new_no 3
        a.on_key(key('c'));
        for ch in "x".chars() {
            a.on_key(key(ch));
        }
        a.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(
            a.comments[0].position.line,
            LineRange::Range { start: 2, end: 3 }
        );
    }

    #[test]
    fn delete_removes_comment_on_line() {
        let mut a = app();
        a.on_key(key('j'));
        a.on_key(key('c'));
        a.on_key(key('z'));
        a.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(a.comments.len(), 1);
        a.on_key(key('d'));
        assert_eq!(a.comments.len(), 0);
    }

    #[test]
    fn esc_cancels_comment_without_saving() {
        let mut a = app();
        a.on_key(key('j'));
        a.on_key(key('c'));
        a.on_key(key('z'));
        a.on_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(a.mode, Mode::Normal);
        assert!(a.comments.is_empty());
    }

    #[test]
    fn w_requests_save_and_quit() {
        let mut a = app();
        a.on_key(key('w'));
        assert!(a.save_requested);
        assert!(a.should_quit);
    }

    #[test]
    fn q_when_clean_quits_immediately() {
        let mut a = app();
        a.on_key(key('q'));
        assert!(a.should_quit);
    }

    #[test]
    fn q_when_dirty_asks_confirmation() {
        let mut a = app();
        a.on_key(key('j'));
        a.on_key(key('c'));
        a.on_key(key('z'));
        a.on_key(KeyEvent::from(KeyCode::Enter));
        a.on_key(key('q'));
        assert_eq!(a.mode, Mode::ConfirmQuit);
        assert!(!a.should_quit);
        a.on_key(key('y'));
        assert!(a.should_quit);
    }

    #[test]
    fn comment_on_removed_line_uses_old_side() {
        let removed_file = FileDiff {
            old_path: Some("src/x.rs".into()),
            new_path: Some("src/x.rs".into()),
            is_binary: false,
            hunks: vec![Hunk {
                old_start: 5,
                old_lines: 1,
                new_start: 5,
                new_lines: 0,
                header: "h".into(),
                lines: vec![Line {
                    kind: LineKind::Removed,
                    old_no: Some(5),
                    new_no: None,
                    content: "gone".into(),
                }],
            }],
        };
        let mut a = App::new(vec![removed_file], vec![], "/repo".into(), "working".into());
        a.on_key(key('j')); // row 0 = Header, row 1 = the Removed line
        a.on_key(key('c'));
        assert_eq!(a.mode, Mode::Comment);
        for ch in "why".chars() {
            a.on_key(key(ch));
        }
        a.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(a.comments.len(), 1);
        assert_eq!(a.comments[0].position.side, Side::Old);
        assert_eq!(a.comments[0].position.line, LineRange::Single(5));
    }

    #[test]
    fn c_on_header_row_is_noop() {
        let mut a = app();
        // line_cursor starts at 0 = Header row
        a.on_key(key('c'));
        assert_eq!(a.mode, Mode::Normal);
        assert!(a.comments.is_empty());
    }

    // --- B: インラインコメント行 ---

    #[test]
    fn comment_row_inserted_below_single_line() {
        let mut a = app();
        a.on_key(key('j')); // cursor=1 -> new_no 2
        a.on_key(key('c'));
        for ch in "note".chars() {
            a.on_key(key(ch));
        }
        a.on_key(KeyEvent::from(KeyCode::Enter));
        // rows: 0 Header, 1 Diff(2), 2 Comment, 3 Diff(3), 4 Diff(4)
        let rows = a.rows();
        assert_eq!(rows.len(), 5);
        assert!(matches!(&rows[2], Row::Comment { body } if body == "note"));
    }

    #[test]
    fn range_comment_inserted_below_end_line() {
        let mut a = app();
        a.on_key(key('j')); // new_no 2
        a.on_key(key('V')); // anchor
        a.on_key(key('j')); // new_no 3
        a.on_key(key('c'));
        a.on_key(key('r'));
        a.on_key(KeyEvent::from(KeyCode::Enter));
        // 範囲 2..3 は末尾行(new_no 3)の直下に置く。
        // rows: 0 H, 1 D(2), 2 D(3), 3 Comment, 4 D(4)
        let rows = a.rows();
        assert!(matches!(rows[3], Row::Comment { .. }));
    }

    #[test]
    fn comment_row_is_not_a_target() {
        let mut a = app();
        a.on_key(key('j'));
        a.on_key(key('c'));
        a.on_key(key('x'));
        a.on_key(KeyEvent::from(KeyCode::Enter));
        // コメント行(index 2)へカーソルを置く。
        a.line_cursor = 2;
        a.on_key(key('c')); // 非ターゲットなので no-op
        assert_eq!(a.mode, Mode::Normal);
        a.on_key(key('d')); // 非ターゲットなので削除されない
        assert_eq!(a.comments.len(), 1);
    }

    // --- C: 移動キー フルセット ---

    #[test]
    fn ctrl_d_u_move_half_page() {
        let mut a = tall(0, 10); // half = 5
        a.on_key(ctrl('d'));
        assert_eq!(a.line_cursor, 5);
        a.on_key(ctrl('d'));
        assert_eq!(a.line_cursor, 10);
        a.on_key(ctrl('u'));
        assert_eq!(a.line_cursor, 5);
    }

    #[test]
    fn ctrl_f_b_move_full_page() {
        let mut a = tall(0, 8); // full = 8
        a.on_key(ctrl('f'));
        assert_eq!(a.line_cursor, 8);
        a.on_key(ctrl('b'));
        assert_eq!(a.line_cursor, 0);
    }

    #[test]
    fn page_keys_move_full_page() {
        let mut a = tall(0, 6);
        a.on_key(KeyEvent::from(KeyCode::PageDown));
        assert_eq!(a.line_cursor, 6);
        a.on_key(KeyEvent::from(KeyCode::PageUp));
        assert_eq!(a.line_cursor, 0);
    }

    #[test]
    fn g_and_shift_g_jump_to_ends() {
        let mut a = tall(5, 10);
        let last = a.rows().len() - 1;
        a.on_key(key('G'));
        assert_eq!(a.line_cursor, last);
        a.on_key(key('g'));
        assert_eq!(a.line_cursor, 0);
    }

    #[test]
    fn movement_clamps_at_bounds() {
        let mut a = tall(0, 10);
        let last = a.rows().len() - 1;
        for _ in 0..10 {
            a.on_key(ctrl('d'));
        }
        assert_eq!(a.line_cursor, last);
        for _ in 0..10 {
            a.on_key(ctrl('u'));
        }
        assert_eq!(a.line_cursor, 0);
    }

    #[test]
    fn ctrl_d_is_page_move_not_delete() {
        // Ctrl+D はページ移動であって、素の d(削除)ではない。
        let mut a = app();
        a.on_key(key('j'));
        a.on_key(key('c'));
        a.on_key(key('x'));
        a.on_key(KeyEvent::from(KeyCode::Enter));
        a.viewport_h = 4;
        a.line_cursor = 1;
        a.on_key(ctrl('d'));
        assert_eq!(a.comments.len(), 1);
    }
}
