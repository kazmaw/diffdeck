//! ファイル一覧・差分ペイン・ステータスバー・モーダルの描画。

use crate::highlight::Highlighter;
use crate::ui::app::{App, Mode, Row};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line as TextLine, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &App, hl: &Highlighter) {
    let area = frame.area();
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    draw_file_list(frame, app, cols[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(cols[1]);

    draw_diff(frame, app, hl, right[0]);
    draw_status(frame, app, right[1]);

    if app.mode == Mode::Comment {
        draw_modal(frame, "comment (Enter=save, Esc=cancel)", &app.input, area);
    } else if app.mode == Mode::ConfirmQuit {
        draw_modal(frame, "unsaved changes — quit? (y/n)", "", area);
    }
}

fn draw_file_list(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<TextLine> = Vec::new();
    for (i, f) in app.files.iter().enumerate() {
        let marker = if i == app.file_cursor { ">" } else { " " };
        let count = app
            .comments
            .iter()
            .filter(|c| c.file_path == f.display_path())
            .count();
        let text = format!(
            "{marker} {} +{} -{} ({})",
            f.display_path(),
            f.added_count(),
            f.removed_count(),
            count
        );
        let style = if i == app.file_cursor {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(TextLine::styled(text, style));
    }
    let block = Block::default().borders(Borders::ALL).title("files");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_diff(frame: &mut Frame, app: &App, hl: &Highlighter, area: Rect) {
    let ext = app
        .files
        .get(app.file_cursor)
        .and_then(|f| f.extension().map(String::from));
    let rows = app.rows();
    let mut lines: Vec<TextLine> = Vec::new();

    // Guard against tiny terminals: bordered block consumes 2 rows.
    let inner_h = (area.height.saturating_sub(2)) as usize;

    if rows.is_empty() {
        // Binary placeholder — independent of slicing logic.
        if app
            .files
            .get(app.file_cursor)
            .map(|f| f.is_binary)
            .unwrap_or(false)
        {
            lines.push(TextLine::from("binary file"));
        }
    } else if inner_h > 0 {
        let total = rows.len();

        // Compute scroll offset: keep line_cursor within [offset, offset+inner_h).
        let offset = if app.line_cursor < inner_h {
            0
        } else {
            (app.line_cursor + 1)
                .saturating_sub(inner_h)
                .min(total.saturating_sub(inner_h))
        };

        // Render only the visible slice; use global index for "selected" check.
        let visible_end = (offset + inner_h).min(total);
        for (local_i, row) in rows[offset..visible_end].iter().enumerate() {
            let global_i = offset + local_i;
            let selected = global_i == app.line_cursor;
            match row {
                Row::Header(text) => {
                    lines.push(TextLine::styled(
                        text.clone(),
                        Style::default().fg(Color::Cyan),
                    ));
                }
                Row::Diff { kind, content, .. } => {
                    let (sign, base) = match kind {
                        crate::model::LineKind::Added => ("+", Color::Green),
                        crate::model::LineKind::Removed => ("-", Color::Red),
                        crate::model::LineKind::Context => (" ", Color::Gray),
                    };
                    let mut spans = vec![Span::styled(sign.to_string(), Style::default().fg(base))];
                    for piece in hl.highlight_line(content, ext.as_deref()) {
                        spans.push(Span::styled(
                            piece.text,
                            Style::default().fg(Color::Rgb(piece.rgb.0, piece.rgb.1, piece.rgb.2)),
                        ));
                    }
                    let mut line = TextLine::from(spans);
                    if selected {
                        line = line.style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                    lines.push(line);
                }
            }
        }
    }
    // inner_h == 0: lines stays empty → renders blank paragraph safely.

    let block = Block::default().borders(Borders::ALL).title("diff");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!(
        " {} | {} | comments: {} | j/k move J/K file c comment V range d del w save q quit",
        app.scope,
        app.repo,
        app.comments.len()
    );
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::Black).bg(Color::White)),
        area,
    );
}

fn draw_modal(frame: &mut Frame, title: &str, body: &str, area: Rect) {
    let w = area.width.saturating_mul(6) / 10;
    let h = 3;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let modal = Rect {
        x,
        y,
        width: w,
        height: h,
    };
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string());
    frame.render_widget(Paragraph::new(body.to_string()).block(block), modal);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::{Comment, LineRange, Side};
    use crate::model::{FileDiff, Hunk, Line, LineKind};
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    fn buffer_text(buf: &Buffer) -> String {
        let area = buf.area;
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn sample_app() -> App {
        let f = FileDiff {
            old_path: Some("src/a.rs".into()),
            new_path: Some("src/a.rs".into()),
            is_binary: false,
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 2,
                header: "fn main".into(),
                lines: vec![
                    Line {
                        kind: LineKind::Context,
                        old_no: Some(1),
                        new_no: Some(1),
                        content: "let a = 1;".into(),
                    },
                    Line {
                        kind: LineKind::Added,
                        old_no: None,
                        new_no: Some(2),
                        content: "let b = 2;".into(),
                    },
                ],
            }],
        };
        App::new(
            vec![f],
            vec![Comment::thread(
                "src/a.rs",
                Side::New,
                LineRange::Single(2),
                "hi",
            )],
            "/repo".into(),
            "working".into(),
        )
    }

    fn render(app: &App) -> String {
        let hl = Highlighter::new();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app, &hl)).unwrap();
        buffer_text(terminal.backend().buffer())
    }

    #[test]
    fn renders_file_path_and_counts() {
        let out = render(&sample_app());
        assert!(out.contains("src/a.rs"), "file path missing:\n{out}");
        assert!(out.contains("+1"), "added count missing:\n{out}");
    }

    #[test]
    fn renders_diff_content() {
        let out = render(&sample_app());
        assert!(out.contains("let b = 2;"), "diff line missing:\n{out}");
    }

    #[test]
    fn renders_status_bar() {
        let out = render(&sample_app());
        assert!(out.contains("working"), "scope missing:\n{out}");
        assert!(out.contains("comments: 1"), "comment count missing:\n{out}");
    }

    #[test]
    fn renders_comment_modal_when_in_comment_mode() {
        let mut app = sample_app();
        app.mode = Mode::Comment;
        app.input = "typing".into();
        let out = render(&app);
        assert!(out.contains("typing"), "modal input missing:\n{out}");
    }

    #[test]
    fn renders_binary_placeholder() {
        let bin = FileDiff {
            old_path: Some("img.png".into()),
            new_path: Some("img.png".into()),
            is_binary: true,
            hunks: vec![],
        };
        let app = App::new(vec![bin], vec![], "/repo".into(), "working".into());
        let hl = Highlighter::new();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &app, &hl)).unwrap();
        let out = buffer_text(terminal.backend().buffer());
        assert!(
            out.contains("binary file"),
            "binary placeholder missing:\n{out}"
        );
    }

    #[test]
    fn renders_confirm_quit_modal() {
        let mut app = sample_app();
        app.mode = Mode::ConfirmQuit;
        let out = render(&app);
        assert!(out.contains("quit"), "confirm-quit modal missing:\n{out}");
    }

    /// Build an App with a single file containing 60 added lines.
    /// Each line has unique content `"ZZZiZZZ"` so `contains` checks are unambiguous.
    /// Uses a `.txt` extension so syntect does not reflow content.
    fn tall_app(line_cursor: usize) -> App {
        use crate::model::{FileDiff, Hunk, Line, LineKind};
        let lines: Vec<Line> = (0..60)
            .map(|i| Line {
                kind: LineKind::Added,
                old_no: None,
                new_no: Some(i as u32 + 1),
                content: format!("ZZZ{i}ZZZ"),
            })
            .collect();
        let f = FileDiff {
            old_path: Some("file.txt".into()),
            new_path: Some("file.txt".into()),
            is_binary: false,
            hunks: vec![Hunk {
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 60,
                header: "tall".into(),
                lines,
            }],
        };
        let mut app = App::new(vec![f], vec![], "/repo".into(), "working".into());
        app.line_cursor = line_cursor;
        app
    }

    /// Render using a small terminal (100×10 → inner_h ≈ 8).
    fn render_small(app: &App) -> String {
        let hl = Highlighter::new();
        // Width 120 avoids wrapping of unique tokens; height 10 → inner_h = 8.
        let backend = TestBackend::new(120, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app, &hl)).unwrap();
        buffer_text(terminal.backend().buffer())
    }

    /// With cursor at the top, the viewport should show early lines and hide far-down lines.
    #[test]
    fn cursor_at_top_shows_first_lines() {
        // rows: index 0 = Header, index 1..=60 = lines ZZZ0ZZZ..ZZZ59ZZZ
        // inner_h = 8; offset = 0 (cursor=0 < 8); visible rows 0..8.
        // Row 0 is the Header, row 1 is ZZZ0ZZZ. Row 50 (ZZZ49ZZZ) is outside.
        let app = tall_app(0);
        let out = render_small(&app);
        assert!(
            out.contains("ZZZ0ZZZ"),
            "first line should be visible at top:\n{out}"
        );
        assert!(
            !out.contains("ZZZ49ZZZ"),
            "line 50 should not be visible when cursor is at top:\n{out}"
        );
    }

    /// With cursor near the bottom, the viewport should scroll so the cursor line is visible.
    #[test]
    fn cursor_near_bottom_scrolls_into_view() {
        // line_cursor = 55 → global row 55 = ZZZ54ZZZ (row 0 is Header).
        // total = 61; inner_h = 8.
        // offset = min(55+1-8, 61-8) = min(48, 53) = 48.
        // visible rows 48..56 → global indices 48..56 → ZZZ47ZZZ..ZZZ55ZZZ visible.
        // ZZZ0ZZZ (global row 1) is well below offset=48, so not visible.
        let app = tall_app(55);
        let out = render_small(&app);
        assert!(
            out.contains("ZZZ54ZZZ"),
            "cursor line should be visible after scrolling:\n{out}"
        );
        assert!(
            !out.contains("ZZZ0ZZZ"),
            "first line should not be visible when scrolled near bottom:\n{out}"
        );
    }
}
