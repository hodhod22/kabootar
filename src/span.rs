#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: u32,
    pub column: u32,
    pub len: u32,
}

impl Span {
    pub fn new(line: u32, column: u32, len: u32) -> Self {
        Self {
            line,
            column,
            len: len.max(1),
        }
    }

    pub fn unknown() -> Self {
        Self {
            line: 1,
            column: 1,
            len: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (line {}, col {})",
            self.message, self.span.line, self.span.column
        )
    }
}
