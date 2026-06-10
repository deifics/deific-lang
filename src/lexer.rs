//! Line-based indentation lexer.
//!
//! Deific's v0 grammar has no bracketed line continuations, so a per-physical-
//! line tokenizer is enough and far simpler than a streaming one. Each logical
//! line emits INDENT/DEDENT as needed, its tokens, then NEWLINE.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Indent,
    Dedent,
    Newline,
    Ident(String),
    Int(i64),
    Str(String),
    Op(String),
    Eof,
}

pub struct LexError {
    pub line: usize,
    pub msg: String,
}

const KEYWORDS: &[&str] = &[
    "def", "return", "for", "in", "while", "if", "elif", "else", "and", "or",
    "not", "True", "False",
];

pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

pub fn lex(src: &str) -> Result<Vec<(Tok, usize)>, LexError> {
    let mut out: Vec<(Tok, usize)> = Vec::new();
    let mut indents: Vec<usize> = vec![0];

    for (i, raw_line) in src.lines().enumerate() {
        let lineno = i + 1;

        // Strip comments (no `#` appears inside our string literals' concerns
        // for v0 beyond simple cases; treat first unquoted # as comment start).
        let line = strip_comment(raw_line);
        if line.trim().is_empty() {
            continue; // blank lines carry no indentation meaning
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
                return Err(LexError {
                    line: lineno,
                    msg: "inconsistent indentation".into(),
                });
            }
        }

        tokenize_line(line.trim_start(), lineno, &mut out)?;
        out.push((Tok::Newline, lineno));
    }

    // Close any open blocks at EOF.
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
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'"' {
            in_str = !in_str;
        } else if b == b'#' && !in_str {
            return &line[..i];
        }
    }
    line
}

fn leading_width(line: &str) -> usize {
    let mut w = 0;
    for c in line.chars() {
        match c {
            ' ' => w += 1,
            '\t' => w += 4, // tabs normalized to 4
            _ => break,
        }
    }
    w
}

fn tokenize_line(
    s: &str,
    lineno: usize,
    out: &mut Vec<(Tok, usize)>,
) -> Result<(), LexError> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' {
            let mut buf = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    // pass escapes through to C++ untouched
                    buf.push(chars[i]);
                    buf.push(chars[i + 1]);
                    i += 2;
                } else {
                    buf.push(chars[i]);
                    i += 1;
                }
            }
            if i >= chars.len() {
                return Err(LexError {
                    line: lineno,
                    msg: "unterminated string".into(),
                });
            }
            i += 1; // closing quote
            out.push((Tok::Str(buf), lineno));
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let lit: String = chars[start..i].iter().collect();
            let v: i64 = lit.parse().map_err(|_| LexError {
                line: lineno,
                msg: format!("integer literal out of range: {}", lit),
            })?;
            out.push((Tok::Int(v), lineno));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            out.push((Tok::Ident(word), lineno));
            continue;
        }

        // Operators / punctuation. Try two-char first.
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        if matches!(
            two.as_str(),
            "==" | "!=" | "<=" | ">=" | "//" | "->"
        ) {
            out.push((Tok::Op(two), lineno));
            i += 2;
            continue;
        }
        let one = c.to_string();
        if "+-*/%<>=()[]:,.".contains(c) {
            out.push((Tok::Op(one), lineno));
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
