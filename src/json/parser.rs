//! Recursive-descent JSON parser. Operates on the input as a byte slice and
//! tracks the current position so error messages can report line/column.

use super::value::Value;

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (line {}, col {})", self.msg, self.line, self.col)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<Value, ParseError> {
    let mut p = Parser { src: input.as_bytes(), pos: 0 };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos < p.src.len() {
        return Err(p.error("unexpected trailing content"));
    }
    Ok(v)
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn error(&self, msg: impl Into<String>) -> ParseError {
        let (line, col) = line_col(self.src, self.pos);
        ParseError { msg: msg.into(), line, col }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            match b {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn expect(&mut self, want: u8) -> Result<(), ParseError> {
        match self.peek() {
            Some(b) if b == want => {
                self.pos += 1;
                Ok(())
            }
            Some(b) => Err(self.error(format!(
                "expected '{}' but found '{}'",
                want as char, b as char
            ))),
            None => Err(self.error(format!(
                "expected '{}' but reached end of input",
                want as char
            ))),
        }
    }

    fn starts_with(&self, kw: &[u8]) -> bool {
        self.src.get(self.pos..self.pos + kw.len()) == Some(kw)
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(Value::String),
            Some(b't') | Some(b'f') => self.parse_bool(),
            Some(b'n') => self.parse_null(),
            Some(b) if b == b'-' || b.is_ascii_digit() => self.parse_number(),
            Some(b) => Err(self.error(format!(
                "unexpected character '{}'",
                b as char
            ))),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn parse_object(&mut self) -> Result<Value, ParseError> {
        self.expect(b'{')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(items));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let val = self.parse_value()?;
            items.push((key, val));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                Some(b) => {
                    return Err(self.error(format!(
                        "expected ',' or '}}' but found '{}'",
                        b as char
                    )));
                }
                None => return Err(self.error("unexpected end of input in object")),
            }
        }
        Ok(Value::Object(items))
    }

    fn parse_array(&mut self) -> Result<Value, ParseError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    continue;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                Some(b) => {
                    return Err(self.error(format!(
                        "expected ',' or ']' but found '{}'",
                        b as char
                    )));
                }
                None => return Err(self.error("unexpected end of input in array")),
            }
        }
        Ok(Value::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(self.error("unterminated string")),
                Some(b'"') => break,
                Some(b'\\') => {
                    match self.bump() {
                        None => return Err(self.error("unterminated escape")),
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\u{0008}'),
                        Some(b'f') => out.push('\u{000C}'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            let cp = self.parse_hex4()?;
                            if (0xD800..=0xDBFF).contains(&cp) {
                                if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                    return Err(self.error("expected low surrogate"));
                                }
                                let low = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err(self.error("invalid low surrogate"));
                                }
                                let combined =
                                    0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                                match char::from_u32(combined) {
                                    Some(c) => out.push(c),
                                    None => return Err(self.error("invalid surrogate pair")),
                                }
                            } else if (0xDC00..=0xDFFF).contains(&cp) {
                                return Err(self.error("unexpected low surrogate"));
                            } else {
                                match char::from_u32(cp) {
                                    Some(c) => out.push(c),
                                    None => {
                                        return Err(self.error(format!(
                                            "invalid unicode codepoint U+{:04X}",
                                            cp
                                        )));
                                    }
                                }
                            }
                        }
                        Some(c) => {
                            return Err(self.error(format!(
                                "invalid escape '\\{}'",
                                c as char
                            )));
                        }
                    }
                }
                Some(b) if b < 0x20 => {
                    return Err(self.error(format!(
                        "unescaped control character 0x{:02X}",
                        b
                    )));
                }
                Some(b) if b < 0x80 => out.push(b as char),
                Some(b) => {
                    let start = self.pos - 1;
                    let len = if b >= 0xF0 {
                        4
                    } else if b >= 0xE0 {
                        3
                    } else if b >= 0xC0 {
                        2
                    } else {
                        return Err(self.error("invalid UTF-8"));
                    };
                    let end = start + len;
                    if end > self.src.len() {
                        return Err(self.error("invalid UTF-8"));
                    }
                    let bytes = &self.src[start..end];
                    match std::str::from_utf8(bytes) {
                        Ok(s) => out.push_str(s),
                        Err(_) => return Err(self.error("invalid UTF-8")),
                    }
                    self.pos = end;
                }
            }
        }
        Ok(out)
    }

    fn parse_hex4(&mut self) -> Result<u32, ParseError> {
        let mut n = 0u32;
        for _ in 0..4 {
            match self.bump() {
                None => return Err(self.error("incomplete unicode escape")),
                Some(b) => {
                    let d = match b {
                        b'0'..=b'9' => (b - b'0') as u32,
                        b'a'..=b'f' => (b - b'a' + 10) as u32,
                        b'A'..=b'F' => (b - b'A' + 10) as u32,
                        _ => {
                            return Err(self.error(format!(
                                "invalid hex digit '{}'",
                                b as char
                            )));
                        }
                    };
                    n = (n << 4) | d;
                }
            }
        }
        Ok(n)
    }

    fn parse_bool(&mut self) -> Result<Value, ParseError> {
        if self.starts_with(b"true") {
            self.pos += 4;
            Ok(Value::Bool(true))
        } else if self.starts_with(b"false") {
            self.pos += 5;
            Ok(Value::Bool(false))
        } else {
            Err(self.error("expected 'true' or 'false'"))
        }
    }

    fn parse_null(&mut self) -> Result<Value, ParseError> {
        if self.starts_with(b"null") {
            self.pos += 4;
            Ok(Value::Null)
        } else {
            Err(self.error("expected 'null'"))
        }
    }

    fn parse_number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(self.error("invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("expected digit after '.'"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("expected digit in exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let lexeme = std::str::from_utf8(&self.src[start..self.pos])
            .expect("number lexeme is ascii")
            .to_string();
        Ok(Value::Number(lexeme))
    }
}

fn line_col(src: &[u8], pos: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    let end = pos.min(src.len());
    for &b in &src[..end] {
        if b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn must_parse(s: &str) -> Value {
        parse(s).unwrap_or_else(|e| panic!("parse({s:?}) failed: {e}"))
    }

    #[test]
    fn primitives() {
        assert_eq!(must_parse("null"), Value::Null);
        assert_eq!(must_parse("true"), Value::Bool(true));
        assert_eq!(must_parse("false"), Value::Bool(false));
        assert_eq!(must_parse("0"), Value::Number("0".into()));
        assert_eq!(must_parse("-1.5e10"), Value::Number("-1.5e10".into()));
        assert_eq!(must_parse("\"hi\""), Value::String("hi".into()));
    }

    #[test]
    fn whitespace() {
        assert_eq!(must_parse("  \n\t null  \n"), Value::Null);
    }

    #[test]
    fn empty_containers() {
        assert_eq!(must_parse("[]"), Value::Array(vec![]));
        assert_eq!(must_parse("{}"), Value::Object(vec![]));
    }

    #[test]
    fn nested() {
        let v = must_parse(r#"{"a":[1,{"b":null}],"c":true}"#);
        assert_eq!(
            v,
            Value::Object(vec![
                (
                    "a".into(),
                    Value::Array(vec![
                        Value::Number("1".into()),
                        Value::Object(vec![("b".into(), Value::Null)]),
                    ])
                ),
                ("c".into(), Value::Bool(true)),
            ])
        );
    }

    #[test]
    fn string_escapes() {
        let v = must_parse(r#""line\nbreak\ttab\"q\\b""#);
        assert_eq!(v, Value::String("line\nbreak\ttab\"q\\b".into()));
    }

    #[test]
    fn unicode_escape() {
        let v = must_parse(r#""é""#);
        assert_eq!(v, Value::String("é".into()));
    }

    #[test]
    fn surrogate_pair() {
        // U+1F600 (grinning face)
        let v = must_parse(r#""😀""#);
        assert_eq!(v, Value::String("😀".into()));
    }

    #[test]
    fn utf8_passthrough() {
        let v = must_parse("\"привет\"");
        assert_eq!(v, Value::String("привет".into()));
    }

    #[test]
    fn preserves_order() {
        let v = must_parse(r#"{"z":1,"a":2,"m":3}"#);
        if let Value::Object(items) = v {
            let keys: Vec<&str> = items.iter().map(|(k, _)| k.as_str()).collect();
            assert_eq!(keys, vec!["z", "a", "m"]);
        } else {
            panic!("expected object");
        }
    }

    #[test]
    fn errors_on_trailing_garbage() {
        assert!(parse("null garbage").is_err());
    }

    #[test]
    fn errors_on_unterminated_string() {
        assert!(parse(r#""hello"#).is_err());
    }

    #[test]
    fn errors_on_invalid_number() {
        assert!(parse("01").is_err());
        assert!(parse("1.").is_err());
        assert!(parse("1e").is_err());
    }

    #[test]
    fn error_position() {
        let err = parse("{\n  \"a\": ?\n}").unwrap_err();
        assert_eq!(err.line, 2);
        assert!(err.msg.contains("unexpected character"));
    }
}
