use crate::span::{Span, Spanned};

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Text(String),
    Expr(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Fn,
    Let,
    Const,
    Return,
    If,
    Else,
    Match,
    Some,
    None,
    Ok,
    Err,
    While,
    For,
    In,
    Of,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Try,
    Catch,
    Finally,
    Async,
    Await,
    Yield,
    Delete,
    Throw,
    Raise,
    Pass,
    Assert,
    With,
    Using,
    As,
    Is,
    Not,
    Extends,
    Interface,
    Implements,
    Class,
    Enum,
    Import,
    Pub,
    This,
    Super,
    Null,
    Undefined,
    NaN,
    Identifier(String),
    PrivateName(String),
    Number(i64),
    Float(f64),
    BigInt(String),
    String(String),
    Template(Vec<TemplatePart>),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Plus,
    PlusEq,
    Minus,
    MinusEq,
    Star,
    StarStar,
    StarEq,
    Slash,
    SlashEq,
    Percent,
    PercentEq,
    Bang,
    Question,
    QuestionDot,
    QuestionQuestion,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    Ushr,
    Fallthrough,
    Do,
    Semicolon,
    Comma,
    Dot,
    DotDotDot,
    Colon,
    FatArrow,
    Eof,
    True,
    False,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub line: u32,
    pub column: u32,
}

impl LexError {
    fn here(lexer: &Lexer, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: lexer.line,
            column: lexer.column,
        }
    }
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (line {}, col {})", self.message, self.line, self.column)
    }
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        if c == Some('\n') {
            self.line += 1;
            self.column = 1;
        } else if c.is_some() {
            self.column += 1;
        }
        self.pos += 1;
        c
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if !c.is_whitespace() {
                break;
            }
            self.next();
        }
    }

    fn span_from(&self, start_line: u32, start_column: u32) -> Span {
        let len = if self.line == start_line {
            self.column.saturating_sub(start_column)
        } else {
            1
        };
        Span::new(start_line, start_column, len)
    }

    fn read_identifier(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.next();
            } else {
                break;
            }
        }
        s
    }

    fn read_numeric_token(&mut self) -> Token {
        let mut digits = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                digits.push(c);
                self.next();
            } else {
                break;
            }
        }

        if self.peek() == Some('n') {
            self.next();
            if digits.is_empty() {
                digits.push('0');
            }
            return Token::BigInt(digits);
        }

        if self.peek() == Some('.') {
            let int_part: i64 = digits.parse().unwrap_or(0);
            self.next();
            let mut frac: f64 = 0.0;
            let mut divisor = 10.0;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    frac += (c as u32 - '0' as u32) as f64 / divisor;
                    divisor *= 10.0;
                    self.next();
                } else {
                    break;
                }
            }
            return Token::Float(int_part as f64 + frac);
        }

        let int_part: i64 = if digits.is_empty() {
            0
        } else {
            digits.parse().unwrap_or(0)
        };
        Token::Number(int_part)
    }

    fn skip_block_comment(&mut self) {
        while let Some(c) = self.next() {
            if c == '*' && self.peek() == Some('/') {
                self.next();
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                self.next();
                break;
            }
            self.next();
        }
    }

    fn read_string_with_delimiter(&mut self, delim: char) -> String {
        self.next();
        let mut s = String::new();
        while let Some(c) = self.next() {
            if c == delim {
                break;
            }
            s.push(c);
        }
        s
    }

    fn read_template(&mut self) -> Result<Token, LexError> {
        self.next();
        let mut parts: Vec<TemplatePart> = Vec::new();
        let mut buf = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(LexError::here(self, "Unterminated template literal"));
                }
                Some('`') => {
                    self.next();
                    if !buf.is_empty() {
                        parts.push(TemplatePart::Text(buf));
                    }
                    if parts.is_empty() {
                        return Ok(Token::String(String::new()));
                    }
                    if parts.len() == 1 {
                        if let TemplatePart::Text(s) = parts.remove(0) {
                            return Ok(Token::String(s));
                        }
                    }
                    return Ok(Token::Template(parts));
                }
                Some('$') if self.chars.get(self.pos + 1) == Some(&'{') => {
                    self.next();
                    self.next();
                    if !buf.is_empty() {
                        parts.push(TemplatePart::Text(buf));
                        buf = String::new();
                    }
                    let mut depth = 1i32;
                    let mut expr = String::new();
                    while let Some(c) = self.next() {
                        if c == '{' {
                            depth += 1;
                        } else if c == '}' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        expr.push(c);
                    }
                    if depth != 0 {
                        return Err(LexError::here(self, "Unterminated ${} in template literal"));
                    }
                    parts.push(TemplatePart::Expr(expr));
                }
                Some(c) => {
                    buf.push(c);
                    self.next();
                }
            }
        }
    }

    fn read_string(&mut self) -> String {
        self.read_string_with_delimiter('"')
    }

    fn scan_token(&mut self) -> Result<Token, LexError> {
        match self.peek() {
            None => Ok(Token::Eof),
            Some(c) => match c {
                '(' => {
                    self.next();
                    Ok(Token::LParen)
                }
                ')' => {
                    self.next();
                    Ok(Token::RParen)
                }
                '{' => {
                    self.next();
                    Ok(Token::LBrace)
                }
                '}' => {
                    self.next();
                    Ok(Token::RBrace)
                }
                '[' => {
                    self.next();
                    Ok(Token::LBracket)
                }
                ']' => {
                    self.next();
                    Ok(Token::RBracket)
                }
                '+' => {
                    self.next();
                    if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::PlusEq)
                    } else {
                        Ok(Token::Plus)
                    }
                }
                '-' => {
                    self.next();
                    if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::MinusEq)
                    } else {
                        Ok(Token::Minus)
                    }
                }
                '*' => {
                    self.next();
                    if self.peek() == Some('*') {
                        self.next();
                        Ok(Token::StarStar)
                    } else if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::StarEq)
                    } else {
                        Ok(Token::Star)
                    }
                }
                '/' => {
                    self.next();
                    if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::SlashEq)
                    } else {
                        Ok(Token::Slash)
                    }
                }
                '%' => {
                    self.next();
                    if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::PercentEq)
                    } else {
                        Ok(Token::Percent)
                    }
                }
                '?' => {
                    self.next();
                    if self.peek() == Some('?') {
                        self.next();
                        Ok(Token::QuestionQuestion)
                    } else if self.peek() == Some('.') {
                        self.next();
                        Ok(Token::QuestionDot)
                    } else {
                        Ok(Token::Question)
                    }
                }
                '#' => {
                    self.next();
                    let mut name = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            name.push(c);
                            self.next();
                        } else {
                            break;
                        }
                    }
                    if name.is_empty() {
                        return Err(LexError::here(self, "Expected identifier after #"));
                    }
                    Ok(Token::PrivateName(name))
                }
                '=' => {
                    self.next();
                    if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::EqEq)
                    } else if self.peek() == Some('>') {
                        self.next();
                        Ok(Token::FatArrow)
                    } else {
                        Ok(Token::Eq)
                    }
                }
                '!' => {
                    self.next();
                    if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::Ne)
                    } else {
                        Ok(Token::Bang)
                    }
                }
                '<' => {
                    self.next();
                    if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::Le)
                    } else if self.peek() == Some('<') {
                        self.next();
                        Ok(Token::Shl)
                    } else {
                        Ok(Token::Lt)
                    }
                }
                '>' => {
                    self.next();
                    if self.peek() == Some('=') {
                        self.next();
                        Ok(Token::Ge)
                    } else if self.peek() == Some('>') {
                        self.next();
                        if self.peek() == Some('>') {
                            self.next();
                            Ok(Token::Ushr)
                        } else {
                            Ok(Token::Shr)
                        }
                    } else {
                        Ok(Token::Gt)
                    }
                }
                '&' => {
                    self.next();
                    if self.peek() == Some('&') {
                        self.next();
                        Ok(Token::AndAnd)
                    } else {
                        Ok(Token::Amp)
                    }
                }
                '|' => {
                    self.next();
                    if self.peek() == Some('|') {
                        self.next();
                        Ok(Token::OrOr)
                    } else {
                        Ok(Token::Pipe)
                    }
                }
                '^' => {
                    self.next();
                    Ok(Token::Caret)
                }
                '~' => {
                    self.next();
                    Ok(Token::Tilde)
                }
                ';' => {
                    self.next();
                    Ok(Token::Semicolon)
                }
                ',' => {
                    self.next();
                    Ok(Token::Comma)
                }
                '.' => {
                    self.next();
                    if self.peek() == Some('.') {
                        self.next();
                        if self.peek() == Some('.') {
                            self.next();
                            Ok(Token::DotDotDot)
                        } else {
                            Err(LexError::here(self, "Expected `...` spread operator"))
                        }
                    } else {
                        Ok(Token::Dot)
                    }
                }
                ':' => {
                    self.next();
                    Ok(Token::Colon)
                }
                '_' => {
                    self.next();
                    Ok(Token::Identifier("_".to_string()))
                }
                '"' => Ok(Token::String(self.read_string())),
                '`' => self.read_template(),
                c if c.is_alphabetic() => {
                    let id = self.read_identifier();
                    Ok(match id.as_str() {
                        "fn" => Token::Fn,
                        "let" => Token::Let,
                        "const" => Token::Const,
                        "return" => Token::Return,
                        "if" => Token::If,
                        "else" => Token::Else,
                        "match" => Token::Match,
                        "Some" => Token::Some,
                        "None" => Token::None,
                        "Ok" => Token::Ok,
                        "Err" => Token::Err,
                        "while" => Token::While,
                        "do" => Token::Do,
                        "for" => Token::For,
                        "in" => Token::In,
                        "of" => Token::Of,
                        "switch" => Token::Switch,
                        "case" => Token::Case,
                        "default" => Token::Default,
                        "break" => Token::Break,
                        "fallthrough" => Token::Fallthrough,
                        "continue" => Token::Continue,
                        "async" => Token::Async,
                        "await" => Token::Await,
                        "yield" => Token::Yield,
                        "delete" => Token::Delete,
                        "throw" => Token::Throw,
                        "raise" => Token::Raise,
                        "pass" => Token::Pass,
                        "assert" => Token::Assert,
                        "with" => Token::With,
                        "using" => Token::Using,
                        "as" => Token::As,
                        "is" => Token::Is,
                        "not" => Token::Not,
                        "extends" => Token::Extends,
                        "interface" => Token::Interface,
                        "implements" => Token::Implements,
                        "try" => Token::Try,
                        "catch" => Token::Catch,
                        "finally" => Token::Finally,
                        "class" => Token::Class,
                        "enum" => Token::Enum,
                        "import" => Token::Import,
                        "pub" => Token::Pub,
                        "self" => Token::This,
                        "this" => {
                            return Err(LexError::here(
                                self,
                                "Kabootar uses `self` for the current instance, not JavaScript `this`",
                            ));
                        }
                        "super" => Token::Super,
                        "null" => Token::Null,
                        "undefined" => Token::Undefined,
                        "NaN" => Token::NaN,
                        "true" => Token::True,
                        "false" => Token::False,
                        _ => Token::Identifier(id),
                    })
                }
                c if c.is_ascii_digit() => Ok(self.read_numeric_token()),
                _ => Err(LexError::here(self, format!("Unknown character: {}", c))),
            },
        }
    }

    pub fn next_token(&mut self) -> Result<Spanned<Token>, LexError> {
        loop {
            self.skip_whitespace();
            if self.peek() == Some('/') && self.chars.get(self.pos + 1) == Some(&'/') {
                self.next();
                self.next();
                self.skip_line_comment();
                continue;
            }
            if self.peek() == Some('/') && self.chars.get(self.pos + 1) == Some(&'*') {
                self.next();
                self.next();
                self.skip_block_comment();
                continue;
            }
            break;
        }
        let start_line = self.line;
        let start_column = self.column;
        let value = self.scan_token()?;
        Ok(Spanned::new(
            value,
            self.span_from(start_line, start_column),
        ))
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Spanned<Token>>, LexError> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let spanned = lexer.next_token()?;
        if spanned.value == Token::Eof {
            break;
        }
        tokens.push(spanned);
    }
    Ok(tokens)
}

pub fn eof_span_after(tokens: &[Spanned<Token>]) -> Span {
    if let Some(last) = tokens.last() {
        Span::new(last.span.line, last.span.column + last.span.len, 1)
    } else {
        Span::unknown()
    }
}
