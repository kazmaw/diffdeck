//! git を実行し DiffSpec の差分を取得する。
use crate::diff_parse::parse_diff;
use crate::model::{DiffSpec, FileDiff, Scope};
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

const COMMON: &[&str] = &["--no-color", "--no-ext-diff", "-M"];

/// DiffSpec を `git` のサブコマンド引数列に変換する。
pub fn git_diff_args(spec: &DiffSpec) -> Vec<String> {
    let mut args: Vec<String> = vec!["diff".to_string()];
    args.extend(COMMON.iter().map(|s| s.to_string()));
    match spec.scope {
        Scope::Working => {
            args.push("HEAD".to_string());
        }
        Scope::Staged => {
            args.push("--staged".to_string());
        }
        Scope::Ref => {
            let t = spec.target.clone().unwrap_or_default();
            args.push(format!("{t}^"));
            args.push(t);
        }
        Scope::Range => {
            if spec.merge_base {
                args.push("--merge-base".to_string());
            }
            if let Some(base) = &spec.base {
                args.push(base.clone());
            }
            if let Some(target) = &spec.target {
                args.push(target.clone());
            }
        }
    }
    args
}

/// repo ディレクトリが git work tree 内かを判定する。
pub fn is_git_repo(repo: &Path) -> bool {
    Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// spec に従って git diff を実行し、構造化モデルを返す。
pub fn load_file_diffs(spec: &DiffSpec, repo: &Path) -> Result<Vec<FileDiff>> {
    if spec.scope == Scope::Ref && spec.target.is_none() {
        bail!("Scope::Ref requires a target ref");
    }
    let args = git_diff_args(spec);
    let output = Command::new("git")
        .args(&args)
        .current_dir(repo)
        .output()
        .context("failed to spawn git")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {}", stderr.trim());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff(&text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scope;

    fn spec(scope: Scope, target: Option<&str>, base: Option<&str>, mb: bool) -> DiffSpec {
        DiffSpec {
            scope,
            target: target.map(String::from),
            base: base.map(String::from),
            merge_base: mb,
        }
    }

    #[test]
    fn working_args() {
        let a = git_diff_args(&spec(Scope::Working, None, None, false));
        assert_eq!(a, vec!["diff", "--no-color", "--no-ext-diff", "-M", "HEAD"]);
    }

    #[test]
    fn staged_args() {
        let a = git_diff_args(&spec(Scope::Staged, None, None, false));
        assert_eq!(a, vec!["diff", "--no-color", "--no-ext-diff", "-M", "--staged"]);
    }

    #[test]
    fn ref_args() {
        let a = git_diff_args(&spec(Scope::Ref, Some("abc"), None, false));
        assert_eq!(a, vec!["diff", "--no-color", "--no-ext-diff", "-M", "abc^", "abc"]);
    }

    #[test]
    fn range_args() {
        let a = git_diff_args(&spec(Scope::Range, Some("feature"), Some("main"), false));
        assert_eq!(a, vec!["diff", "--no-color", "--no-ext-diff", "-M", "main", "feature"]);
    }

    #[test]
    fn merge_base_args() {
        let a = git_diff_args(&spec(Scope::Range, Some("feature"), Some("main"), true));
        assert_eq!(a, vec!["diff", "--no-color", "--no-ext-diff", "-M", "--merge-base", "main", "feature"]);
    }

    #[test]
    fn ref_without_target_is_error() {
        let dir = std::env::temp_dir();
        let s = spec(Scope::Ref, None, None, false);
        let err = load_file_diffs(&s, &dir).unwrap_err();
        assert!(err.to_string().contains("requires a target"));
    }
}
