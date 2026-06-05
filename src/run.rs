//! CLI→git→App の配線と保存・警告ヘルパ。

use crate::cli::Cli;
use crate::comments;
use crate::git;
use crate::model::DiffSpec;
use crate::ui::app::App;
use anyhow::{bail, Result};
use std::path::Path;

/// 端末を起動せずに App を構築する。差分ゼロなら Ok(None)。
pub fn build_app(spec: &DiffSpec, repo: &Path) -> Result<Option<App>> {
    if !git::is_git_repo(repo) {
        bail!("not a git repository: {}", repo.display());
    }
    let files = git::load_file_diffs(spec, repo)?;
    if files.is_empty() {
        return Ok(None);
    }
    let path = comments::comments_path(repo);
    let existing = comments::load(&path)?.map(|f| f.comments).unwrap_or_default();
    let repo_str = repo.display().to_string();
    Ok(Some(App::new(files, existing, repo_str, spec.scope.as_str().to_string())))
}

/// 終了時にコメントを保存する。
pub fn persist(app: &App, repo: &Path) -> Result<()> {
    if !app.save_requested {
        return Ok(());
    }
    let path = comments::comments_path(repo);
    let file = comments::build(&app.repo, &app.scope, app.comments.clone());
    comments::save(&path, &file)?;
    Ok(())
}

/// .diffdeck が gitignore されていなければ警告文を返す。
pub fn gitignore_warning(repo: &Path) -> Option<String> {
    if comments::is_ignored(repo, ".diffdeck/") {
        None
    } else {
        Some("warning: .diffdeck/ is not gitignored — add it to .gitignore".to_string())
    }
}

/// Cli から DiffSpec を作るショートカット（main 用）。
pub fn spec_from_cli(cli: &Cli) -> DiffSpec {
    cli.to_spec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scope;
    use std::fs;
    use std::process::Command;

    fn git(repo: &Path, args: &[&str]) {
        let s = Command::new("git")
            .args(args)
            .current_dir(repo)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", repo)
            .status()
            .unwrap();
        assert!(s.success());
    }

    fn working() -> DiffSpec {
        DiffSpec { scope: Scope::Working, target: None, base: None, merge_base: false }
    }

    #[test]
    fn errors_when_not_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let err = build_app(&working(), dir.path()).unwrap_err();
        assert!(err.to_string().contains("not a git repository"));
    }

    #[test]
    fn returns_none_on_empty_diff() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q", "-b", "main"]);
        fs::write(p.join("a.txt"), "x\n").unwrap();
        git(p, &["add", "a.txt"]);
        git(p, &["commit", "-q", "-m", "init"]);
        assert!(build_app(&working(), p).unwrap().is_none());
    }

    #[test]
    fn builds_app_and_persists_comments() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q", "-b", "main"]);
        fs::write(p.join("a.txt"), "one\n").unwrap();
        git(p, &["add", "a.txt"]);
        git(p, &["commit", "-q", "-m", "init"]);
        fs::write(p.join("a.txt"), "one\ntwo\n").unwrap();

        let mut app = build_app(&working(), p).unwrap().unwrap();
        assert!(!app.files.is_empty());

        // コメントを1件追加して保存をリクエスト
        use crate::comment::{Comment, LineRange, Side};
        app.comments.push(Comment::thread("a.txt", Side::New, LineRange::Single(2), "hi"));
        app.save_requested = true;
        persist(&app, p).unwrap();

        let saved = comments::load(&comments::comments_path(p)).unwrap().unwrap();
        assert_eq!(saved.comments.len(), 1);
        assert_eq!(saved.scope, "working");
    }
}
