use diffdeck::git::{is_git_repo, load_file_diffs};
use diffdeck::model::{DiffSpec, Scope};
use std::fs;
use std::path::Path;
use std::process::Command;

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
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
    assert!(status.success(), "git {:?} failed", args);
}

fn working_spec() -> DiffSpec {
    DiffSpec { scope: Scope::Working, target: None, base: None, merge_base: false }
}

#[test]
fn detects_git_repo() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!is_git_repo(dir.path()));
    git(dir.path(), &["init", "-q", "-b", "main"]);
    assert!(is_git_repo(dir.path()));
}

#[test]
fn loads_working_tree_diff_through_real_git() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"]);
    fs::write(p.join("a.txt"), "one\ntwo\n").unwrap();
    git(p, &["add", "a.txt"]);
    git(p, &["commit", "-q", "-m", "init"]);

    // 未コミットの変更を作る
    fs::write(p.join("a.txt"), "one\ntwo changed\nthree\n").unwrap();

    let files = load_file_diffs(&working_spec(), p).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].new_path.as_deref(), Some("a.txt"));
    assert!(files[0].added_count() >= 1);
}

#[test]
fn empty_diff_yields_no_files() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"]);
    fs::write(p.join("a.txt"), "x\n").unwrap();
    git(p, &["add", "a.txt"]);
    git(p, &["commit", "-q", "-m", "init"]);

    let files = load_file_diffs(&working_spec(), p).unwrap();
    assert!(files.is_empty());
}
