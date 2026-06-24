//! Kv8 lexer — JS-subset tokens.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Number(f64),
    String(String),
    True,
    False,
    Null,
    Let,
    Const,
    Return,
    If,
    Else,
    For,
    Function,
    FatArrow,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Dot,
    Comma,
    Semicolon,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    AndAnd,
    OrOr,
    Eof,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if i + 1 < bytes.len() {
            let two = &input[i..i + 2];
            match two {
                "==" => {
                    out.push(Token::EqEq);
                    i += 2;
                    continue;
                }
                "!=" => {
                    out.push(Token::Ne);
                    i += 2;
                    continue;
                }
                "<=" => {
                    out.push(Token::Le);
                    i += 2;
                    continue;
                }
                ">=" => {
                    out.push(Token::Ge);
                    i += 2;
                    continue;
                }
                "&&" => {
                    out.push(Token::AndAnd);
                    i += 2;
                    continue;
                }
                "||" => {
                    out.push(Token::OrOr);
                    i += 2;
                    continue;
                }
                "=>" => {
                    out.push(Token::FatArrow);
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        match c {
            b'(' => {
                out.push(Token::LParen);
                i += 1;
            }
            b')' => {
                out.push(Token::RParen);
                i += 1;
            }
            b'{' => {
                out.push(Token::LBrace);
                i += 1;
            }
            b'}' => {
                out.push(Token::RBrace);
                i += 1;
            }
            b'.' => {
                out.push(Token::Dot);
                i += 1;
            }
            b',' => {
                out.push(Token::Comma);
                i += 1;
            }
            b';' => {
                out.push(Token::Semicolon);
                i += 1;
            }
            b'=' => {
                out.push(Token::Eq);
                i += 1;
            }
            b'+' => {
                out.push(Token::Plus);
                i += 1;
            }
            b'-' => {
                out.push(Token::Minus);
                i += 1;
            }
            b'*' => {
                out.push(Token::Star);
                i += 1;
            }
            b'!' => {
                out.push(Token::Bang);
                i += 1;
            }
            b'<' => {
                out.push(Token::Lt);
                i += 1;
            }
            b'>' => {
                out.push(Token::Gt);
                i += 1;
            }
            b'/' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                } else {
                    out.push(Token::Slash);
                    i += 1;
                }
            }
            b'"' | b'\'' => {
                let quote = c;
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err("unterminated string".into());
                }
                let s = String::from_utf8_lossy(&bytes[start..i]).to_string();
                i += 1;
                out.push(Token::String(s));
            }
            b'0'..=b'9' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                let n: f64 = String::from_utf8_lossy(&bytes[start..i])
                    .parse()
                    .map_err(|_| "invalid number")?;
                out.push(Token::Number(n));
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$' => {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = String::from_utf8_lossy(&bytes[start..i]).to_string();
                out.push(match word.as_str() {
                    "let" => Token::Let,
                    "const" => Token::Const,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "for" => Token::For,
                    "function" => Token::Function,
                    "true" => Token::True,
                    "false" => Token::False,
                    "null" => Token::Null,
                    _ => Token::Ident(word),
                });
            }
            _ => return Err(format!("unexpected char: {}", c as char)),
        }
    }
    out.push(Token::Eof);
    Ok(out)
}
