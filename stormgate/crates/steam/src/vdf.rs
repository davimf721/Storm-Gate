//! Parser for Valve's text KeyValues format (`.vdf` / `.acf`).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Str(String),
    Obj(Vec<(String, Value)>),
}

impl Value {
    /// Case-insensitive lookup of a direct child.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(items) => items
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Value::Str(_) => None,
        }
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::Str(s) => Some(s),
            Value::Obj(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Value)] {
        match self {
            Value::Obj(items) => items,
            Value::Str(_) => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

const MAX_DEPTH: usize = 64;

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
}

#[derive(Debug, PartialEq)]
enum Token {
    Str(String),
    Open,
    Close,
}

impl Parser<'_> {
    fn err(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            line: self.line,
            message: message.into(),
        }
    }

    fn skip_ws(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c == '\n' {
                self.line += 1;
                self.chars.next();
            } else if c.is_whitespace() {
                self.chars.next();
            } else if c == '/' {
                // `//` comment until end of line.
                let mut clone = self.chars.clone();
                clone.next();
                if clone.peek() == Some(&'/') {
                    while let Some(&c) = self.chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.chars.next();
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn token(&mut self) -> Result<Option<Token>, ParseError> {
        self.skip_ws();
        let Some(c) = self.chars.next() else {
            return Ok(None);
        };
        match c {
            '{' => Ok(Some(Token::Open)),
            '}' => Ok(Some(Token::Close)),
            '"' => {
                let mut s = String::new();
                loop {
                    match self.chars.next() {
                        None => return Err(self.err("unterminated string")),
                        Some('"') => break,
                        Some('\\') => match self.chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'),
                            Some(other) => {
                                s.push('\\');
                                s.push(other);
                            }
                            None => return Err(self.err("unterminated escape")),
                        },
                        Some('\n') => {
                            self.line += 1;
                            s.push('\n');
                        }
                        Some(c) => s.push(c),
                    }
                }
                Ok(Some(Token::Str(s)))
            }
            c => {
                let mut s = String::from(c);
                while let Some(&c) = self.chars.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                        break;
                    }
                    s.push(c);
                    self.chars.next();
                }
                Ok(Some(Token::Str(s)))
            }
        }
    }

    fn object(&mut self, depth: usize, top: bool) -> Result<Vec<(String, Value)>, ParseError> {
        if depth > MAX_DEPTH {
            return Err(self.err("nesting too deep"));
        }
        let mut items = Vec::new();
        loop {
            let key = match self.token()? {
                None if top => return Ok(items),
                None => return Err(self.err("unexpected end of file")),
                Some(Token::Close) if !top => return Ok(items),
                Some(Token::Close) => return Err(self.err("unexpected '}'")),
                Some(Token::Open) => return Err(self.err("expected key, found '{'")),
                Some(Token::Str(k)) => k,
            };
            let value = match self.token()? {
                Some(Token::Str(v)) => Value::Str(v),
                Some(Token::Open) => Value::Obj(self.object(depth + 1, false)?),
                _ => return Err(self.err(format!("missing value for key '{key}'"))),
            };
            // Skip platform conditionals such as [$WIN32].
            self.skip_ws();
            if self.chars.peek() == Some(&'[') {
                for c in self.chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
            items.push((key, value));
        }
    }
}

/// Parses a KeyValues document into a root object.
pub fn parse(text: &str) -> Result<Value, ParseError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Parser {
        chars: text.chars().peekable(),
        line: 1,
    };
    Ok(Value::Obj(p.object(0, true)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested() {
        let doc = parse(
            r#"
// comment
"libraryfolders"
{
    "0"
    {
        "path"      "C:\\Program Files (x86)\\Steam"
        "apps" { "228980" "123" "1091500" "456" }
    }
    unquoted value
}
"#,
        )
        .unwrap();
        let lf = doc.get("LibraryFolders").unwrap();
        let zero = lf.get("0").unwrap();
        assert_eq!(zero.str("path"), Some("C:\\Program Files (x86)\\Steam"));
        assert_eq!(zero.get("apps").unwrap().entries().len(), 2);
        assert_eq!(lf.str("unquoted"), Some("value"));
    }

    #[test]
    fn errors() {
        assert!(parse("\"a\" {").is_err());
        assert!(parse("\"a\"").is_err());
        assert!(parse("}").is_err());
        assert!(parse("\"a\" \"b").is_err());
        let deep = "\"a\" {".repeat(100) + &"}".repeat(100);
        assert!(parse(&deep).is_err());
    }
}
