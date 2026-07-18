//! KSS natives for Kabootar `kstyle { }` blocks.

use crate::runtime::kstyle::{StyleRule, Stylesheet};
use crate::value::{Environment, Value};
use std::cell::RefCell;

thread_local! {
    static KSTYLE_BUILDER: RefCell<Stylesheet> = RefCell::new(Stylesheet::default());
}

pub fn kstyle_lang_globals(env: &mut Environment) {
    env.set("kstyle_reset".into(), Value::NativeFunction(kstyle_reset_native));
    env.set("kstyle_rule".into(), Value::NativeFunction(kstyle_rule_native));
    env.set("kstyle_commit".into(), Value::NativeFunction(kstyle_commit_native));
    env.set("kstyle_css".into(), Value::NativeFunction(kstyle_css_native));
}

fn kstyle_css_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    // Prefer the thread-local builder so CSS is visible across module envs
    // after kstyle_commit (theme.applyDark → global kstyle_css).
    let from_builder = KSTYLE_BUILDER.with(|b| sheet_to_css(&b.borrow()));
    if !from_builder.is_empty() {
        return Ok(Value::String(from_builder));
    }
    Ok(env
        .get("__kstyle")
        .unwrap_or(Value::String(String::new())))
}

fn kstyle_reset_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    KSTYLE_BUILDER.with(|b| *b.borrow_mut() = Stylesheet::default());
    Ok(Value::Null)
}

fn kstyle_rule_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let sel = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("kstyle_rule(selector, prop, value)".into()),
    };
    let prop = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("kstyle_rule prop".into()),
    };
    let val = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("kstyle_rule value".into()),
    };
    KSTYLE_BUILDER.with(|b| {
        let mut sheet = b.borrow_mut();
        if let Some(rule) = sheet.rules.iter_mut().find(|r| r.selector == sel) {
            rule.declarations.insert(prop, val);
        } else {
            let mut decls = std::collections::HashMap::new();
            decls.insert(prop, val);
            sheet.rules.push(StyleRule {
                selector: sel,
                declarations: decls,
            });
        }
        Ok(Value::Null)
    })
}

fn kstyle_commit_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let sheet = KSTYLE_BUILDER.with(|b| b.borrow().clone());
    let css = sheet_to_css(&sheet);
    env.set("__kstyle".into(), Value::String(css));
    Ok(Value::Number(sheet.rules.len() as i64))
}

fn sheet_to_css(sheet: &Stylesheet) -> String {
    let mut out = String::new();
    for rule in &sheet.rules {
        out.push_str(&rule.selector);
        out.push_str(" { ");
        for (k, v) in &rule.declarations {
            out.push_str(k);
            out.push_str(": ");
            out.push_str(v);
            out.push_str("; ");
        }
        out.push_str("}\n");
    }
    out
}

pub fn apply_sheet_to_env(env: &mut Environment, sheet: &Stylesheet) {
    env.set("__kstyle".into(), Value::String(sheet_to_css(sheet)));
}
