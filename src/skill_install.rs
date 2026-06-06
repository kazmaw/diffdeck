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

/// install-skill の解決済みオプション（clap から組み立てる。skill_install を clap から分離）。
pub struct InstallOptions {
    pub target: String,
    pub dir: Option<PathBuf>,
    pub print: bool,
    pub force: bool,
}

/// install-skill を実行する。出力先は引数で受け取りテスト可能にする。
///
/// `--print` 時は本文を `out` に書くだけでファイルは作らない。
/// 非対話前提: 既存ファイルは `--force` なしでは上書きしない（中止して案内）。
pub fn run_install_skill(opts: &InstallOptions, home: &Path, out: &mut impl Write) -> Result<()> {
    if opts.print {
        out.write_all(SKILL_BODY.as_bytes())?;
        return Ok(());
    }
    let dir = resolve_skill_dir(&opts.target, opts.dir.as_deref(), home)?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create directory: {}", dir.display()))?;
    let path = dir.join("SKILL.md");
    if path.exists() && !opts.force {
        bail!(
            "{} already exists. re-run with --force to overwrite.",
            path.display()
        );
    }
    std::fs::write(&path, SKILL_BODY)
        .with_context(|| format!("failed to write: {}", path.display()))?;
    writeln!(out, "installed skill to {}", path.display())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    fn opts(dir: &Path, print: bool, force: bool) -> InstallOptions {
        InstallOptions {
            target: "claude".into(),
            dir: Some(dir.to_path_buf()),
            print,
            force,
        }
    }

    #[test]
    fn install_writes_skill_to_dir() {
        let tmp = tempdir().unwrap();
        let home = Path::new("/unused");
        let mut out = Vec::new();
        run_install_skill(&opts(tmp.path(), false, false), home, &mut out).unwrap();
        let written = std::fs::read_to_string(tmp.path().join("SKILL.md")).unwrap();
        assert_eq!(written, SKILL_BODY);
        assert!(String::from_utf8_lossy(&out).contains("installed skill"));
    }

    #[test]
    fn print_emits_body_and_writes_no_file() {
        let tmp = tempdir().unwrap();
        let home = Path::new("/unused");
        let mut out = Vec::new();
        run_install_skill(&opts(tmp.path(), true, false), home, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), SKILL_BODY);
        assert!(!tmp.path().join("SKILL.md").exists());
    }

    #[test]
    fn existing_file_protected_without_force() {
        let tmp = tempdir().unwrap();
        let home = Path::new("/unused");
        std::fs::write(tmp.path().join("SKILL.md"), "OLD").unwrap();
        let mut out = Vec::new();
        let err = run_install_skill(&opts(tmp.path(), false, false), home, &mut out).unwrap_err();
        assert!(err.to_string().contains("--force"), "msg: {err}");
        // untouched
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("SKILL.md")).unwrap(),
            "OLD"
        );
    }

    #[test]
    fn force_overwrites_existing_file() {
        let tmp = tempdir().unwrap();
        let home = Path::new("/unused");
        std::fs::write(tmp.path().join("SKILL.md"), "OLD").unwrap();
        let mut out = Vec::new();
        run_install_skill(&opts(tmp.path(), false, true), home, &mut out).unwrap();
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("SKILL.md")).unwrap(),
            SKILL_BODY
        );
    }

    #[test]
    fn install_creates_missing_parent_dirs() {
        let tmp = tempdir().unwrap();
        let nested = tmp.path().join("a").join("b");
        let home = Path::new("/unused");
        let mut out = Vec::new();
        run_install_skill(&opts(&nested, false, false), home, &mut out).unwrap();
        assert!(nested.join("SKILL.md").exists());
    }

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
