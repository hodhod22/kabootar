//! Kv8 lexer — JS-subset tokens.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Number(f64),
    String(String),
    Regex(String, String),
    True,
    False,
    Null,
    Undefined,
    Let,
    Var,
    Const,
    Return,
    If,
    Else,
    For,
    While,
    Do,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    Try,
    Catch,
    Finally,
    In,
    Import,
    Export,
    From,
    Instanceof,
    Function,
    Class,
    Extends,
    Static,
    Super,
    New,
    Async,
    Await,
    FatArrow,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Dot,
    Ellipsis,
    Comma,
    Colon,
    Semicolon,
    Question,
    Eq,
    EqEq,
    Ne,
    StrictEq,
    StrictNe,
    Typeof,
    Void,
    Throw,
    Delete,
    This,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    AndAnd,
    OrOr,
    NullishCoalesce,
    OptionalDot,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    Ushr,
    PlusPlus,
    MinusMinus,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    AmpEq,
    PipeEq,
    ShlEq,
    ShrEq,
    UshrEq,
    Template(Vec<TemplateSegment>),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateSegment {
    Lit(String),
    Expr(String),
}

fn read_balanced_brace_expr(input: &str, start: usize) -> Result<(String, usize), String> {
    let bytes = input.as_bytes();
    let mut depth = 1usize;
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return Ok((input[start..i].to_string(), i + 1));
            }
        } else if c == b'"' || c == b'\'' {
            let quote = c;
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
        } else if c == b'`' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'`' {
                if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                    i += 2;
                    let (_, ni) = read_balanced_brace_expr(input, i)?;
                    i = ni;
                    continue;
                }
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
        }
        i += 1;
    }
    Err("unterminated template expression".into())
}

fn read_template_literal(input: &str, start: usize) -> Result<(Vec<TemplateSegment>, usize), String> {
    let bytes = input.as_bytes();
    let mut i = start + 1;
    let mut segments = Vec::new();
    let mut lit = String::new();
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'`' {
            if !lit.is_empty() || segments.is_empty() {
                segments.push(TemplateSegment::Lit(lit));
            }
            return Ok((segments, i + 1));
        }
        if c == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            segments.push(TemplateSegment::Lit(lit));
            lit = String::new();
            i += 2;
            let (expr_src, ni) = read_balanced_brace_expr(input, i)?;
            segments.push(TemplateSegment::Expr(expr_src));
            i = ni;
            continue;
        }
        if c == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            lit.push(match next {
                b'n' => '\n',
                b't' => '\t',
                b'r' => '\r',
                b'`' => '`',
                b'$' => '$',
                b'\\' => '\\',
                other => other as char,
            });
            i += 2;
            continue;
        }
        lit.push(c as char);
        i += 1;
    }
    Err("unterminated template literal".into())
}

fn can_begin_regex(prev: Option<&Token>) -> bool {
    match prev {
        None => true,
        Some(
            Token::LParen
            | Token::LBrace
            | Token::Comma
            | Token::Colon
            | Token::Semicolon
            | Token::Eq
            | Token::Return
            | Token::FatArrow
            | Token::Question
            | Token::OrOr
            | Token::AndAnd
            | Token::NullishCoalesce
            | Token::OptionalDot
            | Token::Amp
            | Token::Pipe
            | Token::Caret
            | Token::Tilde
            | Token::Shl
            | Token::Shr
            | Token::Ushr
            | Token::Bang
            | Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Percent
            | Token::Let
            | Token::Var
            | Token::Const
            |             Token::Throw
            | Token::Case
            | Token::In
            | Token::Typeof
            | Token::Void
            | Token::PlusPlus
            | Token::MinusMinus
            | Token::PlusEq
            | Token::MinusEq
            | Token::StarEq
            | Token::SlashEq,
        ) => true,
        _ => false,
    }
}

fn read_regex(input: &str, start: usize) -> Result<(String, String, usize), String> {
    let bytes = input.as_bytes();
    let mut i = start + 1;
    let mut escaped = false;
    while i < bytes.len() {
        let c = bytes[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if c == b'\\' {
            escaped = true;
            i += 1;
            continue;
        }
        if c == b'/' {
            let pattern = input[start + 1..i].to_string();
            i += 1;
            let flags_start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            let flags = input[flags_start..i].to_string();
            return Ok((pattern, flags, i));
        }
        if c == b'\n' {
            return Err("unterminated regex".into());
        }
        i += 1;
    }
    Err("unterminated regex".into())
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
        if i + 2 < bytes.len() && &input[i..i + 3] == "..." {
            out.push(Token::Ellipsis);
            i += 3;
            continue;
        }
        if i + 3 < bytes.len() {
            let four = &input[i..i + 4];
            if four == ">>>=" {
                out.push(Token::UshrEq);
                i += 4;
                continue;
            }
        }
        if i + 2 < bytes.len() {
            let three = &input[i..i + 3];
            match three {
                "===" => {
                    out.push(Token::StrictEq);
                    i += 3;
                    continue;
                }
                "!==" => {
                    out.push(Token::StrictNe);
                    i += 3;
                    continue;
                }
                ">>>" => {
                    out.push(Token::Ushr);
                    i += 3;
                    continue;
                }
                "<<=" => {
                    out.push(Token::ShlEq);
                    i += 3;
                    continue;
                }
                ">>=" => {
                    out.push(Token::ShrEq);
                    i += 3;
                    continue;
                }
                _ => {}
            }
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
                "??" => {
                    out.push(Token::NullishCoalesce);
                    i += 2;
                    continue;
                }
                "?." => {
                    out.push(Token::OptionalDot);
                    i += 2;
                    continue;
                }
                ">>" => {
                    out.push(Token::Shr);
                    i += 2;
                    continue;
                }
                "<<" => {
                    out.push(Token::Shl);
                    i += 2;
                    continue;
                }
                "&=" => {
                    out.push(Token::AmpEq);
                    i += 2;
                    continue;
                }
                "|=" => {
                    out.push(Token::PipeEq);
                    i += 2;
                    continue;
                }
                "=>" => {
                    out.push(Token::FatArrow);
                    i += 2;
                    continue;
                }
                "+=" => {
                    out.push(Token::PlusEq);
                    i += 2;
                    continue;
                }
                "-=" => {
                    out.push(Token::MinusEq);
                    i += 2;
                    continue;
                }
                "*=" => {
                    out.push(Token::StarEq);
                    i += 2;
                    continue;
                }
                "/=" => {
                    out.push(Token::SlashEq);
                    i += 2;
                    continue;
                }
                "++" => {
                    out.push(Token::PlusPlus);
                    i += 2;
                    continue;
                }
                "--" => {
                    out.push(Token::MinusMinus);
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
            b'[' => {
                out.push(Token::LBracket);
                i += 1;
            }
            b']' => {
                out.push(Token::RBracket);
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
            b':' => {
                out.push(Token::Colon);
                i += 1;
            }
            b';' => {
                out.push(Token::Semicolon);
                i += 1;
            }
            b'?' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'?' {
                    out.push(Token::NullishCoalesce);
                    i += 2;
                } else if i + 1 < bytes.len() && bytes[i + 1] == b'.' {
                    out.push(Token::OptionalDot);
                    i += 2;
                } else {
                    out.push(Token::Question);
                    i += 1;
                }
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
            b'%' => {
                out.push(Token::Percent);
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
            b'&' => {
                out.push(Token::Amp);
                i += 1;
            }
            b'|' => {
                out.push(Token::Pipe);
                i += 1;
            }
            b'^' => {
                out.push(Token::Caret);
                i += 1;
            }
            b'~' => {
                out.push(Token::Tilde);
                i += 1;
            }
            b'/' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                } else if can_begin_regex(out.last()) {
                    let (pattern, flags, end) = read_regex(input, i)?;
                    out.push(Token::Regex(pattern, flags));
                    i = end;
                } else {
                    out.push(Token::Slash);
                    i += 1;
                }
            }
            b'`' => {
                let (segments, end) = read_template_literal(input, i)?;
                out.push(Token::Template(segments));
                i = end;
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
                if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                    i += 1;
                    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                        i += 1;
                    }
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let n: f64 = String::from_utf8_lossy(&bytes[start..i])
                    .parse()
                    .map_err(|_| "invalid number")?;
                out.push(Token::Number(n));
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$' => {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
                {
                    i += 1;
                }
                let word = String::from_utf8_lossy(&bytes[start..i]).to_string();
                out.push(match word.as_str() {
                    "let" => Token::Let,
                    "var" => Token::Var,
                    "const" => Token::Const,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "for" => Token::For,
                    "while" => Token::While,
                    "do" => Token::Do,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "switch" => Token::Switch,
                    "case" => Token::Case,
                    "default" => Token::Default,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "finally" => Token::Finally,
                    "in" => Token::In,
                    "import" => Token::Import,
                    "export" => Token::Export,
                    "from" => Token::From,
                    "instanceof" => Token::Instanceof,
                    "function" => Token::Function,
                    "class" => Token::Class,
                    "extends" => Token::Extends,
                    "static" => Token::Static,
                    "super" => Token::Super,
                    "new" => Token::New,
                    "async" => Token::Async,
                    "await" => Token::Await,
                    "typeof" => Token::Typeof,
                    "void" => Token::Void,
                    "throw" => Token::Throw,
                    "delete" => Token::Delete,
                    "this" => Token::This,
                    "true" => Token::True,
                    "false" => Token::False,
                    "null" => Token::Null,
                    "undefined" => Token::Undefined,
                    _ => Token::Ident(word),
                });
            }
            _ => return Err(format!("unexpected char: {}", c as char)),
        }
    }
    out.push(Token::Eof);
    Ok(out)
}
