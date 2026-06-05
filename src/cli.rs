//! CLI 引数を DiffSpec に解決する。

use crate::model::{DiffSpec, Scope};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "diffdeck", about = "Fast in-terminal diff viewer for Claude Code")]
pub struct Cli {
    /// "staged" / ref / target のいずれか（省略時は作業ツリー）
    pub arg1: Option<String>,
    /// 2点間差分の base
    pub arg2: Option<String>,
    /// base 分岐点からの差分（PR 相当）
    #[arg(long = "merge-base")]
    pub merge_base: bool,
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
        assert_eq!(parse(&[]), DiffSpec { scope: Scope::Working, target: None, base: None, merge_base: false });
    }

    #[test]
    fn staged_keyword() {
        assert_eq!(parse(&["staged"]), DiffSpec { scope: Scope::Staged, target: None, base: None, merge_base: false });
    }

    #[test]
    fn single_ref() {
        assert_eq!(parse(&["abc123"]), DiffSpec { scope: Scope::Ref, target: Some("abc123".into()), base: None, merge_base: false });
    }

    #[test]
    fn two_point_range() {
        assert_eq!(parse(&["feature", "main"]), DiffSpec { scope: Scope::Range, target: Some("feature".into()), base: Some("main".into()), merge_base: false });
    }

    #[test]
    fn merge_base_flag() {
        assert_eq!(parse(&["feature", "main", "--merge-base"]), DiffSpec { scope: Scope::Range, target: Some("feature".into()), base: Some("main".into()), merge_base: true });
    }
}
