//! Render a `Value` back to JSON text.
//!
//! Two output modes (compact / indented), optional alphabetical key order,
//! and optional ANSI coloring controlled by `WriteOpts`.

use super::value::Value;

#[derive(Clone, Copy, Debug)]
pub struct WriteOpts {
    /// `Some(n)` indents each level with `n` spaces; `None` produces compact JSON.
    pub indent: Option<usize>,
    /// Sort object keys alphabetically.
    pub sort_keys: bool,
    /// Wrap tokens with ANSI color codes.
    pub color: bool,
}

impl Default for WriteOpts {
    fn default() -> Self {
        Self { indent: Some(2), sort_keys: false, color: false }
    }
}

const RESET: &str = "\x1b[0m";
const C_KEY: &str = "\x1b[34m";
const C_STR: &str = "\x1b[32m";
const C_NUM: &str = "\x1b[36m";
const C_BOOL: &str = "\x1b[33m";
const C_NULL: &str = "\x1b[90m";

pub fn to_string(v: &Value, opts: WriteOpts) -> String {
    let mut out = String::new();
    write_value(&mut out, v, opts, 0);
    out
}

fn write_value(out: &mut String, v: &Value, opts: WriteOpts, depth: usize) {
    match v {
        Value::Null => paint(out, opts.color, C_NULL, "null"),
        Value::Bool(b) => paint(out, opts.color, C_BOOL, if *b { "true" } else { "false" }),
        Value::Number(n) => paint(out, opts.color, C_NUM, n),
        Value::String(s) => {
            if opts.color {
                out.push_str(C_STR);
            }
            write_string(out, s);
            if opts.color {
                out.push_str(RESET);
            }
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if opts.indent.is_some() {
                    out.push('\n');
                    write_indent(out, opts, depth + 1);
                }
                write_value(out, item, opts, depth + 1);
            }
            if opts.indent.is_some() {
                out.push('\n');
                write_indent(out, opts, depth);
            }
            out.push(']');
        }
        Value::Object(items) => {
            if items.is_empty() {
                out.push_str("{}");
                return;
            }
            let order: Vec<usize> = if opts.sort_keys {
                let mut idxs: Vec<usize> = (0..items.len()).collect();
                idxs.sort_by(|&a, &b| items[a].0.cmp(&items[b].0));
                idxs
            } else {
                (0..items.len()).collect()
            };

            out.push('{');
            for (i, &idx) in order.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if opts.indent.is_some() {
                    out.push('\n');
                    write_indent(out, opts, depth + 1);
                }
                if opts.color {
                    out.push_str(C_KEY);
                }
                write_string(out, &items[idx].0);
                if opts.color {
                    out.push_str(RESET);
                }
                out.push(':');
                if opts.indent.is_some() {
                    out.push(' ');
                }
                write_value(out, &items[idx].1, opts, depth + 1);
            }
            if opts.indent.is_some() {
                out.push('\n');
                write_indent(out, opts, depth);
            }
            out.push('}');
        }
    }
}

fn paint(out: &mut String, color: bool, code: &str, text: &str) {
    if color {
        out.push_str(code);
    }
    out.push_str(text);
    if color {
        out.push_str(RESET);
    }
}

fn write_indent(out: &mut String, opts: WriteOpts, depth: usize) {
    if let Some(n) = opts.indent {
        for _ in 0..n * depth {
            out.push(' ');
        }
    }
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pretty(v: &Value) -> String {
        to_string(v, WriteOpts::default())
    }

    fn compact(v: &Value) -> String {
        to_string(v, WriteOpts { indent: None, sort_keys: false, color: false })
    }

    #[test]
    fn primitives() {
        assert_eq!(pretty(&Value::Null), "null");
        assert_eq!(pretty(&Value::Bool(true)), "true");
        assert_eq!(pretty(&Value::Number("42".into())), "42");
        assert_eq!(pretty(&Value::String("hi".into())), "\"hi\"");
    }

    #[test]
    fn empty_containers() {
        assert_eq!(pretty(&Value::Array(vec![])), "[]");
        assert_eq!(pretty(&Value::Object(vec![])), "{}");
    }

    #[test]
    fn pretty_object() {
        let v = Value::Object(vec![
            ("a".into(), Value::Number("1".into())),
            ("b".into(), Value::Bool(false)),
        ]);
        assert_eq!(pretty(&v), "{\n  \"a\": 1,\n  \"b\": false\n}");
    }

    #[test]
    fn compact_object() {
        let v = Value::Object(vec![
            ("a".into(), Value::Number("1".into())),
            ("b".into(), Value::Bool(false)),
        ]);
        assert_eq!(compact(&v), "{\"a\":1,\"b\":false}");
    }

    #[test]
    fn nested_indent() {
        let v = Value::Object(vec![(
            "x".into(),
            Value::Array(vec![Value::Number("1".into()), Value::Number("2".into())]),
        )]);
        assert_eq!(pretty(&v), "{\n  \"x\": [\n    1,\n    2\n  ]\n}");
    }

    #[test]
    fn sort_keys() {
        let v = Value::Object(vec![
            ("b".into(), Value::Number("1".into())),
            ("a".into(), Value::Number("2".into())),
        ]);
        let opts = WriteOpts { indent: None, sort_keys: true, color: false };
        assert_eq!(to_string(&v, opts), "{\"a\":2,\"b\":1}");
    }

    #[test]
    fn custom_indent() {
        let v = Value::Object(vec![("a".into(), Value::Number("1".into()))]);
        let opts = WriteOpts { indent: Some(4), sort_keys: false, color: false };
        assert_eq!(to_string(&v, opts), "{\n    \"a\": 1\n}");
    }

    #[test]
    fn escapes_control_chars() {
        let v = Value::String("a\nb\tc\"d\\e".into());
        assert_eq!(pretty(&v), "\"a\\nb\\tc\\\"d\\\\e\"");
    }
}
