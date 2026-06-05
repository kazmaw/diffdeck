//! syntect による 1 行単位のシンタックスハイライト。

use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// ハイライト済みの 1 セグメント（前景色 RGB + テキスト）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlSpan {
    pub rgb: (u8, u8, u8),
    pub text: String,
}

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();
        Highlighter { syntax_set, theme }
    }

    /// 1 行を拡張子に応じてハイライトする。
    /// 拡張子が None / 未知ならプレーン 1 セグメントを返す。
    /// 返り値のテキストを連結すると入力 content と一致する（改行は含めない）。
    pub fn highlight_line(&self, content: &str, extension: Option<&str>) -> Vec<HlSpan> {
        if content.is_empty() {
            return Vec::new();
        }

        let syntax = extension
            .and_then(|ext| self.syntax_set.find_syntax_by_extension(ext));
        let syntax = match syntax {
            Some(s) => s,
            None => return vec![HlSpan { rgb: (200, 200, 200), text: content.to_string() }],
        };

        let mut h = HighlightLines::new(syntax, &self.theme);
        let mut out = Vec::new();
        // syntect は行末を含む文字列を期待するため、改行を付けてから除去する。
        let with_nl = format!("{content}\n");
        for piece in LinesWithEndings::from(&with_nl) {
            let ranges = match h.highlight_line(piece, &self.syntax_set) {
                Ok(r) => r,
                Err(_) => return vec![HlSpan { rgb: (200, 200, 200), text: content.to_string() }],
            };
            for (style, text) in ranges {
                let text = text.trim_end_matches('\n');
                if text.is_empty() {
                    continue;
                }
                out.push(HlSpan {
                    rgb: (style.foreground.r, style.foreground.g, style.foreground.b),
                    text: text.to_string(),
                });
            }
        }
        if out.is_empty() {
            out.push(HlSpan { rgb: (200, 200, 200), text: content.to_string() });
        }
        out
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_extension_returns_plain_single_span() {
        let h = Highlighter::new();
        let spans = h.highlight_line("hello world", None);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "hello world");
    }

    #[test]
    fn highlighted_text_concatenates_to_input() {
        let h = Highlighter::new();
        let content = "let x = 42;";
        let spans = h.highlight_line(content, Some("rs"));
        let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, content);
        assert!(!spans.is_empty());
    }

    #[test]
    fn empty_line_is_safe() {
        let h = Highlighter::new();
        let spans = h.highlight_line("", Some("rs"));
        let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, "");
    }
}
