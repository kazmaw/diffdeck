//! difit 互換コメント型。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Old,
    New,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LineRange {
    Single(u32),
    Range { start: u32, end: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub side: Side,
    pub line: LineRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    #[serde(rename = "type")]
    pub comment_type: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub position: Position,
    pub body: String,
}

impl Comment {
    /// difit 互換のスレッドコメントを作る。
    pub fn thread(
        file_path: impl Into<String>,
        side: Side,
        line: LineRange,
        body: impl Into<String>,
    ) -> Self {
        Comment {
            comment_type: "thread".into(),
            file_path: file_path.into(),
            position: Position { side, line },
            body: body.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentFile {
    pub schema: String,
    pub repo: String,
    pub scope: String,
    pub comments: Vec<Comment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_serializes_like_difit() {
        let c = Comment::thread(
            "src/auth.ts",
            Side::New,
            LineRange::Single(16),
            "warn 要らない？",
        );
        let v: serde_json::Value = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "thread");
        assert_eq!(v["filePath"], "src/auth.ts");
        assert_eq!(v["position"]["side"], "new");
        assert_eq!(v["position"]["line"], 16);
        assert_eq!(v["body"], "warn 要らない？");
    }

    #[test]
    fn range_line_serializes_as_object() {
        let c = Comment::thread(
            "src/ui.tsx",
            Side::New,
            LineRange::Range { start: 36, end: 39 },
            "dead code?",
        );
        let v: serde_json::Value = serde_json::to_value(&c).unwrap();
        assert_eq!(v["position"]["line"]["start"], 36);
        assert_eq!(v["position"]["line"]["end"], 39);
    }

    #[test]
    fn round_trip_preserves_value() {
        let original = CommentFile {
            schema: "diffdeck/v1".into(),
            repo: "/Users/wada/proj/feat-login".into(),
            scope: "working".into(),
            comments: vec![
                Comment::thread("src/auth.ts", Side::New, LineRange::Single(16), "a"),
                Comment::thread(
                    "src/ui.tsx",
                    Side::Old,
                    LineRange::Range { start: 1, end: 4 },
                    "b",
                ),
            ],
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: CommentFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn deserializes_difit_shaped_input() {
        let input = r#"{
            "type": "thread",
            "filePath": "src/auth.ts",
            "position": { "side": "new", "line": 16 },
            "body": "hi"
        }"#;
        let c: Comment = serde_json::from_str(input).unwrap();
        assert_eq!(c.position.side, Side::New);
        assert_eq!(c.position.line, LineRange::Single(16));
    }
}
