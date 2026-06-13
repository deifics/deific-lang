//! Line-based indentation lexer.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Indent,
    Dedent,
    Newline,
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    Op(String),
    Eof,
}

#[derive(Debug)]
pub struct LexError {
    pub line: usize,
    pub msg: String,
}

const KEYWORDS: &[&str] = &[
    "func", "return", "for", "in", "while", "if", "elif", "else",
    "and", "or", "not", "True", "False",
    "break", "continue", "pass", "None", "bigint", "ref",
];

pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

pub fn lex(src: &str) -> Result<Vec<(Tok, usize)>, LexError> {
    let mut out: Vec<(Tok, usize)> = Vec::new();
    let mut indents: Vec<usize> = vec![0];

    for (i, raw_line) in src.lines().enumerate() {
        let lineno = i + 1;
        let line = strip_comment(raw_line);
        if line.trim().is_empty() {
            continue;
        }

        let indent = leading_width(line);
        let cur = *indents.last().unwrap();
        if indent > cur {
            indents.push(indent);
            out.push((Tok::Indent, lineno));
        } else {
            while indent < *indents.last().unwrap() {
                indents.pop();
                out.push((Tok::Dedent, lineno));
            }
            if indent != *indents.last().unwrap() {
                return Err(LexError { line: lineno, msg: "inconsistent indentation".into() });
            }
        }

        tokenize_line(line.trim_start(), lineno, &mut out)?;
        out.push((Tok::Newline, lineno));
    }

    while indents.len() > 1 {
        indents.pop();
        out.push((Tok::Dedent, src.lines().count()));
    }
    out.push((Tok::Eof, src.lines().count() + 1));
    Ok(out)
}

fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_str = false;
    let mut str_char = b'"';
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if !in_str && (b == b'"' || b == b'\'') {
            in_str = true;
            str_char = b;
        } else if in_str && b == str_char && (i == 0 || bytes[i - 1] != b'\\') {
            in_str = false;
        } else if b == b'#' && !in_str {
            return &line[..i];
        }
        i += 1;
    }
    line
}

fn leading_width(line: &str) -> usize {
    let mut w = 0;
    for c in line.chars() {
        match c {
            ' ' => w += 1,
            '\t' => w += 4,
            _ => break,
        }
    }
    w
}

fn tokenize_line(s: &str, lineno: usize, out: &mut Vec<(Tok, usize)>) -> Result<(), LexError> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // String literals — both " and '
        if c == '"' || c == '\'' {
            let quote = c;
            let mut buf = String::new();
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    buf.push(chars[i]);
                    buf.push(chars[i + 1]);
                    i += 2;
                } else {
                    buf.push(chars[i]);
                    i += 1;
                }
            }
            if i >= chars.len() {
                return Err(LexError { line: lineno, msg: "unterminated string".into() });
            }
            i += 1;
            out.push((Tok::Str(buf), lineno));
            continue;
        }

        // Numeric literals (integer or float)
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            // Check for float: digit sequence followed by . digit or e/E
            let mut is_float = false;
            if i < chars.len() && chars[i] == '.' {
                let after_dot = if i + 1 < chars.len() { chars[i + 1] } else { '\0' };
                if after_dot.is_ascii_digit() || after_dot == 'e' || after_dot == 'E' {
                    is_float = true;
                    i += 1; // consume '.'
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                is_float = true;
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let lit: String = chars[start..i].iter().collect();
            if is_float {
                let v: f64 = lit.parse().map_err(|_| LexError {
                    line: lineno,
                    msg: format!("invalid float literal: {}", lit),
                })?;
                out.push((Tok::Float(v), lineno));
            } else {
                let v: i64 = lit.parse().map_err(|_| LexError {
                    line: lineno,
                    msg: format!("integer literal out of range: {}", lit),
                })?;
                out.push((Tok::Int(v), lineno));
            }
            continue;
        }

        // Float starting with '.'  e.g. .5
        if c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let lit: String = chars[start..i].iter().collect();
            let v: f64 = lit.parse().map_err(|_| LexError {
                line: lineno,
                msg: format!("invalid float literal: {}", lit),
            })?;
            out.push((Tok::Float(v), lineno));
            continue;
        }

        // Identifiers / keywords
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            out.push((Tok::Ident(word), lineno));
            continue;
        }

        // Operators — try 3-char first, then 2-char, then 1-char.
        let rem = chars.len() - i;

        if rem >= 3 {
            let three: String = chars[i..i + 3].iter().collect();
            if matches!(three.as_str(), "<<=" | ">>=" | "//=" | "**=") {
                out.push((Tok::Op(three), lineno));
                i += 3;
                continue;
            }
        }

        if rem >= 2 {
            let two: String = chars[i..i + 2].iter().collect();
            if matches!(
                two.as_str(),
                "==" | "!=" | "<=" | ">=" | "//" | "->"
                | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^="
                | "**" | "<<" | ">>"
            ) {
                out.push((Tok::Op(two), lineno));
                i += 2;
                continue;
            }
        }

        if "+-*/%<>=()[]:,.&|^~{}!".contains(c) {
            out.push((Tok::Op(c.to_string()), lineno));
            i += 1;
            continue;
        }

        return Err(LexError {
            line: lineno,
            msg: format!("unexpected character: {:?}", c),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        lex(src).unwrap().into_iter().map(|(t, _)| t).collect()
    }

    #[test]
    fn test_int() {
        assert!(matches!(toks("42")[0], Tok::Int(42)));
    }

    #[test]
    fn test_float() {
        assert!(matches!(toks("3.14")[0], Tok::Float(_)));
        assert!(matches!(toks(".5")[0], Tok::Float(_)));
        assert!(matches!(toks("1e5")[0], Tok::Float(_)));
    }

    #[test]
    fn test_augmented_ops() {
        let t = toks("x += 1");
        assert!(matches!(&t[1], Tok::Op(o) if o == "+="));
    }

    #[test]
    fn test_power() {
        let t = toks("x **= 2");
        assert!(matches!(&t[1], Tok::Op(o) if o == "**="));
    }

    #[test]
    fn test_shift() {
        let t = toks("x << 2");
        assert!(matches!(&t[1], Tok::Op(o) if o == "<<"));
    }

    #[test]
    fn test_new_keywords() {
        assert!(is_keyword("break"));
        assert!(is_keyword("continue"));
        assert!(is_keyword("pass"));
        assert!(is_keyword("None"));
        assert!(is_keyword("bigint"));
        assert!(is_keyword("ref"));
    }

    #[test]
    fn test_single_quote_string() {
        let t = toks("'hello'");
        assert!(matches!(&t[0], Tok::Str(s) if s == "hello"));
    }

    #[test]
    fn test_braces() {
        let t = toks("{x: y}");
        assert!(matches!(&t[0], Tok::Op(o) if o == "{"));
        assert!(matches!(&t[4], Tok::Op(o) if o == "}"));
    }
}
