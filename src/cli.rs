//! CLI 引数を DiffSpec に解決する。サブコマンド省略時は従来の位置引数で diff スコープを解決する。

use crate::model::{DiffSpec, Scope};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "diffdeck",
    about = "Fast in-terminal diff viewer for Claude Code",
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// サブコマンド（省略時は下の位置引数で diff スコープを解決）。
    #[command(subcommand)]
    pub command: Option<Command>,

    /// "staged" / ref / target のいずれか（省略時は作業ツリー）
    pub arg1: Option<String>,
    /// 2点間差分の base
    pub arg2: Option<String>,
    /// base 分岐点からの差分（PR 相当）
    #[arg(long = "merge-base")]
    pub merge_base: bool,
}

/// diffdeck のサブコマンド。MVP は install-skill のみ。
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Install the diffdeck AI skill into your AI tool's skills directory.
    InstallSkill(InstallSkillArgs),
}

/// `diffdeck install-skill` の引数。
#[derive(clap::Args, Debug)]
pub struct InstallSkillArgs {
    /// Target AI tool. Only "claude" is supported for now.
    #[arg(long, default_value = "claude")]
    pub target: String,
    /// Install directory; overrides --target.
    #[arg(long)]
    pub dir: Option<PathBuf>,
    /// Print the skill body to stdout instead of installing.
    #[arg(long)]
    pub print: bool,
    /// Overwrite an existing SKILL.md without confirmation.
    #[arg(long)]
    pub force: bool,
}

impl Cli {
    pub fn to_spec(&self) -> DiffSpec {
        match (self.arg1.as_deref(), self.arg2.as_deref()) {
            (None, _) => DiffSpec {
                scope: Scope::Working,
                target: None,
                base: None,
                merge_base: false,
            },
            (Some("staged"), None) => DiffSpec {
                scope: Scope::Staged,
                target: None,
                base: None,
                merge_base: false,
            },
            (Some(target), None) => DiffSpec {
                scope: Scope::Ref,
                target: Some(target.to_string()),
                base: None,
                merge_base: false,
            },
            (Some(target), Some(base)) => DiffSpec {
                scope: Scope::Range,
                target: Some(target.to_string()),
                base: Some(base.to_string()),
                merge_base: self.merge_base,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> DiffSpec {
        let mut full = vec!["diffdeck"];
        full.extend_from_slice(args);
        Cli::parse_from(full).to_spec()
    }

    #[test]
    fn no_args_is_working_tree() {
        assert_eq!(
            parse(&[]),
            DiffSpec {
                scope: Scope::Working,
                target: None,
                base: None,
                merge_base: false
            }
        );
    }

    #[test]
    fn staged_keyword() {
        assert_eq!(
            parse(&["staged"]),
            DiffSpec {
                scope: Scope::Staged,
                target: None,
                base: None,
                merge_base: false
            }
        );
    }

    #[test]
    fn single_ref() {
        assert_eq!(
            parse(&["abc123"]),
            DiffSpec {
                scope: Scope::Ref,
                target: Some("abc123".into()),
                base: None,
                merge_base: false
            }
        );
    }

    #[test]
    fn two_point_range() {
        assert_eq!(
            parse(&["feature", "main"]),
            DiffSpec {
                scope: Scope::Range,
                target: Some("feature".into()),
                base: Some("main".into()),
                merge_base: false
            }
        );
    }

    #[test]
    fn merge_base_flag() {
        assert_eq!(
            parse(&["feature", "main", "--merge-base"]),
            DiffSpec {
                scope: Scope::Range,
                target: Some("feature".into()),
                base: Some("main".into()),
                merge_base: true
            }
        );
    }

    #[test]
    fn install_skill_subcommand_parses() {
        let cli = Cli::parse_from(["diffdeck", "install-skill", "--print"]);
        match cli.command {
            Some(Command::InstallSkill(args)) => {
                assert!(args.print);
                assert_eq!(args.target, "claude");
                assert!(args.dir.is_none());
                assert!(!args.force);
            }
            other => panic!("expected InstallSkill, got {other:?}"),
        }
    }

    #[test]
    fn install_skill_accepts_dir_and_force() {
        let cli = Cli::parse_from(["diffdeck", "install-skill", "--dir", "/tmp/x", "--force"]);
        match cli.command {
            Some(Command::InstallSkill(args)) => {
                assert_eq!(args.dir.as_deref(), Some(std::path::Path::new("/tmp/x")));
                assert!(args.force);
            }
            other => panic!("expected InstallSkill, got {other:?}"),
        }
    }

    #[test]
    fn diff_args_still_resolve_when_no_subcommand() {
        // 後方互換: 新サブコマンドが位置引数に影響しないこと
        let cli = Cli::parse_from(["diffdeck", "feature", "main", "--merge-base"]);
        assert!(cli.command.is_none());
        assert_eq!(
            cli.to_spec(),
            DiffSpec {
                scope: Scope::Range,
                target: Some("feature".into()),
                base: Some("main".into()),
                merge_base: true
            }
        );
    }
}
