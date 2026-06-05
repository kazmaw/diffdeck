#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub is_binary: bool,
    pub hunks: Vec<Hunk>,
}

impl FileDiff {
    /// 表示用パス。new 優先、なければ old、どちらも無ければ "(unknown)"。
    pub fn display_path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .unwrap_or("(unknown)")
    }

    /// 追加行数（kind == Added）。
    pub fn added_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == LineKind::Added)
            .count()
    }

    /// 削除行数（kind == Removed）。
    pub fn removed_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == LineKind::Removed)
            .count()
    }

    /// 表示パスの拡張子（"rs" など）。なければ None。
    pub fn extension(&self) -> Option<&str> {
        let path = self.display_path();
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Working,
    Staged,
    Ref,
    Range,
}

impl Scope {
    /// CommentFile.scope に書く文字列表現。
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::Working => "working",
            Scope::Staged => "staged",
            Scope::Ref => "ref",
            Scope::Range => "range",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSpec {
    pub scope: Scope,
    pub target: Option<String>,
    pub base: Option<String>,
    pub merge_base: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_file() -> FileDiff {
        FileDiff {
            old_path: Some("src/auth.rs".into()),
            new_path: Some("src/auth.rs".into()),
            is_binary: false,
            hunks: vec![Hunk {
                old_start: 10,
                old_lines: 2,
                new_start: 10,
                new_lines: 3,
                header: "fn login".into(),
                lines: vec![
                    Line {
                        kind: LineKind::Context,
                        old_no: Some(10),
                        new_no: Some(10),
                        content: "ctx".into(),
                    },
                    Line {
                        kind: LineKind::Removed,
                        old_no: Some(11),
                        new_no: None,
                        content: "old".into(),
                    },
                    Line {
                        kind: LineKind::Added,
                        old_no: None,
                        new_no: Some(11),
                        content: "new1".into(),
                    },
                    Line {
                        kind: LineKind::Added,
                        old_no: None,
                        new_no: Some(12),
                        content: "new2".into(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn display_path_prefers_new() {
        assert_eq!(sample_file().display_path(), "src/auth.rs");
    }

    #[test]
    fn display_path_uses_old_when_deleted() {
        let f = FileDiff {
            new_path: None,
            ..sample_file()
        };
        assert_eq!(f.display_path(), "src/auth.rs");
    }

    #[test]
    fn display_path_unknown_when_both_none() {
        let f = FileDiff {
            old_path: None,
            new_path: None,
            ..sample_file()
        };
        assert_eq!(f.display_path(), "(unknown)");
    }

    #[test]
    fn counts_added_and_removed() {
        let f = sample_file();
        assert_eq!(f.added_count(), 2);
        assert_eq!(f.removed_count(), 1);
    }

    #[test]
    fn extension_of_path() {
        assert_eq!(sample_file().extension(), Some("rs"));
    }

    #[test]
    fn scope_as_str() {
        assert_eq!(Scope::Working.as_str(), "working");
        assert_eq!(Scope::Staged.as_str(), "staged");
        assert_eq!(Scope::Ref.as_str(), "ref");
        assert_eq!(Scope::Range.as_str(), "range");
    }
}
