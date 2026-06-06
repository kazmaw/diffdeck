//! install-skill サブコマンドをビルド済みバイナリ経由で検証する。

use std::process::Command;

/// cargo がテスト用に提供するビルド済みバイナリのパス。
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_diffdeck")
}

#[test]
fn print_outputs_embedded_skill() {
    let out = Command::new(bin())
        .args(["install-skill", "--print"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    // 埋め込み本文がリポジトリのソースと一致すること（単一ソース）
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.claude/skills/diffdeck/SKILL.md"
    ))
    .unwrap();
    assert_eq!(body, src);
}

#[test]
fn dir_installs_skill_file() {
    let tmp = tempfile::tempdir().unwrap();
    let status = Command::new(bin())
        .args(["install-skill", "--dir"])
        .arg(tmp.path())
        .status()
        .unwrap();
    assert!(status.success());
    let written = std::fs::read_to_string(tmp.path().join("SKILL.md")).unwrap();
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.claude/skills/diffdeck/SKILL.md"
    ))
    .unwrap();
    assert_eq!(written, src);
}

#[test]
fn unknown_target_exits_nonzero() {
    let out = Command::new(bin())
        .args(["install-skill", "--target", "cursor"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("claude"), "stderr: {err}");
}
