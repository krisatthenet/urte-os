//! Hand-written lexer for the URTE Pipeline Language.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // keywords
    Pipeline,
    Scale,
    Stage,
    Guard,
    If,
    Emit,
    // literals / names
    Ident(String),
    Str(String),
    Num(f64),
    // punctuation / operators
    LBrace,
    RBrace,
    Semi,
    Gt,
    Ge,
    Lt,
    Le,
    EqEq,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub line: usize,
    pub msg: String,
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut out = Vec::new();
    let mut line = 1usize;
    let bytes = src.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            '\n' => {
                line += 1;
                i += 1;
            }
            c if c.is_whitespace() => i += 1,
            '/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                // line comment
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            '{' => {
                out.push(Token { tok: Tok::LBrace, line });
                i += 1;
            }
            '}' => {
                out.push(Token { tok: Tok::RBrace, line });
                i += 1;
            }
            ';' => {
                out.push(Token { tok: Tok::Semi, line });
                i += 1;
            }
            '>' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Token { tok: Tok::Ge, line });
                    i += 2;
                } else {
                    out.push(Token { tok: Tok::Gt, line });
                    i += 1;
                }
            }
            '<' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Token { tok: Tok::Le, line });
                    i += 2;
                } else {
                    out.push(Token { tok: Tok::Lt, line });
                    i += 1;
                }
            }
            '=' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Token { tok: Tok::EqEq, line });
                    i += 2;
                } else {
                    return Err(LexError { line, msg: "expected '==' ".into() });
                }
            }
            '"' => {
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && bytes[j] != b'"' {
                    if bytes[j] == b'\n' {
                        return Err(LexError { line, msg: "unterminated string".into() });
                    }
                    j += 1;
                }
                if j >= bytes.len() {
                    return Err(LexError { line, msg: "unterminated string".into() });
                }
                let s = src[start..j].to_string();
                out.push(Token { tok: Tok::Str(s), line });
                i = j + 1;
            }
            c if c.is_ascii_digit() || (c == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit()) => {
                let start = i;
                while i < bytes.len()
                    && ((bytes[i] as char).is_ascii_digit() || bytes[i] == b'.')
                {
                    i += 1;
                }
                let text = &src[start..i];
                match text.parse::<f64>() {
                    Ok(n) => out.push(Token { tok: Tok::Num(n), line }),
                    Err(_) => return Err(LexError { line, msg: format!("bad number '{text}'") }),
                }
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < bytes.len()
                    && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_')
                {
                    i += 1;
                }
                let word = &src[start..i];
                let tok = match word {
                    "pipeline" => Tok::Pipeline,
                    "scale" => Tok::Scale,
                    "stage" => Tok::Stage,
                    "guard" => Tok::Guard,
                    "if" => Tok::If,
                    "emit" => Tok::Emit,
                    _ => Tok::Ident(word.to_string()),
                };
                out.push(Token { tok, line });
            }
            other => {
                return Err(LexError { line, msg: format!("unexpected character '{other}'") });
            }
        }
    }

    out.push(Token { tok: Tok::Eof, line });
    Ok(out)
}
