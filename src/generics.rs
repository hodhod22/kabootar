//! Native Kabootar generics — monomorphization helpers (v1: fn-only).

use crate::ast::{Expr, KabType, Literal};
use std::collections::{HashMap, HashSet};

/// Known compile-time types for generic inference (G6: locals/globals + class ctor names).
#[derive(Default, Clone)]
pub struct TypeInferenceCtx {
    bindings: HashMap<String, String>,
    class_names: HashSet<String>,
}

impl TypeInferenceCtx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(&mut self, name: impl Into<String>, ty: impl Into<String>) {
        self.bindings.insert(name.into(), ty.into());
    }

    pub fn add_class(&mut self, name: impl Into<String>) {
        self.class_names.insert(name.into());
    }

    pub fn lookup(&self, name: &str) -> Option<&str> {
        self.bindings.get(name).map(String::as_str)
    }

    pub fn is_class(&self, name: &str) -> bool {
        self.class_names.contains(name)
    }
}

/// Template map key for a generic class method (`Counter$get`).
pub fn generic_method_template_key(class_name: &str, method_name: &str) -> String {
    format!("{class_name}${method_name}")
}

/// Monomorphized method name stored on the class (`get$Number`).
pub fn mangle_method(method_name: &str, type_names: &[String]) -> String {
    mangle(method_name, type_names)
}

/// Class name of the receiver for generic method dispatch (instance binding or ctor call).
pub fn infer_receiver_class_name(expr: &Expr, ctx: &TypeInferenceCtx) -> Option<String> {
    infer_type_from_expr(expr, Some(ctx)).filter(|t| ctx.is_class(t))
}

/// Runtime-style type name for monomorphization (`id$Number`, `pair$Number_String`).
pub fn mangle(base: &str, type_names: &[String]) -> String {
    if type_names.is_empty() {
        return base.to_string();
    }
    format!("{}${}", base, type_names.join("_"))
}

/// Resolve `Base<T>` against concrete specialization args (`Child$Number` → `Base$Number`).
pub fn resolve_type_ref(
    base: &str,
    type_args: &[String],
    template_params: &[String],
    concrete_args: &[String],
) -> String {
    if type_args.is_empty() {
        return base.to_string();
    }
    let mut resolved = Vec::new();
    for arg in type_args {
        if let Some(i) = template_params.iter().position(|p| p == arg) {
            resolved.push(concrete_args[i].clone());
        } else {
            resolved.push(arg.clone());
        }
    }
    mangle(base, &resolved)
}

/// Infer a concrete type name from a compile-time expression.
pub fn infer_type_from_expr(expr: &Expr, ctx: Option<&TypeInferenceCtx>) -> Option<String> {
    match expr {
        Expr::Literal(lit) => Some(infer_type_from_literal(lit)),
        Expr::Variable(name) => ctx.and_then(|c| c.lookup(name).map(String::from)),
        Expr::Call { func, .. } => {
            if let Expr::Variable(name) = func.as_ref() {
                if ctx.map(|c| c.is_class(name)).unwrap_or(false) {
                    return Some(name.clone());
                }
            }
            None
        }
        _ => None,
    }
}

pub fn infer_type_from_literal(lit: &Literal) -> String {
    match lit {
        Literal::Number(_) => "Number".into(),
        Literal::BigInt(_) => "BigInt".into(),
        Literal::Float(_) => "Float".into(),
        Literal::String(_) => "String".into(),
        Literal::Bool(_) => "Bool".into(),
        Literal::Null => "Null".into(),
        Literal::Undefined => "Undefined".into(),
        Literal::Nan => "Float".into(),
        Literal::Array(_) => "Array".into(),
        Literal::Object(_) => "Object".into(),
        Literal::Ok(_) => "Result".into(),
        Literal::Err(_) => "Result".into(),
        Literal::Some(_) => "Option".into(),
        Literal::None => "Option".into(),
    }
}

pub fn kab_type_name(ty: &KabType) -> &str {
    match ty {
        KabType::Named(name) => name.as_str(),
    }
}

/// Resolve type arguments for a generic call (explicit or inferred from arguments).
pub fn resolve_type_args(
    fn_name: &str,
    type_params: &[String],
    explicit: &[String],
    arg_exprs: &[Expr],
    ctx: Option<&TypeInferenceCtx>,
) -> Result<Vec<String>, String> {
    if !explicit.is_empty() {
        if explicit.len() != type_params.len() {
            return Err(format!(
                "{fn_name} expects {} type argument(s), got {}",
                type_params.len(),
                explicit.len()
            ));
        }
        return Ok(explicit.to_vec());
    }
    if type_params.len() == 1 && arg_exprs.len() == 1 {
        if let Some(t) = infer_type_from_expr(&arg_exprs[0], ctx) {
            return Ok(vec![t]);
        }
    }
    if type_params.len() == arg_exprs.len() {
        let mut inferred = Vec::new();
        for arg in arg_exprs {
            inferred.push(
                infer_type_from_expr(arg, ctx).ok_or_else(|| {
                    format!("Cannot infer type for {fn_name}; use {fn_name}<Type>(...)")
                })?,
            );
        }
        return Ok(inferred);
    }
    Err(format!(
        "Cannot infer type for {fn_name}; use {}<{}>(...)",
        fn_name,
        type_params.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mangle_single_and_pair() {
        assert_eq!(mangle("id", &["Number".into()]), "id$Number");
        assert_eq!(
            mangle("pair", &["Number".into(), "String".into()]),
            "pair$Number_String"
        );
    }

    #[test]
    fn infer_from_bound_variable() {
        let mut ctx = TypeInferenceCtx::new();
        ctx.bind("n", "Number");
        let expr = Expr::Variable("n".into());
        assert_eq!(
            infer_type_from_expr(&expr, Some(&ctx)),
            Some("Number".into())
        );
    }

    #[test]
    fn infer_from_class_ctor_call() {
        let mut ctx = TypeInferenceCtx::new();
        ctx.add_class("Point");
        let expr = Expr::Call {
            func: Box::new(Expr::Variable("Point".into())),
            type_args: vec![],
            args: vec![],
        };
        assert_eq!(
            infer_type_from_expr(&expr, Some(&ctx)),
            Some("Point".into())
        );
    }

    #[test]
    fn resolve_id_from_variable_binding() {
        let mut ctx = TypeInferenceCtx::new();
        ctx.bind("n", "Number");
        let args = vec![Expr::Variable("n".into())];
        let resolved = resolve_type_args("id", &["T".into()], &[], &args, Some(&ctx)).unwrap();
        assert_eq!(resolved, vec!["Number"]);
    }
}
