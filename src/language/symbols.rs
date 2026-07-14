use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Function,
    Class,
    Method,
    Field,
    Parameter,
    TypeParam,
    Module,
    Imported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    /// For imported symbols: definition span inside the module source.
    pub module_span: Option<Span>,
    pub module_name: Option<String>,
}

impl Symbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind, span: Span) -> Self {
        Self {
            name: name.into(),
            kind,
            span,
            module_span: None,
            module_name: None,
        }
    }

    pub fn imported(
        name: impl Into<String>,
        module: impl Into<String>,
        import_span: Span,
        def_span: Span,
    ) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Imported,
            span: import_span,
            module_span: Some(def_span),
            module_name: Some(module.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionSite {
    pub line: u32,
    pub column: u32,
    pub len: u32,
}

pub fn definition_before<'a>(
    symbols: &'a [Symbol],
    name: &str,
    use_line: u32,
    use_column: u32,
) -> Option<&'a Symbol> {
    symbols
        .iter()
        .filter(|sym| sym.name == name)
        .filter(|sym| position_before(sym.span, use_line, use_column))
        .max_by_key(|sym| (sym.span.line, sym.span.column))
}

pub fn resolve_definition(sym: &Symbol) -> DefinitionSite {
    if let (Some(module_span), Some(_)) = (sym.module_span, sym.module_name.as_ref()) {
        DefinitionSite {
            line: module_span.line,
            column: module_span.column,
            len: module_span.len,
        }
    } else {
        DefinitionSite {
            line: sym.span.line,
            column: sym.span.column,
            len: sym.span.len,
        }
    }
}

fn position_before(span: Span, line: u32, column: u32) -> bool {
    span.line < line || (span.line == line && span.column < column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_latest_definition_before_use() {
        let symbols = vec![
            Symbol::new("x", SymbolKind::Variable, Span::new(1, 5, 1)),
            Symbol::new("x", SymbolKind::Variable, Span::new(3, 5, 1)),
        ];
        let found = definition_before(&symbols, "x", 4, 10).unwrap();
        assert_eq!(found.span.line, 3);
    }
}
