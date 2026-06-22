//! SQL lexer. Keywords are case-insensitive; string literals use single quotes.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // keywords
    Select,
    Insert,
    Into,
    Values,
    From,
    Where,
    And,
    Create,
    Table,
    // literals / names
    Ident(String),
    Num(f64),
    Str(String),
    // punctuation / operators
    Star,
    Comma,
    LParen,
    RParen,
    Semi,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SqlLexError {
    pub pos: usize,
    pub msg: String,
}

pub fn lex(src: &str) -> Result<Vec<Tok>, SqlLexError> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut out = Vec::new();

    while i < b.len() {
        let c = b[i] as char;
        match c {
            c if c.is_whitespace() => i += 1,
            '*' => {
                out.push(Tok::Star);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            ';' => {
                out.push(Tok::Semi);
                i += 1;
            }
            '=' => {
                out.push(Tok::Eq);
                i += 1;
            }
            '!' => {
                if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Ne);
                    i += 2;
                } else {
                    return Err(SqlLexError { pos: i, msg: "expected '!='".into() });
                }
            }
            '<' => {
                if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Le);
                    i += 2;
                } else if i + 1 < b.len() && b[i + 1] == b'>' {
                    out.push(Tok::Ne);
                    i += 2;
                } else {
                    out.push(Tok::Lt);
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < b.len() && b[i + 1] == b'=' {
                    out.push(Tok::Ge);
                    i += 2;
                } else {
                    out.push(Tok::Gt);
                    i += 1;
                }
            }
            '\'' => {
                let start = i + 1;
                let mut j = start;
                while j < b.len() && b[j] != b'\'' {
                    j += 1;
                }
                if j >= b.len() {
                    return Err(SqlLexError { pos: i, msg: "unterminated string".into() });
                }
                out.push(Tok::Str(src[start..j].to_string()));
                i = j + 1;
            }
            c if c.is_ascii_digit()
                || (c == '-' && i + 1 < b.len() && (b[i + 1] as char).is_ascii_digit()) =>
            {
                let start = i;
                if c == '-' {
                    i += 1;
                }
                while i < b.len() && ((b[i] as char).is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                let text = &src[start..i];
                match text.parse::<f64>() {
                    Ok(n) => out.push(Tok::Num(n)),
                    Err(_) => return Err(SqlLexError { pos: start, msg: format!("bad number '{text}'") }),
                }
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < b.len() && ((b[i] as char).is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let word = &src[start..i];
                let tok = match word.to_ascii_uppercase().as_str() {
                    "SELECT" => Tok::Select,
                    "INSERT" => Tok::Insert,
                    "INTO" => Tok::Into,
                    "VALUES" => Tok::Values,
                    "FROM" => Tok::From,
                    "WHERE" => Tok::Where,
                    "AND" => Tok::And,
                    "CREATE" => Tok::Create,
                    "TABLE" => Tok::Table,
                    _ => Tok::Ident(word.to_string()),
                };
                out.push(tok);
            }
            other => {
                return Err(SqlLexError { pos: i, msg: format!("unexpected character '{other}'") });
            }
        }
    }

    out.push(Tok::Eof);
    Ok(out)
}
