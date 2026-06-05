//! unified diff テキストを FileDiff へパースする。

use crate::model::{FileDiff, Hunk, Line, LineKind};

/// `git diff` の出力テキストを構造化モデルへパースする。
pub fn parse_diff(text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut cur: Option<FileDiff> = None;
    let mut old_no: u32 = 0;
    let mut new_no: u32 = 0;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some(f) = cur.take() {
                files.push(f);
            }
            let (a, b) = parse_diff_git_paths(rest);
            cur = Some(FileDiff {
                old_path: a,
                new_path: b,
                is_binary: false,
                hunks: Vec::new(),
            });
        } else if let Some(rest) = line.strip_prefix("--- ") {
            if let Some(f) = cur.as_mut() {
                f.old_path = strip_diff_path(rest);
            }
        } else if let Some(rest) = line.strip_prefix("+++ ") {
            if let Some(f) = cur.as_mut() {
                f.new_path = strip_diff_path(rest);
            }
        } else if let Some(rest) = line.strip_prefix("rename from ") {
            if let Some(f) = cur.as_mut() {
                f.old_path = Some(rest.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("rename to ") {
            if let Some(f) = cur.as_mut() {
                f.new_path = Some(rest.to_string());
            }
        } else if line.starts_with("Binary files ") {
            if let Some(f) = cur.as_mut() {
                f.is_binary = true;
            }
        } else if line.starts_with("@@") {
            if let Some(f) = cur.as_mut() {
                let (os, ol, ns, nl, header) = parse_hunk_header(line);
                old_no = os;
                new_no = ns;
                f.hunks.push(Hunk {
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    header,
                    lines: Vec::new(),
                });
            }
        } else if let Some(f) = cur.as_mut() {
            if f.hunks.is_empty() {
                continue; // ハンク前のメタ行（index など）は無視
            }
            let hunk = f.hunks.last_mut().unwrap();
            if let Some(content) = line.strip_prefix('+') {
                hunk.lines.push(Line { kind: LineKind::Added, old_no: None, new_no: Some(new_no), content: content.to_string() });
                new_no += 1;
            } else if let Some(content) = line.strip_prefix('-') {
                hunk.lines.push(Line { kind: LineKind::Removed, old_no: Some(old_no), new_no: None, content: content.to_string() });
                old_no += 1;
            } else if let Some(content) = line.strip_prefix(' ') {
                hunk.lines.push(Line { kind: LineKind::Context, old_no: Some(old_no), new_no: Some(new_no), content: content.to_string() });
                old_no += 1;
                new_no += 1;
            } else if line.starts_with('\\') {
                // "\ No newline at end of file" は無視
            }
        }
    }
    if let Some(f) = cur.take() {
        files.push(f);
    }
    files
}

/// "a/X b/Y" 形式から (old, new) を推定する。--- / +++ が無いバイナリ用フォールバック。
fn parse_diff_git_paths(rest: &str) -> (Option<String>, Option<String>) {
    if let Some((a, b)) = rest.split_once(" b/") {
        let a = a.strip_prefix("a/").unwrap_or(a).to_string();
        (Some(a), Some(b.to_string()))
    } else {
        (None, None)
    }
}

/// "a/path" / "b/path" / "/dev/null" を解決する。
fn strip_diff_path(s: &str) -> Option<String> {
    let s = s.trim();
    if s == "/dev/null" {
        return None;
    }
    let s = s.strip_prefix("a/").or_else(|| s.strip_prefix("b/")).unwrap_or(s);
    Some(s.to_string())
}

/// "@@ -os,ol +ns,nl @@ header" を解析する。行数省略時は 1。
fn parse_hunk_header(line: &str) -> (u32, u32, u32, u32, String) {
    let body = line.strip_prefix("@@ ").unwrap_or(line);
    let end = body.find(" @@").unwrap_or(body.len());
    let nums = &body[..end];
    let header = body[end..].trim_start_matches(" @@").trim().to_string();

    let mut it = nums.split_whitespace();
    let (os, ol) = parse_range(it.next().unwrap_or("-0"));
    let (ns, nl) = parse_range(it.next().unwrap_or("+0"));
    (os, ol, ns, nl, header)
}

/// "-10,3" / "+5" を (start, lines) に。符号は無視、count 省略時 1。
fn parse_range(token: &str) -> (u32, u32) {
    let t = token.trim_start_matches(['-', '+']);
    match t.split_once(',') {
        Some((a, b)) => (a.parse().unwrap_or(0), b.parse().unwrap_or(0)),
        None => (t.parse().unwrap_or(0), 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_modification() {
        let input = "\
diff --git a/src/auth.rs b/src/auth.rs
index 1111111..2222222 100644
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -10,3 +10,4 @@ fn login() {
 ctx line
-removed line
+added one
+added two
";
        let files = parse_diff(input);
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.old_path.as_deref(), Some("src/auth.rs"));
        assert_eq!(f.new_path.as_deref(), Some("src/auth.rs"));
        assert!(!f.is_binary);
        assert_eq!(f.hunks.len(), 1);

        let h = &f.hunks[0];
        assert_eq!((h.old_start, h.old_lines, h.new_start, h.new_lines), (10, 3, 10, 4));
        assert_eq!(h.lines, vec![
            Line { kind: LineKind::Context, old_no: Some(10), new_no: Some(10), content: "ctx line".into() },
            Line { kind: LineKind::Removed, old_no: Some(11), new_no: None, content: "removed line".into() },
            Line { kind: LineKind::Added, old_no: None, new_no: Some(11), content: "added one".into() },
            Line { kind: LineKind::Added, old_no: None, new_no: Some(12), content: "added two".into() },
        ]);
    }

    #[test]
    fn parses_added_file() {
        let input = "\
diff --git a/new.txt b/new.txt
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
+first
+second
";
        let files = parse_diff(input);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].old_path, None);
        assert_eq!(files[0].new_path.as_deref(), Some("new.txt"));
        assert_eq!(files[0].hunks[0].lines.len(), 2);
        assert_eq!(files[0].hunks[0].lines[0].new_no, Some(1));
    }

    #[test]
    fn parses_deleted_file() {
        let input = "\
diff --git a/old.txt b/old.txt
deleted file mode 100644
index 4444444..0000000
--- a/old.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-gone one
-gone two
";
        let files = parse_diff(input);
        assert_eq!(files[0].old_path.as_deref(), Some("old.txt"));
        assert_eq!(files[0].new_path, None);
        assert_eq!(files[0].hunks[0].lines[0].kind, LineKind::Removed);
        assert_eq!(files[0].hunks[0].lines[0].old_no, Some(1));
    }

    #[test]
    fn parses_binary_file() {
        let input = "\
diff --git a/img.png b/img.png
index 5555555..6666666 100644
Binary files a/img.png and b/img.png differ
";
        let files = parse_diff(input);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_binary);
        assert_eq!(files[0].new_path.as_deref(), Some("img.png"));
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn parses_rename() {
        let input = "\
diff --git a/old_name.rs b/new_name.rs
similarity index 95%
rename from old_name.rs
rename to new_name.rs
index 7777777..8888888 100644
--- a/old_name.rs
+++ b/new_name.rs
@@ -1,1 +1,1 @@
-fn a() {}
+fn b() {}
";
        let files = parse_diff(input);
        assert_eq!(files[0].old_path.as_deref(), Some("old_name.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("new_name.rs"));
    }

    #[test]
    fn parses_multiple_files_and_hunks() {
        let input = "\
diff --git a/a.rs b/a.rs
index 1..2 100644
--- a/a.rs
+++ b/a.rs
@@ -1,1 +1,1 @@
-x
+y
diff --git a/b.rs b/b.rs
index 3..4 100644
--- a/b.rs
+++ b/b.rs
@@ -1,1 +1,1 @@
-p
+q
@@ -10,1 +10,1 @@
-m
+n
";
        let files = parse_diff(input);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].new_path.as_deref(), Some("a.rs"));
        assert_eq!(files[1].new_path.as_deref(), Some("b.rs"));
        assert_eq!(files[1].hunks.len(), 2);
        assert_eq!(files[1].hunks[1].new_start, 10);
    }

    #[test]
    fn handles_single_count_omitted() {
        // "@@ -5 +5 @@" のように行数が省略された場合は 1 とみなす
        let input = "\
diff --git a/c.rs b/c.rs
index 1..2 100644
--- a/c.rs
+++ b/c.rs
@@ -5 +5 @@
-a
+b
";
        let files = parse_diff(input);
        let h = &files[0].hunks[0];
        assert_eq!((h.old_start, h.old_lines, h.new_start, h.new_lines), (5, 1, 5, 1));
    }
}
