use crate::ast::{BindingPattern, CallArg, ClassMethod, Expr, FnParam, KabType, Literal, Stmt};
use crate::generics::mangle;
use crate::language::{Symbol, SymbolKind};
use crate::lexer::tokenize;
use crate::parser::Parser;

pub const TYPE_NAMES: &[&str] = &[
    "Number", "String", "Bool", "Null", "Undefined", "Float", "BigInt", "Array", "Object",
    "Result", "Option",
];

pub fn format_type_params(type_params: &[String]) -> String {
    if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
    }
}

pub fn fn_signature(
    name: &str,
    type_params: &[String],
    params: &[FnParam],
    return_type: Option<&KabType>,
) -> String {
    let tparams = format_type_params(type_params);
    let ps: Vec<String> = params
        .iter()
        .map(|p| {
            if let Some(t) = &p.type_ann {
                format!("{}: {}", p.name, format_kab_type(t))
            } else {
                p.name.clone()
            }
        })
        .collect();
    let ret = return_type
        .map(|t| format!(" -> {}", format_kab_type(t)))
        .unwrap_or_default();
    format!("fn {name}{tparams}({}){ret}", ps.join(", "))
}

fn format_kab_type(t: &KabType) -> String {
    match t {
        KabType::Named(n) => n.clone(),
        KabType::Ref(inner) => format!("&{}", format_kab_type(inner)),
        KabType::RefMut(inner) => format!("&mut {}", format_kab_type(inner)),
    }
}

pub fn class_signature(name: &str, type_params: &[String]) -> String {
    format!("class {name}{}", format_type_params(type_params))
}

pub fn enum_signature(name: &str, type_params: &[String]) -> String {
    format!("enum {name}{}", format_type_params(type_params))
}

pub fn method_signature(
    class: &str,
    name: &str,
    type_params: &[String],
    params: &[String],
) -> String {
    format!(
        "{class}.{name}{}({})",
        format_type_params(type_params),
        params.join(", ")
    )
}

pub fn demangle_name(name: &str) -> (String, Option<Vec<String>>) {
    if let Some(idx) = name.find('$') {
        let base = name[..idx].to_string();
        let types: Vec<String> = name[idx + 1..].split('_').map(String::from).collect();
        (base, Some(types))
    } else {
        (name.to_string(), None)
    }
}

pub fn specialized_signature_note(base_sig: &str, types: &[String]) -> String {
    format!(
        "{base_sig}\n\nSpecialization: `{}`",
        types.join("$")
    )
}

pub fn generic_word_at(line: u32, character: u32, text: &str) -> String {
    let line_text = text.lines().nth(line as usize).unwrap_or("");
    let col = (character as usize).min(line_text.len());
    let chars: Vec<char> = line_text.chars().collect();
    let mut start = col;
    while start > 0 && is_generic_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_generic_ident_char(chars[end]) {
        end += 1;
    }
    chars[start..end].iter().collect()
}

fn is_generic_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

pub fn in_type_arg_context(source: &str, line: u32, column: u32) -> bool {
    let line_text = source.lines().nth(line as usize).unwrap_or("");
    let col = (column as usize).min(line_text.len());
    let before: String = line_text.chars().take(col).collect();
    if let Some(lt) = before.rfind('<') {
        let after_lt = &before[lt + 1..];
        !after_lt.contains('>')
    } else {
        false
    }
}

pub fn hover_for_symbol(word: &str, sym: &Symbol, stmts: &[Stmt]) -> Option<String> {
    match sym.kind {
        SymbolKind::TypeParam => Some(format!("`{word}` — type parameter")),
        SymbolKind::Function => lookup_fn_signature(word, stmts),
        SymbolKind::Class => lookup_class_signature(word, stmts),
        SymbolKind::Method => lookup_method_signature(word, stmts),
        _ => None,
    }
}

pub fn hover_from_ast(word: &str, stmts: &[Stmt]) -> Option<String> {
    let (base, spec) = demangle_name(word);
    let lookup = if spec.is_some() { base.as_str() } else { word };

    if let Some(sig) = lookup_fn_signature(lookup, stmts) {
        if let Some(types) = &spec {
            return Some(specialized_signature_note(&sig, types));
        }
        return Some(sig);
    }
    if let Some(sig) = lookup_class_signature(lookup, stmts) {
        if let Some(types) = &spec {
            return Some(specialized_signature_note(&sig, types));
        }
        return Some(sig);
    }
    lookup_enum_signature(lookup, stmts).or_else(|| lookup_method_signature(word, stmts))
}

/// When the cursor is on a method name after `.`, return `(receiver, method)`.
pub fn member_context_at(source: &str, line: u32, column: u32, word: &str) -> Option<(String, String)> {
    let line_text = source.lines().nth(line as usize)?;
    let col = (column as usize).min(line_text.len());
    let chars: Vec<char> = line_text.chars().collect();

    let mut start = col;
    while start > 0 && is_generic_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_generic_ident_char(chars[end]) {
        end += 1;
    }
    let method: String = chars[start..end].iter().collect();
    if method != word || method.is_empty() {
        return None;
    }

    let mut dot = start;
    while dot > 0 && chars[dot - 1].is_whitespace() {
        dot -= 1;
    }
    if dot == 0 || chars[dot - 1] != '.' {
        return None;
    }

    let mut rend = dot - 1;
    while rend > 0 && chars[rend - 1].is_whitespace() {
        rend -= 1;
    }
    let mut rstart = rend;
    while rstart > 0 && is_generic_ident_char(chars[rstart - 1]) {
        rstart -= 1;
    }
    let receiver: String = chars[rstart..rend].iter().collect();
    if receiver.is_empty() {
        return None;
    }
    Some((receiver, method))
}

pub fn collect_type_env(stmts: &[Stmt]) -> std::collections::HashMap<String, String> {
    let mut env = std::collections::HashMap::new();
    walk_stmts(stmts, &mut |stmt| {
        if let Stmt::Let {
            pattern,
            init: Some(init),
            ..
        } = stmt
        {
            if let BindingPattern::Name(name) = pattern {
                if let Some(t) = infer_expr_type(init, &env, stmts) {
                    env.insert(name.clone(), t);
                }
            }
        }
    });
    env
}

fn infer_expr_type(
    expr: &Expr,
    env: &std::collections::HashMap<String, String>,
    stmts: &[Stmt],
) -> Option<String> {
    match expr {
        Expr::Literal(lit) => infer_literal_type(lit),
        Expr::Variable(name) => env.get(name).cloned(),
        Expr::Call {
            func,
            type_args,
            args,
            ..
        } => {
            if let Expr::Variable(class_name) = func.as_ref() {
                if !type_args.is_empty() {
                    return Some(mangle(class_name, type_args));
                }
                let mut inferred = Vec::new();
                for arg in args.iter() {
                    if let CallArg::Expr(e) = arg {
                        if let Some(t) = infer_expr_type(e, env, stmts) {
                            inferred.push(t);
                        }
                    }
                }
                if class_has_type_params(class_name, stmts) {
                    if !inferred.is_empty() {
                        return Some(mangle(class_name, &inferred));
                    }
                    return None;
                }
                if class_exists(class_name, stmts) {
                    return Some(class_name.clone());
                }
            }
            None
        }
        _ => None,
    }
}

fn infer_literal_type(lit: &Literal) -> Option<String> {
    match lit {
        Literal::Number(_) => Some("Number".into()),
        Literal::Float(_) => Some("Float".into()),
        Literal::BigInt(_) => Some("BigInt".into()),
        Literal::String(_) => Some("String".into()),
        Literal::Bool(_) => Some("Bool".into()),
        Literal::Null => Some("Null".into()),
        Literal::Undefined => Some("Undefined".into()),
        _ => None,
    }
}

fn class_has_type_params(name: &str, stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        if let Stmt::Class {
            name: n,
            type_params,
            ..
        } = stmt
        {
            if n == name {
                return !type_params.is_empty();
            }
        }
    }
    false
}

fn class_exists(name: &str, stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        if let Stmt::Class { name: n, .. } = stmt {
            if n == name {
                return true;
            }
        }
    }
    false
}

pub fn hover_method_with_receiver(
    class_type: &str,
    method: &str,
    stmts: &[Stmt],
) -> Option<String> {
    let (class_base, _) = demangle_name(class_type);
    for stmt in stmts {
        if let Stmt::Class {
            name,
            methods,
            ..
        } = stmt
        {
            if name == class_base.as_str() {
                for ClassMethod {
                    name: mname,
                    type_params,
                    params,
                    ..
                } in methods
                {
                    if mname == method {
                        let sig = method_signature(class_type, mname, type_params, params);
                        return Some(format!("Receiver: `{class_type}`\n\n{sig}"));
                    }
                }
            }
        }
    }
    None
}

pub fn hover_member_at(source: &str, line: u32, column: u32, word: &str) -> Option<String> {
    let (receiver, method) = member_context_at(source, line, column, word)?;
    let (stmts, _) = parse_for_lsp(source)?;
    let env = collect_type_env(&stmts);
    let class_type = env.get(&receiver)?;
    hover_method_with_receiver(class_type, &method, &stmts)
}

fn lookup_fn_signature(name: &str, stmts: &[Stmt]) -> Option<String> {
    let mut found = None;
    walk_stmts(stmts, &mut |stmt| {
        if let Stmt::Expr(Expr::Function {
            name: n,
            type_params,
            params,
            return_type,
            ..
        }) = stmt
        {
            if n == name {
                found = Some(fn_signature(n, type_params, params, return_type.as_ref()));
            }
        }
    });
    found
}

fn lookup_class_signature(name: &str, stmts: &[Stmt]) -> Option<String> {
    for stmt in stmts {
        if let Stmt::Class {
            name: n,
            type_params,
            ..
        } = stmt
        {
            if n == name {
                return Some(class_signature(n, type_params));
            }
        }
    }
    None
}

fn lookup_enum_signature(name: &str, stmts: &[Stmt]) -> Option<String> {
    for stmt in stmts {
        if let Stmt::Enum {
            name: n,
            type_params,
            ..
        } = stmt
        {
            if n == name {
                return Some(enum_signature(n, type_params));
            }
        }
    }
    None
}

fn lookup_method_signature(name: &str, stmts: &[Stmt]) -> Option<String> {
    let (base, spec_types) = demangle_name(name);
    let method_name = base.as_str();

    for stmt in stmts {
        if let Stmt::Class {
            name: class_name,
            methods,
            ..
        } = stmt
        {
            for ClassMethod {
                name: mname,
                type_params,
                params,
                ..
            } in methods
            {
                if mname == method_name {
                    let sig = method_signature(class_name, mname, type_params, params);
                    if let Some(types) = spec_types {
                        return Some(specialized_signature_note(&sig, &types));
                    }
                    return Some(sig);
                }
            }
        }
    }
    None
}

fn walk_stmts(stmts: &[Stmt], f: &mut dyn FnMut(&Stmt)) {
    for stmt in stmts {
        f(stmt);
        if let Stmt::Expr(Expr::Block(body)) = stmt {
            walk_stmts(body, f);
        }
    }
}

pub fn collect_generic_templates(stmts: &[Stmt]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Expr(Expr::Function {
                name,
                type_params,
                ..
            }) if !type_params.is_empty() => {
                out.push((name.clone(), format!("fn {name}{}", format_type_params(type_params))));
            }
            Stmt::Class {
                name,
                type_params,
                ..
            } if !type_params.is_empty() => {
                out.push((name.clone(), class_signature(name, type_params)));
            }
            Stmt::Enum {
                name,
                type_params,
                ..
            } if !type_params.is_empty() => {
                out.push((name.clone(), enum_signature(name, type_params)));
            }
            _ => {}
        }
    }
    out
}

pub fn parse_for_lsp(source: &str) -> Option<(Vec<Stmt>, Vec<Symbol>)> {
    let tokens = tokenize(source).ok()?;
    let mut parser = Parser::with_eof(tokens);
    let stmts = parser.parse_program().ok()?;
    let symbols = parser.symbols().to_vec();
    Some((stmts, symbols))
}

pub fn symbol_covering<'a>(
    symbols: &'a [Symbol],
    word: &str,
    line: u32,
    column: u32,
) -> Option<&'a Symbol> {
    let line1 = line + 1;
    let column1 = column + 1;
    symbols.iter().find(|sym| {
        sym.name == word
            && sym.span.line == line1
            && column1 >= sym.span.column
            && column1 < sym.span.column + sym.span.len
    })
}
