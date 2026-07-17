use super::ast::*;
use super::lexer::{eof_span_after, tokenize, TemplatePart, Token};
use crate::language::{Symbol, SymbolKind};
use crate::span::{ParseError, Span, Spanned};

pub struct Parser {
    tokens: Vec<Spanned<Token>>,
    pos: usize,
    symbols: Vec<Symbol>,
    /// Line of the most recently consumed primary token (for newline-safe postfix).
    last_primary_line: u32,
}

impl Parser {
    pub fn new(tokens: Vec<Spanned<Token>>) -> Self {
        Self {
            tokens,
            pos: 0,
            symbols: Vec::new(),
            last_primary_line: 1,
        }
    }

    pub fn with_eof(mut tokens: Vec<Spanned<Token>>) -> Self {
        let span = eof_span_after(&tokens);
        tokens.push(Spanned::new(Token::Eof, span));
        Self::new(tokens)
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    fn record(&mut self, name: impl Into<String>, kind: SymbolKind, span: Span) {
        self.symbols.push(Symbol::new(name, kind, span));
    }

    fn parse_member_name(&mut self) -> Result<String, ParseError> {
        let spanned = self.bump().ok_or_else(|| self.err("Expected member name"))?;
        match spanned.value {
            Token::Identifier(s) => Ok(s),
            Token::PrivateName(s) => Ok(format!("#{s}")),
            Token::Is => Ok("is".into()),
            Token::For => Ok("for".into()),
            Token::Return => Ok("return".into()),
            Token::Throw => Ok("throw".into()),
            Token::Try => Ok("try".into()),
            Token::None => Ok("None".into()),
            Token::Some => Ok("Some".into()),
            Token::Ok => Ok("Ok".into()),
            Token::Err => Ok("Err".into()),
            _ => Err(Self::err_at(spanned.span, "Expected member name")),
        }
    }

    fn parse_enum_variant_name(&mut self) -> Result<String, ParseError> {
        let spanned = self.bump().ok_or_else(|| self.err("Expected variant name"))?;
        Ok(match spanned.value {
            Token::Identifier(s) => s,
            Token::None => "None".into(),
            Token::Some => "Some".into(),
            Token::Ok => "Ok".into(),
            Token::Err => "Err".into(),
            Token::True => "True".into(),
            Token::False => "False".into(),
            _ => return Err(Self::err_at(spanned.span, "Expected variant name")),
        })
    }

    fn parse_class_member_name(&mut self) -> Result<(String, bool), ParseError> {
        let spanned = self.bump().ok_or_else(|| self.err("Expected field or method name"))?;
        match spanned.value {
            Token::PrivateName(s) => Ok((format!("#{s}"), true)),
            Token::Identifier(s) => Ok((s, false)),
            _ => Err(Self::err_at(spanned.span, "Expected field or method name")),
        }
    }

    fn index_module_exports(&mut self, module: &str, import_span: Span) {
        let Some(src) = crate::modules::builtin_source(module) else {
            return;
        };
        let Ok(tokens) = tokenize(src) else {
            return;
        };
        let mut parser = Parser::with_eof(tokens);
        if parser.parse_program().is_err() {
            return;
        }
        for sym in parser.symbols() {
            if sym.kind == SymbolKind::Function {
                self.symbols.push(Symbol::imported(
                    sym.name.clone(),
                    module,
                    import_span,
                    sym.span,
                ));
            }
        }
    }

    fn peek(&self) -> Option<&Spanned<Token>> {
        self.tokens.get(self.pos)
    }

    fn peek_token(&self) -> Option<&Token> {
        self.peek().map(|t| &t.value)
    }

    fn bump(&mut self) -> Option<Spanned<Token>> {
        let t = self.peek().cloned();
        if t.is_some() {
        self.pos += 1;
        }
        t
    }

    fn at(&self, expected: Token) -> bool {
        self.peek_token() == Some(&expected)
    }

    fn can_start_yield_operand(&self) -> bool {
        matches!(
            self.peek_token(),
            Some(
                Token::Number(_)
                    | Token::BigInt(_)
                    | Token::Float(_)
                    | Token::String(_)
                    | Token::Template(_)
                    | Token::Identifier(_)
                    | Token::Some
                    | Token::None
                    | Token::Ok
                    | Token::Err
                    | Token::True
                    | Token::False
                    | Token::Null
                    | Token::Undefined
                    | Token::NaN
                    | Token::This
                    | Token::Super
                    | Token::Match
                    | Token::Try
                    | Token::If
                    | Token::While
                    | Token::For
                    | Token::Switch
                    | Token::Fn
                    | Token::Pub
                    | Token::Async
                    | Token::Await
                    | Token::Yield
                    | Token::Delete
                    | Token::Throw
                    | Token::Raise
                    | Token::LParen
                    | Token::LBrace
                    | Token::LBracket
                    | Token::Bang
                    | Token::Not
                    | Token::Minus
                    | Token::Tilde
            )
        )
    }

    fn err(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            span: self.peek().map(|t| t.span).unwrap_or(Span::unknown()),
        }
    }

    fn err_at(span: Span, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            span,
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if let Some(spanned) = self.peek() {
            if spanned.value == expected {
                self.pos += 1;
                Ok(())
            } else {
                Err(Self::err_at(
                    spanned.span,
                    format!("Expected {:?}, found {:?}", expected, spanned.value),
                ))
            }
        } else {
            Err(self.err(format!("Expected {:?}, found EOF", expected)))
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while !self.at(Token::Eof) {
            stmts.push(self.parse_stmt()?);
            if self.at(Token::Semicolon) {
                self.bump();
            }
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_token() {
            Some(Token::Let) => self.parse_let_stmt(false, false),
            Some(Token::Const) => self.parse_let_stmt(false, true),
            Some(Token::Pub) => self.parse_pub_stmt(),
            Some(Token::Class) => self.parse_class_or_struct_stmt(false),
            Some(Token::Struct) => self.parse_class_or_struct_stmt(true),
            Some(Token::Enum) => self.parse_enum_stmt(),
            Some(Token::Interface) | Some(Token::Trait) => self.parse_interface_stmt(),
            Some(Token::Import) => self.parse_import_stmt(false),
            Some(Token::Return) => self.parse_return_stmt(),
            Some(Token::Do) => Ok(Stmt::Expr(self.parse_do_while_loop()?)),
            Some(Token::While) => Ok(Stmt::Expr(self.parse_while_loop()?)),
            Some(Token::Switch) => Ok(Stmt::Expr(self.parse_switch()?)),
            Some(Token::For) => Ok(Stmt::Expr(self.parse_for_loop()?)),
            Some(Token::Break) => self.parse_break(),
            Some(Token::Fallthrough) => self.parse_fallthrough(),
            Some(Token::Continue) => self.parse_continue(),
            Some(Token::Pass) => {
                self.bump();
                Ok(Stmt::Expr(Expr::Pass))
            }
            Some(Token::Assert) => Ok(Stmt::Expr(self.parse_assert()?)),
            Some(Token::With) => Ok(Stmt::Expr(self.parse_with()?)),
            Some(Token::Using) => self.parse_using_stmt(),
            Some(Token::Async) | Some(Token::Fn) => {
                Ok(Stmt::Expr(self.parse_function()?))
            }
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    fn parse_assert(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Assert)?;
        let condition = self.parse_expr()?;
        let message = if self.at(Token::Comma) {
            self.bump();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        Ok(Expr::Assert {
            condition: Box::new(condition),
            message,
        })
    }

    fn parse_with(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::With)?;
        let value = self.parse_expr()?;
        self.expect(Token::As)?;
        let name = self.expect_identifier("binding name")?;
        let body = self.parse_block()?;
        Ok(Expr::With {
            name,
            value: Box::new(value),
            body: Box::new(body),
        })
    }

    fn parse_using_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Using)?;
        let name = self.expect_identifier("binding name")?;
        self.expect(Token::Eq)?;
        let init = self.parse_expr()?;
        Ok(Stmt::Using { name, init })
    }

    fn parse_pub_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Pub)?;
        match self.peek_token() {
            Some(Token::Let) => self.parse_let_stmt(true, false),
            Some(Token::Const) => self.parse_let_stmt(true, true),
            Some(Token::Fn) | Some(Token::Async) => {
                Ok(Stmt::Expr(self.parse_function_with_public(true)?))
            }
            Some(Token::Import) => self.parse_import_stmt(true),
            _ => Err(self.err("Expected `let`, `const`, `fn` or `import` after `pub`")),
        }
    }

    fn parse_let_stmt(&mut self, public: bool, immutable: bool) -> Result<Stmt, ParseError> {
        if immutable {
            self.expect(Token::Const)?;
        } else {
        self.expect(Token::Let)?;
        }
        let pattern = self.parse_binding_pattern()?;
        let init = if self.at(Token::Eq) {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Stmt::Let {
            pattern,
            init,
            public,
            immutable,
        })
    }

    fn parse_import_stmt(&mut self, public: bool) -> Result<Stmt, ParseError> {
        self.expect(Token::Import)?;
        let name = match self.bump() {
            Some(spanned) => match spanned.value {
                Token::String(s) => {
                    self.record(s.clone(), SymbolKind::Module, spanned.span);
                    self.index_module_exports(&s, spanned.span);
                    s
                }
                _ => {
                    return Err(Self::err_at(
                        spanned.span,
                        "import expects a module name string",
                    ))
                }
            },
            None => return Err(self.err("import expects a module name string")),
        };
        Ok(Stmt::Import {
            module: name,
            public,
        })
    }

    fn parse_import_expr_after_keyword(&mut self) -> Result<Expr, ParseError> {
        if self.at(Token::Dot) {
            self.bump();
            let member = self.parse_member_name()?;
            if member == "meta" {
                return Ok(Expr::ImportMeta);
            }
            return Err(self.err("import expression expects .meta or (specifier)"));
        }
        if self.at(Token::LParen) {
            self.bump();
            let spec = self.parse_expr()?;
            self.expect(Token::RParen)?;
            return Ok(Expr::DynamicImport(Box::new(spec)));
        }
        Err(self.err("import expression expects .meta or (specifier)"))
    }

    fn parse_import_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Import)?;
        self.parse_import_expr_after_keyword()
    }

    fn parse_interface_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Interface | Token::Trait => {}
                _ => return Err(Self::err_at(spanned.span, "Expected interface or trait")),
            },
            None => return Err(self.err("Expected interface or trait")),
        }
        let name = match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Identifier(s) => {
                    self.record(s.clone(), SymbolKind::Class, spanned.span);
                    s
                }
                _ => return Err(Self::err_at(spanned.span, "Expected interface/trait name")),
            },
            None => return Err(self.err("Expected interface/trait name")),
        };
        let mut type_params = Vec::new();
        if self.at(Token::Lt) && self.lookahead_type_params() {
            type_params = self.parse_type_param_list()?;
        }
        self.expect(Token::LBrace)?;
        let mut methods = Vec::new();
        let mut associated_types = Vec::new();
        while !self.at(Token::RBrace) && !self.at(Token::Eof) {
            if self.at_identifier("type") {
                associated_types.push(self.parse_interface_associated_type()?);
            } else {
                methods.push(self.parse_interface_method()?);
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::Interface {
            name,
            type_params,
            associated_types,
            methods,
        })
    }

    fn parse_interface_associated_type(&mut self) -> Result<String, ParseError> {
        // No Token::Type — parse identifier "type" as keyword.
        if !self.at_identifier("type") {
            return Err(self.err("Expected associated type declaration"));
        }
        self.bump();
        let name = self.expect_identifier("associated type name")?;
        self.expect(Token::Semicolon)?;
        Ok(name)
    }

    fn parse_interface_method(&mut self) -> Result<InterfaceMethod, ParseError> {
        self.expect(Token::Fn)?;
        let name = match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Identifier(s) => {
                    self.record(s.clone(), SymbolKind::Method, spanned.span);
                    s
                }
                _ => return Err(Self::err_at(spanned.span, "Expected method name")),
            },
            None => return Err(self.err("Expected method name")),
        };
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while !self.at(Token::RParen) {
            params.push(self.expect_identifier("parameter")?);
            if self.at(Token::Comma) {
                self.bump();
            }
        }
        self.expect(Token::RParen)?;
        let body = if self.at(Token::Semicolon) {
            self.bump();
            None
        } else if self.at(Token::LBrace) {
            Some(self.parse_block()?)
        } else {
            return Err(self.err("Expected `;` or method body after interface method"));
        };
        Ok(InterfaceMethod { name, params, body })
    }

    fn parse_enum_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Enum)?;
        let name = match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Identifier(s) => {
                    self.record(s.clone(), SymbolKind::Class, spanned.span);
                    s
                }
                _ => return Err(Self::err_at(spanned.span, "Expected enum name")),
            },
            None => return Err(self.err("Expected enum name")),
        };
        let mut type_params = Vec::new();
        if self.at(Token::Lt) && self.lookahead_type_params() {
            type_params = self.parse_type_param_list()?;
        }
        self.expect(Token::LBrace)?;
        let mut variants = Vec::new();
        while !self.at(Token::RBrace) && !self.at(Token::Eof) {
            if self.at(Token::Comma) {
                self.bump();
                continue;
            }
            let vname = self.parse_enum_variant_name()?;
            let fields = if self.at(Token::LParen) {
                self.bump();
                let mut fields = Vec::new();
                while !self.at(Token::RParen) {
                    fields.push(self.expect_identifier("field")?);
                    if self.at(Token::Comma) {
                        self.bump();
                    }
                }
                self.expect(Token::RParen)?;
                fields
            } else {
                Vec::new()
            };
            variants.push(EnumVariant {
                name: vname,
                fields,
            });
            if self.at(Token::Comma) {
                self.bump();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::Enum {
            name,
            type_params,
            variants,
        })
    }

    fn parse_class_or_struct_stmt(&mut self, is_struct: bool) -> Result<Stmt, ParseError> {
        if is_struct {
            self.expect(Token::Struct)?;
        } else {
            self.expect(Token::Class)?;
        }
        let name = match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Identifier(s) => {
                    self.record(s.clone(), SymbolKind::Class, spanned.span);
                    s
                }
                _ => {
                    return Err(Self::err_at(
                        spanned.span,
                        if is_struct {
                            "Expected struct name"
                        } else {
                            "Expected class name"
                        },
                    ))
                }
            },
            None => {
                return Err(self.err(if is_struct {
                    "Expected struct name"
                } else {
                    "Expected class name"
                }))
            }
        };
        let mut type_params = Vec::new();
        if self.at(Token::Lt) && self.lookahead_type_params() {
            type_params = self.parse_type_param_list()?;
        }
        let extends = if !is_struct && self.at(Token::Extends) {
            self.bump();
            Some(self.expect_identifier("base class")?)
        } else {
            None
        };
        let mut extends_type_args = Vec::new();
        if extends.is_some() && self.at(Token::Lt) && self.lookahead_type_params() {
            extends_type_args = self.parse_type_arg_list()?;
        }
        let mut implements = Vec::new();
        if !is_struct && self.at(Token::Implements) {
            self.bump();
            loop {
                let iface = self.expect_identifier("interface")?;
                let type_args = if self.at(Token::Lt) && self.lookahead_type_args() {
                    self.parse_type_arg_list()?
                } else {
                    Vec::new()
                };
                implements.push(crate::generics::mangle(&iface, &type_args));
                if self.at(Token::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        let where_clause = self.parse_where_clause()?;
        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut associated_types = Vec::new();
        while !self.at(Token::RBrace) && !self.at(Token::Eof) {
            if self.at_identifier("type") {
                associated_types.push(self.parse_class_associated_type()?);
            } else if self.at(Token::Fn) {
                methods.push(self.parse_class_method(is_struct)?);
            } else {
                fields.push(self.parse_class_field()?);
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Stmt::Class {
            name,
            type_params,
            extends,
            extends_type_args,
            implements,
            where_clause,
            associated_types,
            fields,
            methods,
            is_struct,
        })
    }

    fn parse_class_associated_type(&mut self) -> Result<(String, String), ParseError> {
        if !self.at_identifier("type") {
            return Err(self.err("Expected associated type binding"));
        }
        self.bump();
        let name = self.expect_identifier("associated type name")?;
        self.expect(Token::Eq)?;
        let type_name = self.expect_identifier("associated type value")?;
        if self.at(Token::Semicolon) {
            self.bump();
        }
        Ok((name, type_name))
    }

    fn parse_where_clause(&mut self) -> Result<Vec<crate::ast::WhereBound>, ParseError> {
        if !self.at(Token::Where) {
            return Ok(Vec::new());
        }
        self.bump();
        let mut bounds = Vec::new();
        loop {
            let type_param = self.expect_identifier("type parameter in where clause")?;
            self.expect(Token::Colon)?;
            let trait_name = self.expect_identifier("trait name in where clause")?;
            bounds.push(crate::ast::WhereBound {
                type_param,
                trait_name,
            });
            if self.at(Token::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        Ok(bounds)
    }

    fn parse_class_field(&mut self) -> Result<ClassField, ParseError> {
        let (name, private) = self.parse_class_member_name()?;
        if !private {
            self.record(name.clone(), SymbolKind::Field, self.peek().map(|t| t.span).unwrap_or(Span::new(0, 0, 0)));
        }
        let type_name = if self.at(Token::Colon) {
            self.bump();
            match self.bump() {
                Some(spanned) => match spanned.value {
                    Token::Identifier(s) => s,
                    _ => return Err(Self::err_at(spanned.span, "Expected field type")),
                },
                None => return Err(self.err("Expected field type")),
            }
        } else if private {
            String::new()
        } else {
            return Err(self.err("Expected field type"));
        };
        let default = if self.at(Token::Eq) {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        if self.at(Token::Semicolon) {
            self.bump();
        }
        Ok(ClassField {
            name,
            type_name,
            default,
            private,
        })
    }

    fn parse_class_method(&mut self, is_struct: bool) -> Result<ClassMethod, ParseError> {
        self.expect(Token::Fn)?;
        let (name, private) = self.parse_class_member_name()?;
        if !private {
            self.record(name.clone(), SymbolKind::Method, self.peek().map(|t| t.span).unwrap_or(Span::new(0, 0, 0)));
        }
        let mut type_params = Vec::new();
        if self.at(Token::Lt) && self.lookahead_type_params() {
            type_params = self.parse_type_param_list()?;
        }
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        let mut has_self_receiver = false;
        if is_struct {
            // Optional `self` / `&self` / `&mut self` as first parameter.
            if self.at(Token::Self_) {
                self.bump();
                has_self_receiver = true;
                if self.at(Token::Comma) {
                    self.bump();
                }
            } else if self.at(Token::Amp) {
                self.bump();
                let mut_self = self.at_identifier("mut");
                if mut_self {
                    self.bump();
                }
                if self.at(Token::Self_) {
                    self.bump();
                    has_self_receiver = true;
                    if self.at(Token::Comma) {
                        self.bump();
                    }
                } else {
                    return Err(self.err("Expected `self` after `&` / `&mut` in struct method"));
                }
            }
        }
        while !self.at(Token::RParen) {
            match self.bump() {
                Some(spanned) => match spanned.value {
                    Token::Identifier(p) => {
                        self.record(p.clone(), SymbolKind::Parameter, spanned.span);
                        params.push(p);
                        // Optional `: Type` annotation (stored only as name today).
                        if self.at(Token::Colon) {
                            self.bump();
                            let _ = self.parse_type()?;
                        }
                    }
                    Token::Self_ => {
                        return Err(Self::err_at(
                            spanned.span,
                            if is_struct {
                                "`self` must be the first struct method parameter"
                            } else {
                                "use `this` in class methods; `self` is for struct"
                            },
                        ));
                    }
                    _ => return Err(Self::err_at(spanned.span, "Expected parameter name")),
                },
                None => return Err(self.err("Expected parameter name")),
            }
            if self.at(Token::Comma) {
                self.bump();
            }
        }
        self.expect(Token::RParen)?;
        let where_clause = self.parse_where_clause()?;
        let body = self.parse_block()?;
        Ok(ClassMethod {
            name,
            type_params,
            params,
            where_clause,
            body,
            private,
            has_self_receiver,
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Return)?;
        let expr = if !self.at(Token::Semicolon) && !self.at(Token::RBrace) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Stmt::Return(expr))
    }

    fn parse_while_loop(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::While)?;
        if self.at(Token::Let) {
            self.bump();
            let pattern = self.parse_pattern()?;
            self.expect(Token::Eq)?;
            let scrutinee = Box::new(self.parse_expr()?);
            let body = Box::new(self.parse_block()?);
            return Ok(Expr::WhileLet {
                pattern,
                scrutinee,
                body,
            });
        }
        let condition = Box::new(self.parse_expr()?);
        let body = Box::new(self.parse_block()?);
        Ok(Expr::While(condition, body))
    }

    fn parse_do_while_loop(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Do)?;
        let body = Box::new(self.parse_block()?);
        self.expect(Token::While)?;
        let condition = Box::new(self.parse_expr()?);
        Ok(Expr::DoWhile(body, condition))
    }

    fn parse_for_loop(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::For)?;
        let async_for = if self.at(Token::Await) {
            self.bump();
            true
        } else {
            false
        };
        if self.at(Token::Let) || self.at(Token::Const) {
            let saved = self.pos;
            let immutable = self.at(Token::Const);
            self.bump();
            if matches!(self.peek_token(), Some(Token::Identifier(_))) {
                let var = match self.bump() {
                        Some(spanned) => match spanned.value {
                            Token::Identifier(s) => {
                                self.record(s.clone(), SymbolKind::Variable, spanned.span);
                                s
                            }
                            _ => return Err(Self::err_at(spanned.span, "Expected identifier after for")),
                        },
                        None => return Err(self.err("Expected identifier after for")),
                    };
                    if self.at(Token::Of) || self.at(Token::In) {
                        if async_for && !self.at(Token::Of) {
                            return Err(self.err("for await requires `of` iteration"));
                        }
                        let by_value = if self.at(Token::Of) {
                            self.bump();
                            true
                        } else {
        self.expect(Token::In)?;
                            false
                        };
        let iterable = Box::new(self.parse_expr()?);
        let body = Box::new(self.parse_block()?);
                        return Ok(Expr::ForEach(ForeachLoop {
                            var,
                            iterable,
                            body,
                            by_value,
                            async_for,
                            immutable,
                        }));
                    }
            }
            self.pos = saved;
            let immutable = self.at(Token::Const);
            let init = Some(Box::new(self.parse_let_stmt(false, immutable)?));
            self.expect(Token::Semicolon)?;
            let cond = if !self.at(Token::Semicolon) {
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            self.expect(Token::Semicolon)?;
            let step = if !self.at(Token::LBrace) {
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            let body = Box::new(self.parse_block()?);
            return Ok(Expr::ForClassic {
                init,
                cond,
                step,
                body,
            });
        }
        let mut immutable = false;
        if self.at(Token::Let) || self.at(Token::Const) {
            immutable = self.at(Token::Const);
            self.bump();
        }
        let var = match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Identifier(s) => {
                    self.record(s.clone(), SymbolKind::Variable, spanned.span);
                    s
                }
                _ => return Err(Self::err_at(spanned.span, "Expected identifier after for")),
            },
            None => return Err(self.err("Expected identifier after for")),
        };
        let by_value = if self.at(Token::Of) {
            self.bump();
            true
        } else {
            if async_for {
                return Err(self.err("for await requires `of` iteration"));
            }
            self.expect(Token::In)?;
            false
        };
        let iterable = Box::new(self.parse_expr()?);
        let body = Box::new(self.parse_block()?);
        Ok(Expr::ForEach(ForeachLoop {
            var,
            iterable,
            body,
            by_value,
            async_for,
            immutable,
        }))
    }

    fn parse_switch(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Switch)?;
        self.expect(Token::LParen)?;
        let scrutinee = Box::new(self.parse_expr()?);
        self.expect(Token::RParen)?;
        self.expect(Token::LBrace)?;
        let mut cases = Vec::new();
        let mut default_body = None;
        while !self.at(Token::RBrace) && !self.at(Token::Eof) {
            if self.at(Token::Case) {
                let mut values = Vec::new();
                while self.at(Token::Case) {
                    self.bump();
                    values.push(self.parse_expr()?);
                    self.expect(Token::Colon)?;
                }
                let body = self.parse_switch_case_body()?;
                for value in values {
                    cases.push(SwitchCase {
                        value,
                        body: body.clone(),
                    });
                }
            } else if self.at(Token::Default) {
                self.bump();
                self.expect(Token::Colon)?;
                default_body = Some(Box::new(self.parse_switch_case_body()?));
            } else {
                return Err(self.err("Expected `case` or `default` in switch"));
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Switch {
            scrutinee,
            cases,
            default_body,
        })
    }

    fn parse_switch_case_body(&mut self) -> Result<Expr, ParseError> {
        if self.at(Token::LBrace) {
            return self.parse_block();
        }
        let stmt = self.parse_stmt()?;
        Ok(match stmt {
            Stmt::Expr(e) => e,
            other => Expr::Block(vec![other]),
        })
    }

    fn parse_break(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Break)?;
        Ok(Stmt::Expr(Expr::Break))
    }

    fn parse_fallthrough(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Fallthrough)?;
        Ok(Stmt::Expr(Expr::Fallthrough))
    }

    fn parse_continue(&mut self) -> Result<Stmt, ParseError> {
        self.expect(Token::Continue)?;
        Ok(Stmt::Expr(Expr::Continue))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_ternary()
    }

    fn is_ternary_question(&mut self) -> bool {
        if !self.at(Token::Question) {
            return false;
        }
        let saved = self.pos;
        self.pos += 1;
        let parsed = self.parse_expr();
        let has_colon = self.at(Token::Colon);
        self.pos = saved;
        parsed.is_ok() && has_colon
    }

    fn parse_ternary(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_arrow()?;
        if self.at(Token::QuestionQuestion) {
            return Ok(left);
        }
        if self.at(Token::Question) {
            if self.is_ternary_question() {
                self.bump();
                let then_branch = self.parse_expr()?;
                self.expect(Token::Colon)?;
                let else_branch = self.parse_ternary()?;
                return Ok(Expr::Ternary(
                    Box::new(left),
                    Box::new(then_branch),
                    Box::new(else_branch),
                ));
            }
            self.bump();
            left = Expr::ResultQuestion(Box::new(left));
        }
        Ok(left)
    }

    fn parse_fn_param_list(&mut self) -> Result<(Vec<crate::ast::FnParam>, Option<String>), ParseError> {
        let mut params = Vec::new();
        let mut rest = None;
        let mut saw_default = false;
        while !self.at(Token::RParen) {
            if self.at(Token::DotDotDot) {
                self.bump();
                let name = self.expect_identifier("rest parameter")?;
                self.record(name.clone(), SymbolKind::Parameter, Span::unknown());
                rest = Some(name);
                break;
            }
            let name = self.expect_identifier("parameter")?;
            self.record(name.clone(), SymbolKind::Parameter, Span::unknown());
            let type_ann = if self.at(Token::Colon) {
                self.bump();
                Some(self.parse_type()?)
            } else {
                None
            };
            let default = if self.at(Token::Eq) {
                self.bump();
                saw_default = true;
                Some(self.parse_expr()?)
            } else {
                if saw_default {
                    return Err(self.err("Non-default parameter after default parameter"));
                }
                None
            };
            params.push(crate::ast::FnParam {
                name,
                type_ann,
                default,
            });
            if self.at(Token::Comma) {
                self.bump();
            } else if !self.at(Token::RParen) && !self.at(Token::DotDotDot) {
                return Err(self.err("Expected `,` or `)` in parameter list"));
            }
        }
        if self.at(Token::Comma) {
            self.bump();
        }
        Ok((params, rest))
    }

    fn try_parse_paren_param_list(&mut self) -> Option<(Vec<crate::ast::FnParam>, Option<String>)> {
        let start = self.pos;
        if self.at(Token::RParen) {
            self.bump();
            return Some((Vec::new(), None));
        }
        match self.parse_fn_param_list() {
            Ok(sig) => {
                if !self.at(Token::RParen) {
                    self.pos = start;
                    return None;
                }
                self.bump();
                Some(sig)
            }
            Err(_) => {
                self.pos = start;
                None
            }
        }
    }

    fn parse_arrow(&mut self) -> Result<Expr, ParseError> {
        let async_fn = if self.at(Token::Async) {
            self.bump();
            true
        } else {
            false
        };

        if self.at(Token::LParen) {
            if self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| matches!(t.value, Token::LParen))
            {
                return self.parse_assign();
            }
            let saved = self.pos;
            self.bump();
            if let Some((params, rest)) = self.try_parse_paren_param_list() {
                if self.at(Token::FatArrow) {
                    self.bump();
                    let body = self.parse_arrow_body()?;
                    return Ok(Expr::Arrow {
                        params,
                        rest,
                        body: Box::new(body),
                        async_fn,
                        generator_fn: false,
                    });
                }
            }
            self.pos = saved;
        }

        if matches!(self.peek_token(), Some(Token::Identifier(_))) {
            if self.tokens.get(self.pos + 1).map(|t| &t.value) == Some(&Token::FatArrow) {
                let name = self.expect_identifier("parameter")?;
                self.record(name.clone(), SymbolKind::Parameter, Span::unknown());
                self.expect(Token::FatArrow)?;
                let body = self.parse_arrow_body()?;
                return Ok(Expr::Arrow {
                    params: vec![crate::ast::FnParam {
                        name,
                        type_ann: None,
                        default: None,
                    }],
                    rest: None,
                    body: Box::new(body),
                    async_fn,
                    generator_fn: false,
                });
            }
        }

        if async_fn && self.at(Token::Fn) {
            return self.parse_function_with_public(false);
        }

        if async_fn {
            return Err(self.err("`async` must be followed by `fn` or an arrow function"));
        }

        self.parse_assign()
    }

    fn parse_arrow_body(&mut self) -> Result<Expr, ParseError> {
        if self.at(Token::LBrace) {
            self.parse_block()
        } else {
            self.parse_arrow()
        }
    }

    fn assign_target_from_expr(&self, expr: Expr) -> Result<AssignTarget, ParseError> {
        match expr {
            Expr::Variable(name) => Ok(AssignTarget::Name(name)),
            Expr::Member(obj, field, _) => Ok(AssignTarget::Member(obj, field)),
            Expr::Index(obj, idx) => Ok(AssignTarget::Index(obj, idx)),
            other => Err(self.err(format!("Invalid assignment target: {:?}", other))),
        }
    }

    fn parse_assign(&mut self) -> Result<Expr, ParseError> {
        if self.at(Token::LBracket) || (self.at(Token::LBrace) && self.peek_destructure_object()) {
            let saved = self.pos;
            if let Ok(pat) = self.try_parse_binding_pattern() {
                if self.at(Token::Eq) {
                    self.bump();
            let value = self.parse_assign()?;
                    return Ok(Expr::Assign(
                        AssignTarget::Pattern(pat),
                        Box::new(value),
                    ));
                }
            }
            self.pos = saved;
        }
        let left = self.parse_nullish()?;
        if let Some(op) = self.peek_compound_assign_op() {
            self.bump();
            let target = self.assign_target_from_expr(left.clone())?;
            let right = self.parse_assign()?;
            let value = Expr::Binary(Box::new(left), op, Box::new(right));
            return Ok(Expr::Assign(target, Box::new(value)));
        }
        if self.at(Token::Eq) {
            self.bump();
            let target = self.assign_target_from_expr(left)?;
            let value = self.parse_assign()?;
            Ok(Expr::Assign(target, Box::new(value)))
        } else {
            Ok(left)
        }
    }

    fn peek_compound_assign_op(&self) -> Option<BinaryOp> {
        match self.peek_token()? {
            Token::PlusEq => Some(BinaryOp::Add),
            Token::MinusEq => Some(BinaryOp::Sub),
            Token::StarEq => Some(BinaryOp::Mul),
            Token::SlashEq => Some(BinaryOp::Div),
            Token::PercentEq => Some(BinaryOp::Mod),
            Token::AndAndEq => Some(BinaryOp::And),
            Token::OrOrEq => Some(BinaryOp::Or),
            Token::QuestionQuestionEq => Some(BinaryOp::NullishCoalesce),
            _ => None,
        }
    }

    fn try_parse_binding_pattern(&mut self) -> Result<BindingPattern, ParseError> {
        self.parse_binding_pattern()
    }

    fn peek_destructure_object(&mut self) -> bool {
        let saved = self.pos;
        if !self.at(Token::LBrace) {
            return false;
        }
        self.bump();
        let is_destructure = self.at(Token::DotDotDot)
            || matches!(self.peek_token(), Some(Token::Identifier(_)))
            || self.at(Token::RBrace);
        self.pos = saved;
        is_destructure
    }

    fn parse_binding_pattern(&mut self) -> Result<BindingPattern, ParseError> {
        match self.peek_token() {
            Some(Token::LBracket) => {
                self.bump();
                let mut items = Vec::new();
                while !self.at(Token::RBracket) {
                    if self.at(Token::Comma) {
                        self.bump();
                        continue;
                    }
                    if self.at(Token::DotDotDot) {
                        self.bump();
                        let name = self.parse_optional_rest_name("rest binding")?;
                        items.push(BindingPattern::Rest(name));
                    } else if self.at_identifier("_") {
                        self.bump();
                        items.push(BindingPattern::Wildcard);
                    } else {
                        items.push(self.parse_binding_pattern()?);
                    }
                    if self.at(Token::Comma) {
                        self.bump();
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(BindingPattern::Array(items))
            }
            Some(Token::LBrace) => {
                self.bump();
                let mut fields = Vec::new();
                while !self.at(Token::RBrace) {
                    if self.at(Token::Comma) {
                        self.bump();
                        continue;
                    }
                    if self.at(Token::DotDotDot) {
                        self.bump();
                        let name = self.parse_optional_rest_name("rest binding")?;
                        fields.push(ObjectBind::Rest(name));
                    } else {
                        let key = match self.bump() {
                            Some(spanned) => match spanned.value {
                                Token::Identifier(s) | Token::String(s) => s,
                                _ => {
                                    return Err(Self::err_at(
                                        spanned.span,
                                        "Expected object binding key",
                                    ))
                                }
                            },
                            None => return Err(self.err("Expected object binding key")),
                        };
                        if self.at(Token::Colon) {
                            self.bump();
                            let pattern = self.parse_binding_pattern()?;
                            fields.push(ObjectBind::Field { key, pattern });
                        } else {
                            fields.push(ObjectBind::Shorthand(key));
                        }
                    }
                    if self.at(Token::Comma) {
                        self.bump();
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(BindingPattern::Object(fields))
            }
            Some(Token::Identifier(_)) => {
                let spanned = self.bump().unwrap();
                let Token::Identifier(name) = spanned.value else {
                    unreachable!();
                };
                self.record(name.clone(), SymbolKind::Variable, spanned.span);
                Ok(BindingPattern::Name(name))
            }
            _ => Err(self.err("Expected binding pattern")),
        }
    }

    fn at_identifier(&self, name: &str) -> bool {
        matches!(
            self.peek_token(),
            Some(Token::Identifier(s)) if s == name
        )
    }

    fn expect_identifier(&mut self, ctx: &str) -> Result<String, ParseError> {
        match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Identifier(s) => Ok(s),
                _ => Err(Self::err_at(spanned.span, format!("Expected identifier for {}", ctx))),
            },
            None => Err(self.err(format!("Expected identifier for {}", ctx))),
        }
    }

    fn parse_nullish(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_or()?;
        while self.at(Token::QuestionQuestion) {
            self.bump();
            let right = self.parse_or()?;
            left = Expr::Binary(Box::new(left), BinaryOp::NullishCoalesce, Box::new(right));
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.at(Token::OrOr) {
            self.bump();
            let right = self.parse_and()?;
            left = Expr::Binary(Box::new(left), BinaryOp::Or, Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_bit_or()?;
        while self.at(Token::AndAnd) {
            self.bump();
            let right = self.parse_bit_or()?;
            left = Expr::Binary(Box::new(left), BinaryOp::And, Box::new(right));
        }
        Ok(left)
    }

    fn parse_bit_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_bit_xor()?;
        while self.at(Token::Pipe) {
            self.bump();
            let right = self.parse_bit_xor()?;
            left = Expr::Binary(Box::new(left), BinaryOp::BitOr, Box::new(right));
        }
        Ok(left)
    }

    fn parse_bit_xor(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_bit_and()?;
        while self.at(Token::Caret) {
            self.bump();
            let right = self.parse_bit_and()?;
            left = Expr::Binary(Box::new(left), BinaryOp::BitXor, Box::new(right));
        }
        Ok(left)
    }

    fn parse_bit_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        while self.at(Token::Amp) {
            self.bump();
            let right = self.parse_comparison()?;
            left = Expr::Binary(Box::new(left), BinaryOp::BitAnd, Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_shift()?;
        while let Some(op) = self.peek_token().cloned() {
            let bin_op = match op {
                Token::EqEq => {
                    self.bump();
                    BinaryOp::Eq
                }
                Token::Ne => {
                    self.bump();
                    BinaryOp::Ne
                }
                Token::Lt => {
                    self.bump();
                    BinaryOp::Lt
                }
                Token::Le => {
                    self.bump();
                    BinaryOp::Le
                }
                Token::Gt => {
                    self.bump();
                    BinaryOp::Gt
                }
                Token::Ge => {
                    self.bump();
                    BinaryOp::Ge
                }
                Token::In => {
                    self.bump();
                    BinaryOp::In
                }
                Token::Is => {
                    self.bump();
                    if self.at(Token::Not) {
                        self.bump();
                        BinaryOp::IsNot
                    } else {
                        BinaryOp::Is
                    }
                }
                _ => break,
            };
            let right = self.parse_shift()?;
            left = Expr::Binary(Box::new(left), bin_op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_function_or_additive()?;
        while let Some(op) = self.peek_token().cloned() {
            let bin_op = match op {
                Token::Shl => BinaryOp::Shl,
                Token::Shr => BinaryOp::Shr,
                Token::Ushr => BinaryOp::Ushr,
                _ => break,
            };
            self.bump();
            let right = self.parse_function_or_additive()?;
            left = Expr::Binary(Box::new(left), bin_op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_function_or_additive(&mut self) -> Result<Expr, ParseError> {
        if self.at(Token::Fn) || self.at(Token::Pub) || self.at(Token::Async) {
            self.parse_function()
        } else {
            self.parse_additive()
        }
    }

    fn parse_function(&mut self) -> Result<Expr, ParseError> {
        self.parse_function_with_public(false)
    }

    fn parse_function_with_public(&mut self, public: bool) -> Result<Expr, ParseError> {
        let mut is_public = public;
        let mut async_fn = false;
        let mut generator_fn = false;
        loop {
            if self.at(Token::Pub) {
                self.bump();
                is_public = true;
            } else if self.at(Token::Async) {
                self.bump();
                async_fn = true;
            } else {
                break;
            }
        }
        self.expect(Token::Fn)?;
        if self.at(Token::Star) {
            self.bump();
            generator_fn = true;
        }
        let name = match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Identifier(s) => {
                    self.record(s.clone(), SymbolKind::Function, spanned.span);
                    s
                }
                _ => return Err(Self::err_at(spanned.span, "Expected function name")),
            },
            None => return Err(self.err("Expected function name")),
        };
        let mut type_params = Vec::new();
        if self.at(Token::Lt) && self.lookahead_type_params() {
            type_params = self.parse_type_param_list()?;
        }
        self.expect(Token::LParen)?;
        let (params, rest) = self.parse_fn_param_list()?;
        self.expect(Token::RParen)?;
        let return_type = if self.at(Token::ReturnArrow) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        let where_clause = self.parse_where_clause()?;
        self.expect(Token::LBrace)?;
        let mut stmts = Vec::new();
        while !self.at(Token::RBrace) && !self.at(Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(Token::RBrace)?;
        let body = Expr::Block(stmts);
        Ok(Expr::Function {
            name,
            type_params,
            params,
            rest,
            return_type,
            where_clause,
            body: Box::new(body),
            public: is_public,
            async_fn,
            generator_fn,
        })
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_power()?;
        while let Some(op) = self.peek_token().cloned() {
            match op {
                Token::Plus => {
                    self.bump();
                    let right = self.parse_power()?;
                    left = Expr::Binary(Box::new(left), BinaryOp::Add, Box::new(right));
                }
                Token::Minus => {
                    self.bump();
                    let right = self.parse_power()?;
                    left = Expr::Binary(Box::new(left), BinaryOp::Sub, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_multiplicative()?;
        if self.at(Token::StarStar) {
            self.bump();
            let right = self.parse_power()?;
            return Ok(Expr::Binary(Box::new(left), BinaryOp::Pow, Box::new(right)));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.peek_token().cloned() {
            match op {
                Token::Star => {
                    self.bump();
                    let right = self.parse_unary()?;
                    left = Expr::Binary(Box::new(left), BinaryOp::Mul, Box::new(right));
                }
                Token::Slash => {
                    self.bump();
                    let right = self.parse_unary()?;
                    left = Expr::Binary(Box::new(left), BinaryOp::Div, Box::new(right));
                }
                Token::Percent => {
                    self.bump();
                    let right = self.parse_unary()?;
                    left = Expr::Binary(Box::new(left), BinaryOp::Mod, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.at(Token::Yield) {
            self.bump();
            if self.at(Token::Star) {
                self.bump();
                let inner = self.parse_assign()?;
                return Ok(Expr::YieldStar(Box::new(inner)));
            }
            let inner = if self.can_start_yield_operand() {
                Some(self.parse_assign()?)
            } else {
                None
            };
            return Ok(Expr::Yield(Box::new(
                inner.unwrap_or(Expr::Literal(Literal::Undefined)),
            )));
        }
        if self.at(Token::Await) {
            self.bump();
            let inner = self.parse_unary()?;
            return Ok(Expr::Await(Box::new(inner)));
        }
        if self.at(Token::Bang) || self.at(Token::Not) {
            self.bump();
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryOp::Not, Box::new(inner)));
        }
        if self.at(Token::Amp) {
            self.bump();
            let exclusive = matches!(
                self.peek_token(),
                Some(Token::Identifier(s)) if s == "mut"
            );
            if exclusive {
                self.bump();
            }
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary(
                if exclusive {
                    UnaryOp::RefMut
                } else {
                    UnaryOp::Ref
                },
                Box::new(inner),
            ));
        }
        if self.at(Token::Minus) {
            self.bump();
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryOp::Neg, Box::new(inner)));
        }
        if self.at(Token::Tilde) {
            self.bump();
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryOp::BitNot, Box::new(inner)));
        }
        if self.at(Token::Delete) {
            self.bump();
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryOp::Delete, Box::new(inner)));
        }
        if self.at(Token::Throw) || self.at(Token::Raise) {
            self.bump();
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryOp::Throw, Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let atom = self.parse_atom()?;
        self.parse_postfix(atom)
    }

    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        match self.bump() {
            Some(spanned) => {
                self.last_primary_line = spanned.span.line;
                match spanned.value {
                Token::Number(n) => Ok(Expr::Literal(Literal::Number(n))),
                Token::BigInt(digits) => Ok(Expr::Literal(Literal::BigInt(digits))),
                Token::Float(f) => Ok(Expr::Literal(Literal::Float(f))),
                Token::String(s) => Ok(Expr::Literal(Literal::String(s))),
                Token::Template(parts) => self.parse_template(parts),
                Token::Identifier(name) => Ok(Expr::Variable(name)),
                Token::Some => {
                self.expect(Token::LParen)?;
                let inner = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(Expr::Literal(Literal::Some(Box::new(inner))))
            }
                Token::None => Ok(Expr::Literal(Literal::None)),
                Token::Ok => {
                self.expect(Token::LParen)?;
                let inner = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(Expr::Literal(Literal::Ok(Box::new(inner))))
            }
                Token::True => Ok(Expr::Literal(Literal::Bool(true))),
                Token::False => Ok(Expr::Literal(Literal::Bool(false))),
                Token::Null => Ok(Expr::Literal(Literal::Null)),
                Token::Undefined => Ok(Expr::Literal(Literal::Undefined)),
                Token::NaN => Ok(Expr::Literal(Literal::Nan)),
                Token::Import => self.parse_import_expr_after_keyword(),
                Token::This => Ok(Expr::This),
                Token::Self_ => Ok(Expr::Self_),
                Token::Super => Ok(Expr::Super),
                Token::Err => {
                self.expect(Token::LParen)?;
                let inner = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(Expr::Literal(Literal::Err(Box::new(inner))))
            }
                Token::Match => self.parse_match(),
                Token::Try => self.parse_try_catch(),
                Token::If => {
                    if self.at(Token::Let) {
                        self.bump();
                        let pattern = self.parse_pattern()?;
                        self.expect(Token::Eq)?;
                        let scrutinee = Box::new(self.parse_expr()?);
                        let body = Box::new(self.parse_block()?);
                        let else_branch = if self.at(Token::Else) {
                            self.bump();
                            let branch = if self.at(Token::LBrace) {
                                self.parse_block()?
                            } else {
                                self.parse_expr()?
                            };
                            Some(Box::new(branch))
                        } else {
                            None
                        };
                        return Ok(Expr::IfLet {
                            pattern,
                            scrutinee,
                            body,
                            else_branch,
                        });
                    }
                let cond = self.parse_expr()?;
                let then_branch = self.parse_block()?;
                    let else_branch = if self.at(Token::Else) {
                        self.bump();
                        let branch = if self.at(Token::LBrace) {
                        self.parse_block()?
                    } else {
                        self.parse_expr()?
                    };
                    Some(Box::new(branch))
                } else {
                    None
                };
                Ok(Expr::If(Box::new(cond), Box::new(then_branch), else_branch))
            }
                Token::LParen => {
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
                Token::LBrace => self.parse_brace_expr(),
                Token::LBracket => {
                    let mut items = Vec::new();
                    while !self.at(Token::RBracket) {
                        if self.at(Token::DotDotDot) {
                            self.bump();
                            items.push(ArrayPiece::Spread(self.parse_expr()?));
                        } else {
                            items.push(ArrayPiece::Item(self.parse_expr()?));
                        }
                        if self.at(Token::Comma) {
                            self.bump();
                        }
                    }
                    self.expect(Token::RBracket)?;
                    Ok(Expr::Literal(Literal::Array(items)))
                }
                other => Err(Self::err_at(
                    spanned.span,
                    format!("Unexpected token: {:?}", other),
                )),
                }
            }
            None => Err(self.err("Unexpected end of input")),
        }
    }

    fn parse_postfix(&mut self, mut left: Expr) -> Result<Expr, ParseError> {
        let mut pending_type_args = Vec::new();
        loop {
            match self.peek_token() {
                Some(Token::Lt) if Self::is_generic_instantiation_callee(&left) && self.lookahead_type_args() => {
                    pending_type_args = self.parse_type_arg_list()?;
                }
                Some(Token::LBracket) => {
                    let bracket_line = self.peek().map(|t| t.span.line).unwrap_or(0);
                    if bracket_line != self.last_primary_line {
                        break;
                    }
                    self.bump();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    left = Expr::Index(Box::new(left), Box::new(index));
                }
                Some(Token::Dot) => {
                    self.bump();
                    let name = self.parse_member_name()?;
                    left = Expr::Member(Box::new(left), name, std::mem::take(&mut pending_type_args));
                }
                Some(Token::QuestionDot) => {
                    self.bump();
                    if self.at(Token::LBracket) {
                        self.bump();
                        let index = self.parse_expr()?;
                        self.expect(Token::RBracket)?;
                        left = Expr::OptionalIndex(Box::new(left), Box::new(index));
                    } else if self.at(Token::LParen) {
                        self.bump();
                        let mut args = Vec::new();
                        while !self.at(Token::RParen) {
                            if self.at(Token::DotDotDot) {
                                self.bump();
                                args.push(CallArg::Spread(self.parse_expr()?));
                            } else {
                                args.push(CallArg::Expr(self.parse_expr()?));
                            }
                            if self.at(Token::Comma) {
                                self.bump();
                            }
                        }
                        self.expect(Token::RParen)?;
                        left = Expr::OptionalCall(Box::new(left), args);
                    } else {
                        let name = self.parse_member_name()?;
                        left = Expr::OptionalMember(Box::new(left), name);
                    }
                }
                Some(Token::LParen) => {
                    self.bump();
                    let mut args = Vec::new();
                    while !self.at(Token::RParen) {
                        if self.at(Token::DotDotDot) {
                            self.bump();
                            args.push(CallArg::Spread(self.parse_expr()?));
                        } else {
                            args.push(CallArg::Expr(self.parse_expr()?));
                        }
                        if self.at(Token::Comma) {
                            self.bump();
                        }
                    }
                    self.expect(Token::RParen)?;
                    left = Expr::Call {
                        func: Box::new(left),
                        type_args: pending_type_args,
                        args,
                    };
                    pending_type_args = Vec::new();
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn is_generic_instantiation_callee(expr: &Expr) -> bool {
        matches!(expr, Expr::Variable(_) | Expr::Member(_, _, _))
    }

    fn lookahead_type_params(&self) -> bool {
        self.lookahead_type_args()
    }

    /// `<` … `>` where inner tokens are identifiers (not comparison).
    fn lookahead_type_args(&self) -> bool {
        let mut i = self.pos + 1;
        if !matches!(
            self.tokens.get(i).map(|t| &t.value),
            Some(Token::Identifier(_))
        ) {
            return false;
        }
        i += 1;
        while i < self.tokens.len() {
            match &self.tokens[i].value {
                Token::Gt => return true,
                Token::Lt => return false,
                Token::Comma | Token::Identifier(_) => i += 1,
                _ => return false,
            }
        }
        false
    }

    fn parse_type_param_list(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(Token::Lt)?;
        let mut names = Vec::new();
        while !self.at(Token::Gt) {
            let name = match self.bump() {
                Some(spanned) => match spanned.value {
                    Token::Identifier(s) => {
                        self.record(s.clone(), SymbolKind::TypeParam, spanned.span);
                        s
                    }
                    _ => return Err(Self::err_at(spanned.span, "Expected type parameter")),
                },
                None => return Err(self.err("Expected type parameter")),
            };
            names.push(name);
            if self.at(Token::Comma) {
                self.bump();
            } else if !self.at(Token::Gt) {
                return Err(self.err("Expected `,` or `>` in type parameter"));
            }
        }
        self.expect(Token::Gt)?;
        Ok(names)
    }

    fn parse_type_arg_list(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(Token::Lt)?;
        let args = self.parse_type_ident_list("type argument")?;
        self.expect(Token::Gt)?;
        Ok(args)
    }

    fn parse_type_ident_list(&mut self, ctx: &str) -> Result<Vec<String>, ParseError> {
        let mut names = Vec::new();
        while !self.at(Token::Gt) {
            names.push(self.expect_identifier(ctx)?);
            if self.at(Token::Comma) {
                self.bump();
            } else if !self.at(Token::Gt) {
                return Err(self.err(format!("Expected `,` or `>` in {}", ctx)));
            }
        }
        Ok(names)
    }

    fn parse_type(&mut self) -> Result<crate::ast::KabType, ParseError> {
        if self.at(Token::Amp) {
            self.bump();
            let exclusive = matches!(
                self.peek_token(),
                Some(Token::Identifier(s)) if s == "mut"
            );
            if exclusive {
                self.bump();
            }
            let inner = self.parse_type()?;
            return Ok(if exclusive {
                crate::ast::KabType::RefMut(Box::new(inner))
            } else {
                crate::ast::KabType::Ref(Box::new(inner))
            });
        }
        let name = self.expect_identifier("type")?;
        Ok(crate::ast::KabType::Named(name))
    }

    fn parse_match(&mut self) -> Result<Expr, ParseError> {
        let value = Box::new(self.parse_expr()?);
        self.expect(Token::LBrace)?;
        let mut arms = Vec::new();
        while !self.at(Token::RBrace) {
            let pattern = self.parse_pattern()?;
            let guard = if self.at(Token::If) {
                self.bump();
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(Token::FatArrow)?;
            let body = self.parse_expr()?;
            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });
            if self.at(Token::Comma) {
                self.bump();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Match(value, arms))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        if self.at(Token::LBracket) {
            return self.parse_array_pattern();
        }
        if self.at(Token::LBrace) {
            return self.parse_object_pattern();
        }
        match self.bump() {
            Some(spanned) => match spanned.value {
                Token::Identifier(s) => {
                if s == "_" {
                    Ok(Pattern::Wildcard)
                    } else if self.at(Token::Dot) {
                        self.bump();
                        let variant = self.parse_member_name()?;
                        let fields = if self.at(Token::LParen) {
                            self.bump();
                            let mut fields = Vec::new();
                            while !self.at(Token::RParen) {
                                if self.at(Token::Comma) {
                                    self.bump();
                                    continue;
                                }
                                fields.push(self.parse_pattern()?);
                                if self.at(Token::Comma) {
                                    self.bump();
                                }
                            }
                            self.expect(Token::RParen)?;
                            fields
                        } else {
                            Vec::new()
                        };
                        Ok(Pattern::EnumVariant {
                            enum_name: s,
                            variant,
                            fields,
                        })
                } else {
                    Ok(Pattern::Variable(s))
                }
            }
                Token::Number(n) => Ok(Pattern::Number(n)),
                Token::Float(f) => Ok(Pattern::Float(f)),
                Token::String(s) => Ok(Pattern::String(s)),
                Token::Null => Ok(Pattern::Null),
                Token::Undefined => Ok(Pattern::Undefined),
                Token::NaN => Ok(Pattern::Nan),
                Token::True => Ok(Pattern::Bool(true)),
                Token::False => Ok(Pattern::Bool(false)),
                Token::Some => {
                self.expect(Token::LParen)?;
                let inner = self.parse_pattern()?;
                self.expect(Token::RParen)?;
                Ok(Pattern::Some(Box::new(inner)))
            }
                Token::None => Ok(Pattern::None),
                Token::Ok => {
                self.expect(Token::LParen)?;
                let inner = self.parse_pattern()?;
                self.expect(Token::RParen)?;
                Ok(Pattern::Ok(Box::new(inner)))
            }
                Token::Err => {
                self.expect(Token::LParen)?;
                let inner = self.parse_pattern()?;
                self.expect(Token::RParen)?;
                Ok(Pattern::Err(Box::new(inner)))
            }
                other => Err(Self::err_at(
                    spanned.span,
                    format!("Unexpected token in pattern: {:?}", other),
                )),
            },
            None => Err(self.err("Expected pattern")),
        }
    }

    fn parse_optional_rest_name(&mut self, ctx: &str) -> Result<String, ParseError> {
        if self.at(Token::RBracket) || self.at(Token::RBrace) || self.at(Token::Comma) {
            Ok(String::new())
        } else {
            self.expect_identifier(ctx)
        }
    }

    fn parse_array_pattern(&mut self) -> Result<Pattern, ParseError> {
        self.expect(Token::LBracket)?;
        let mut items = Vec::new();
        while !self.at(Token::RBracket) {
            if self.at(Token::Comma) {
                self.bump();
                continue;
            }
            if self.at(Token::DotDotDot) {
                self.bump();
                let name = self.parse_optional_rest_name("rest pattern")?;
                items.push(PatternPiece::Rest(name));
            } else if self.at_identifier("_") {
                self.bump();
                items.push(PatternPiece::Wildcard);
            } else {
                items.push(PatternPiece::Item(self.parse_pattern()?));
            }
            if self.at(Token::Comma) {
                self.bump();
            }
        }
        self.expect(Token::RBracket)?;
        Ok(Pattern::Array(items))
    }

    fn parse_object_pattern(&mut self) -> Result<Pattern, ParseError> {
        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();
        while !self.at(Token::RBrace) {
            if self.at(Token::Comma) {
                self.bump();
                continue;
            }
            if self.at(Token::DotDotDot) {
                self.bump();
                let name = self.parse_optional_rest_name("rest pattern")?;
                fields.push(PatternField::Rest(name));
            } else {
                let key = match self.bump() {
                    Some(spanned) => match spanned.value {
                        Token::Identifier(s) | Token::String(s) => s,
                        _ => {
                            return Err(Self::err_at(
                                spanned.span,
                                "Expected object pattern key",
                            ))
                        }
                    },
                    None => return Err(self.err("Expected object pattern key")),
                };
                if self.at(Token::Colon) {
                    self.bump();
                    let pattern = self.parse_pattern()?;
                    fields.push(PatternField::Field { key, pattern });
                } else {
                    fields.push(PatternField::Shorthand(key));
                }
            }
            if self.at(Token::Comma) {
                self.bump();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Pattern::Object(fields))
    }

    fn parse_brace_expr(&mut self) -> Result<Expr, ParseError> {
        if self.peek_object_field() {
            self.parse_object_literal()
        } else {
            self.parse_block_body()
        }
    }

    fn peek_object_field(&mut self) -> bool {
        if self.at(Token::DotDotDot) {
            return true;
        }
        if self.at(Token::RBrace) {
            return true;
        }
        if !matches!(
            self.peek_token(),
            Some(Token::Identifier(_)) | Some(Token::String(_))
        ) {
            return false;
        }
        let saved = self.pos;
        self.bump();
        let is_object = self.at(Token::Colon) || self.at(Token::LParen);
        self.pos = saved;
        is_object
    }

    fn parse_object_key(&mut self) -> Result<String, ParseError> {
        let spanned = self.bump().ok_or_else(|| self.err("Expected object key"))?;
        match spanned.value {
            Token::Identifier(s) | Token::String(s) => Ok(s),
            Token::Return => Ok("return".into()),
            Token::Throw => Ok("throw".into()),
            Token::Is => Ok("is".into()),
            Token::For => Ok("for".into()),
            _ => Err(Self::err_at(spanned.span, "Expected object key")),
        }
    }

    fn parse_object_literal(&mut self) -> Result<Expr, ParseError> {
        let mut fields = Vec::new();
        while !self.at(Token::RBrace) {
            if self.at(Token::Comma) {
                self.bump();
                continue;
            }
            if self.at(Token::DotDotDot) {
                self.bump();
                fields.push(ObjectPiece::Spread(self.parse_expr()?));
            } else {
                let key = self.parse_object_key()?;
                if self.at(Token::LParen) {
                    self.expect(Token::LParen)?;
                    let (params, rest) = self.parse_fn_param_list()?;
                    self.expect(Token::RParen)?;
                    let body = Box::new(self.parse_block()?);
                    fields.push(ObjectPiece::Method {
                        key,
                        params,
                        rest,
                        body,
                        async_fn: false,
                    });
                } else {
                    self.expect(Token::Colon)?;
                    let value = self.parse_expr()?;
                    fields.push(ObjectPiece::Field { key, value });
                }
            }
            if self.at(Token::Comma) {
                self.bump();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Literal(Literal::Object(fields)))
    }

    fn parse_try_catch(&mut self) -> Result<Expr, ParseError> {
        let body = Box::new(self.parse_block()?);
        self.expect(Token::Catch)?;
        self.expect(Token::LParen)?;
        let err_name = self.expect_identifier("catch parameter")?;
        self.expect(Token::RParen)?;
        let handler = Box::new(self.parse_block()?);
        let finally = if self.at(Token::Finally) {
            self.bump();
            Some(Box::new(self.parse_block()?))
        } else {
            None
        };
        Ok(Expr::TryCatch {
            body,
            err_name,
            handler,
            finally,
        })
    }

    fn parse_block_body(&mut self) -> Result<Expr, ParseError> {
        let mut stmts = Vec::new();
        while !self.at(Token::RBrace) && !self.at(Token::Eof) {
            while self.at(Token::Semicolon) {
                self.bump();
            }
            if self.at(Token::RBrace) || self.at(Token::Eof) {
                break;
            }
            stmts.push(self.parse_stmt()?);
            if self.at(Token::Semicolon) {
                self.bump();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Block(stmts))
    }

    fn parse_template(&mut self, parts: Vec<TemplatePart>) -> Result<Expr, ParseError> {
        let mut result: Option<Expr> = None;
        for part in parts {
            let piece = match part {
                TemplatePart::Text(t) => Expr::Literal(Literal::String(t)),
                TemplatePart::Expr(src) => {
                    let tokens = tokenize(&src).map_err(|e| ParseError {
                        message: e.to_string(),
                        span: Span::unknown(),
                    })?;
                    let mut sub = Parser::with_eof(tokens);
                    sub.parse_expr()?
                }
            };
            result = Some(match result {
                None => piece,
                Some(prev) => Expr::Binary(Box::new(prev), BinaryOp::Add, Box::new(piece)),
            });
        }
        Ok(result.unwrap_or(Expr::Literal(Literal::String(String::new()))))
    }

    fn parse_block(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LBrace)?;
        self.parse_block_body()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::SymbolKind;
    use crate::lexer::tokenize;

    #[test]
    fn parse_error_points_at_bad_token() {
        let tokens = tokenize("let = 1;").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let err = parser.parse_program().unwrap_err();
        assert_eq!(err.span.line, 1);
        assert!(err.span.column >= 5);
        assert!(err.message.contains("identifier") || err.message.contains("Expected"));
    }

    #[test]
    fn parse_error_points_at_unexpected_token() {
        let tokens = tokenize("1 + ;").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let err = parser.parse_program().unwrap_err();
        assert_eq!(err.span.line, 1);
        assert!(err.message.contains("Unexpected"));
    }

    #[test]
    fn parses_iife_arrow() {
        let tokens = tokenize("((n) => n + 1)(4)").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Stmt::Expr(Expr::Call { func, args, .. }) => {
                assert!(matches!(func.as_ref(), Expr::Arrow { .. }));
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected call, got {other:?}"),
        }
    }

    #[test]
    fn parses_compound_assignment() {
        let tokens = tokenize("n += 2").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        match &stmts[0] {
            Stmt::Expr(Expr::Assign(AssignTarget::Name(name), value)) => {
                assert_eq!(name, "n");
                match value.as_ref() {
                    Expr::Binary(lhs, BinaryOp::Add, _) => {
                        assert!(matches!(lhs.as_ref(), Expr::Variable(n) if n == "n"));
                    }
                    other => panic!("expected n = n + _, got {other:?}"),
                }
            }
            other => panic!("expected compound assign, got {other:?}"),
        }
    }

    #[test]
    fn parses_anonymous_array_destructure_rest() {
        let tokens = tokenize("let [a, ...] = [1]").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        match &stmts[0] {
            Stmt::Let { pattern, .. } => match pattern {
                BindingPattern::Array(items) => {
                    assert!(matches!(&items[1], BindingPattern::Rest(name) if name.is_empty()));
                }
                other => panic!("expected array pattern, got {other:?}"),
            },
            other => panic!("expected let, got {other:?}"),
        }
    }

    #[test]
    fn parses_anonymous_array_match_rest() {
        let tokens = tokenize("match [1] { [x, ...] => x, _ => 0 }").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        match &stmts[0] {
            Stmt::Expr(Expr::Match(_, arms)) => match &arms[0].pattern {
                Pattern::Array(items) => {
                    assert!(matches!(items[0], PatternPiece::Item(Pattern::Variable(_))));
                    assert!(matches!(&items[1], PatternPiece::Rest(name) if name.is_empty()));
                }
                other => panic!("expected array pattern, got {other:?}"),
            },
            other => panic!("expected match, got {other:?}"),
        }
    }

    #[test]
    fn parses_in_membership_operator() {
        let tokens = tokenize(r#"1 in [1, 2]"#).unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        assert!(matches!(
            stmts[0],
            Stmt::Expr(Expr::Binary(_, BinaryOp::In, _))
        ));
    }

    #[test]
    fn parses_nullish_coalesce_after_number() {
        let tokens = tokenize("0 ?? 9").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        assert!(matches!(
            stmts[0],
            Stmt::Expr(Expr::Binary(_, BinaryOp::NullishCoalesce, _))
        ));
    }

    #[test]
    fn parses_grouped_parenthesized_expressions() {
        let tokens = tokenize("(1 + 2)").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], Stmt::Expr(Expr::Binary(_, BinaryOp::Add, _))));

        let tokens = tokenize("({ ...o, b: 2 })").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(
            stmts[0],
            Stmt::Expr(Expr::Literal(Literal::Object(_)))
        ));
    }

    #[test]
    fn collects_symbols_for_class_and_methods() {
        let tokens = tokenize("class Person { name: String; fn greet() { name } }").unwrap();
        let mut parser = Parser::with_eof(tokens);
        parser.parse_program().unwrap();
        assert!(parser
            .symbols()
            .iter()
            .any(|s| s.name == "Person" && s.kind == SymbolKind::Class));
        assert!(parser
            .symbols()
            .iter()
            .any(|s| s.name == "greet" && s.kind == SymbolKind::Method));
    }

    #[test]
    fn paren_is_expr_parses_as_binary() {
        let tokens = tokenize("(a is not undefined)").unwrap();
        let mut parser = Parser::with_eof(tokens);
        let stmts = parser.parse_program().unwrap();
        let Stmt::Expr(expr) = &stmts[0] else {
            panic!("expected expr stmt, got {:?}", stmts[0]);
        };
        assert!(
            matches!(
                expr,
                Expr::Binary(_, BinaryOp::IsNot, _)
                    | Expr::Binary(_, BinaryOp::Is, _)
            ),
            "expected is/is not binary, got {:?}",
            expr
        );
    }
}
