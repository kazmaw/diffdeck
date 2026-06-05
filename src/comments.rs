//! .diffdeck/comments.json の読み書きと gitignore 判定。

use crate::comment::{Comment, CommentFile};
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub const SCHEMA: &str = "diffdeck/v1";

/// repo 直下の .diffdeck/comments.json のパス。
pub fn comments_path(repo: &Path) -> std::path::PathBuf {
    repo.join(".diffdeck").join("comments.json")
}

/// 既存ファイルを読む。無ければ None。
pub fn load(path: &Path) -> Result<Option<CommentFile>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let parsed: CommentFile = serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(parsed))
}

/// 全コメントから CommentFile を組み立てる。
pub fn build(repo: &str, scope: &str, comments: Vec<Comment>) -> CommentFile {
    CommentFile {
        schema: SCHEMA.to_string(),
        repo: repo.to_string(),
        scope: scope.to_string(),
        comments,
    }
}

/// 親ディレクトリを作って整形 JSON を書き出す。
pub fn save(path: &Path, file: &CommentFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(file)?;
    std::fs::write(path, json).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// repo 内の相対パスが gitignore 済みかを判定する（未 ignore 警告用）。
pub fn is_ignored(repo: &Path, rel: &str) -> bool {
    Command::new("git")
        .args(["check-ignore", "-q", rel])
        .current_dir(repo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::{LineRange, Side};

    #[test]
    fn load_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = comments_path(dir.path());
        assert!(load(&path).unwrap().is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = comments_path(dir.path());
        let file = build(
            "/tmp/repo",
            "working",
            vec![Comment::thread("a.rs", Side::New, LineRange::Single(3), "hi")],
        );
        save(&path, &file).unwrap();
        assert!(path.exists());

        let loaded = load(&path).unwrap().unwrap();
        assert_eq!(loaded, file);
        assert_eq!(loaded.schema, "diffdeck/v1");
    }

    #[test]
    fn build_sets_schema_and_meta() {
        let f = build("/r", "staged", vec![]);
        assert_eq!(f.schema, "diffdeck/v1");
        assert_eq!(f.repo, "/r");
        assert_eq!(f.scope, "staged");
        assert!(f.comments.is_empty());
    }
}
