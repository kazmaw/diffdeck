//! `install-skill` サブコマンドの実装。skill 本文の埋め込みと配置。

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};

/// バイナリに埋め込む skill 本文。一次情報はリポジトリの SKILL.md（二重管理を避ける）。
pub const SKILL_BODY: &str = include_str!("../.claude/skills/diffdeck/SKILL.md");

/// サポートする配置ターゲット一覧（エラーメッセージにも使う）。
pub const SUPPORTED_TARGETS: &[&str] = &["claude"];

/// 配置先ディレクトリを解決する純粋関数（I/O なし）。
///
/// `dir` 指定があれば最優先。なければ `target` から導出する。
/// `home` はホームディレクトリ（テスト時に差し替え可能）。
pub fn resolve_skill_dir(target: &str, dir: Option<&Path>, home: &Path) -> Result<PathBuf> {
    if let Some(d) = dir {
        return Ok(d.to_path_buf());
    }
    match target {
        "claude" => Ok(home.join(".claude").join("skills").join("diffdeck")),
        other => bail!(
            "unknown --target '{}'. supported targets: {}",
            other,
            SUPPORTED_TARGETS.join(", ")
        ),
    }
}

/// ホームディレクトリを環境変数から得る（新規 dep を増やさない）。
pub fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("could not determine home directory (HOME / USERPROFILE unset)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn claude_target_resolves_under_home() {
        let home = Path::new("/home/u");
        let dir = resolve_skill_dir("claude", None, home).unwrap();
        assert_eq!(dir, Path::new("/home/u/.claude/skills/diffdeck"));
    }

    #[test]
    fn explicit_dir_overrides_target() {
        let home = Path::new("/home/u");
        let dir = resolve_skill_dir("claude", Some(Path::new("/tmp/x")), home).unwrap();
        assert_eq!(dir, Path::new("/tmp/x"));
    }

    #[test]
    fn unknown_target_errors_with_supported_list() {
        let home = Path::new("/home/u");
        let err = resolve_skill_dir("cursor", None, home).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cursor"), "msg was: {msg}");
        assert!(msg.contains("claude"), "msg was: {msg}");
    }
}
