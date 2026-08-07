//! Kv8 native API registration.

use super::context::Kv8Context;
use super::eval::{dom_to_kabootar, eval_script};
use crate::runtime::kabootar_dom::assign_ids;
use crate::runtime::render::{frame_to_object, RenderEngine};
use crate::value::{Environment, Value};
use std::collections::HashMap;

fn kv8_create_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Kv8Context(Kv8Context::default()))
}

fn kv8_eval_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let script = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kv8_eval(ctx, script) expects string".into()),
    };
    let result = eval_script(&ctx, script)?;
    Ok(kv8_value_to_kabootar(&result))
}

/// C3: self-host `evalSource` hot path is Kab `evalSourceWith` (H6a — no kv8_eval_source).
fn kv8_css_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let css = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kv8_css(ctx, css) expects string".into()),
    };
    Ok(Value::Number(ctx.set_stylesheet(css)? as i64))
}

fn kv8_dom_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    Ok(dom_to_kabootar(&ctx.root_dom()?))
}

fn kv8_paint_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let w = args.get(1).and_then(|v| match v {
        Value::Number(n) => Some(*n as f64),
        _ => None,
    }).unwrap_or(1280.0);
    let h = args.get(2).and_then(|v| match v {
        Value::Number(n) => Some(*n as f64),
        _ => None,
    }).unwrap_or(720.0);
    let root = ctx.root_dom()?;
    let sheet = ctx.with_mut(|inner| Ok(inner.stylesheet.clone()))?;
    let mut engine = RenderEngine::with_viewport(w, h);
    engine.set_stylesheet(sheet);
    let frame = engine.compose(&root);
    crate::runtime::frame_buffer::publish_frame(frame.clone());
    Ok(Value::from_object(frame_to_object(&frame)))
}

fn kv8_computed_style_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let node = match args.get(1) {
        Some(Value::KabootarDom(n)) => n,
        _ => return Err("kv8_computed_style(ctx, node) expects DOM node".into()),
    };
    let style = ctx.computed_style(node)?;
    let mut o = HashMap::new();
    o.insert("display".into(), Value::String(style.display));
    o.insert("color".into(), Value::String(style.color));
    o.insert("background".into(), Value::String(style.background));
    o.insert("fontSize".into(), Value::String(style.font_size));
    Ok(Value::from_object(o))
}

fn kv8_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut o = HashMap::new();
    o.insert("engine".into(), Value::String("kv8".into()));
    o.insert("version".into(), Value::String("0.2.0".into()));
    o.insert("dom".into(), Value::String("kdom".into()));
    o.insert("css".into(), Value::String("kss".into()));
    o.insert("js_subset".into(), Value::Bool(true));
    o.insert("jit".into(), Value::Bool(true));
    o.insert("arrow_bytecode".into(), Value::Bool(true));
    o.insert("vfs_kv8".into(), Value::Bool(true));
    o.insert("hot_path_predictor".into(), Value::Bool(true));
    o.insert("inline_cache".into(), Value::Bool(true));
    o.insert("dom_index".into(), Value::Bool(true));
    o.insert("style_cache".into(), Value::Bool(true));
    o.insert("program_cache".into(), Value::Bool(true));
    o.insert("ownership_gc".into(), Value::String("rust-no-pause".into()));
    o.insert("zero_copy_dom".into(), Value::String("singleton+index".into()));
    Ok(Value::from_object(o))
}

fn kv8_opt_info_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    ctx.with_mut(|inner| {
        let mut o = HashMap::new();
        o.insert(
            "program_cache".into(),
            Value::Number(inner.opt.program_cache.len() as i64),
        );
        o.insert(
            "arrow_cache".into(),
            Value::Number(inner.opt.arrow_cache.len() as i64),
        );
        o.insert(
            "style_cache".into(),
            Value::Number(inner.opt.style_cache.len() as i64),
        );
        o.insert(
            "dom_nodes_indexed".into(),
            Value::Number(inner.opt.dom_paths.len() as i64),
        );
        o.insert(
            "hot_members".into(),
            Value::Number(inner.opt.predictor.hot_members() as i64),
        );
        o.insert(
            "hot_calls".into(),
            Value::Number(inner.opt.predictor.hot_calls() as i64),
        );
        let (compiled, hits) = inner.jit.as_ref().map(|j| j.stats()).unwrap_or((0, 0));
        o.insert("compiled_loops".into(), Value::Number(compiled as i64));
        o.insert("loop_hits".into(), Value::Number(hits as i64));
        Ok(Value::from_object(o))
    })
}

fn kv8_jit_info_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let (compiled, hits) = ctx.with_mut(|inner| {
        let jit = inner.jit.as_ref();
        Ok(jit.map(|j| j.stats()).unwrap_or((0, 0)))
    })?;
    let mut o = HashMap::new();
    o.insert("compiled_loops".into(), Value::Number(compiled as i64));
    o.insert("loop_hits".into(), Value::Number(hits as i64));
    Ok(Value::from_object(o))
}

fn kv8_load_vfs_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let path = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("kv8_load_vfs(ctx, path)".into()),
    };
    let os = env.get("os").ok_or("os handle required")?;
    let Value::OsHandle(handle) = os else {
        return Err("os handle required".into());
    };
    let node = super::vfs_module::load_vfs_module(&ctx, &handle, path)?;
    Ok(Value::KabootarDom(node))
}

fn kv8_run_ui_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let kml = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("kv8_run_ui(ctx, kml, css) expects KML string".into()),
    };
    let css = args.get(2).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }).unwrap_or_default();
    let mut root = crate::kml::parse_kml(&kml)?;
    assign_ids(&mut root);
    ctx.with_mut(|inner| {
        inner.document.root = root;
        inner.css_text = css.clone();
        inner.stylesheet = crate::runtime::kstyle::parse_stylesheet(&css);
        Ok(())
    })?;
    kv8_paint_native(&[Value::Kv8Context(ctx), Value::Number(1280), Value::Number(720)], env)
}

fn kv8_run_html_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let kml = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("kv8_run_html(kml) expects string".into()),
    };
    let css = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }).unwrap_or_default();
    let ctx = Kv8Context::default();
    kv8_run_ui_native(
        &[Value::Kv8Context(ctx.clone()), Value::String(kml), Value::String(css)],
        env,
    )?;
    Ok(Value::Kv8Context(ctx))
}

fn kv8_drain_timers_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let n = super::eval::drain_timers(&ctx)?;
    Ok(Value::Number(n))
}

fn kv8_drain_event_loop_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = expect_ctx(args, 0)?;
    let n = super::eval::drain_event_loop(&ctx)?;
    Ok(Value::Number(n))
}

fn kv8_self_hosting_probe_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = match args.first() {
        Some(Value::Kv8Context(c)) => c.clone(),
        None => Kv8Context::default(),
        _ => return Err("kv8_self_hosting_probe(ctx?) expects Kv8Context or no args".into()),
    };
    let results = super::smoke::run_all_probes(&ctx);
    Ok(super::smoke::probe_report_value(&results))
}

fn kv8_minimum_app_shell_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = match args.first() {
        Some(Value::Kv8Context(c)) => c.clone(),
        None => Kv8Context::default(),
        _ => return Err("kv8_minimum_app_shell(ctx?) expects Kv8Context or no args".into()),
    };
    let root = super::smoke::minimum_app_shell(&ctx)?;
    Ok(kv8_value_to_kabootar(&root))
}

fn kv8_react_smoke_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = match args.first() {
        Some(Value::Kv8Context(c)) => c.clone(),
        None => Kv8Context::default(),
        _ => return Err("kv8_react_smoke(ctx?) expects Kv8Context or no args".into()),
    };
    let n = super::smoke::react_smoke_path(&ctx)?;
    Ok(kv8_value_to_kabootar(&n))
}

fn kv8_react_bundle_smoke_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = match args.first() {
        Some(Value::Kv8Context(c)) => c.clone(),
        None => Kv8Context::default(),
        _ => return Err("kv8_react_bundle_smoke(ctx?) expects Kv8Context or no args".into()),
    };
    let n = super::smoke::react_bundle_smoke_path(&ctx)?;
    Ok(kv8_value_to_kabootar(&n))
}

fn kv8_react_bundle_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(super::bundle::react_bundle_info())
}

fn kv8_load_react_runtime_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = match args.first() {
        Some(Value::Kv8Context(c)) => c.clone(),
        None => Kv8Context::default(),
        _ => return Err("kv8_load_react_runtime(ctx?) expects Kv8Context or no args".into()),
    };
    let react = super::bundle::load_react_runtime(&ctx)?;
    Ok(kv8_value_to_kabootar(&react))
}

fn kv8_react_runtime_bundle_smoke_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = match args.first() {
        Some(Value::Kv8Context(c)) => c.clone(),
        None => Kv8Context::default(),
        _ => return Err("kv8_react_runtime_bundle_smoke(ctx?) expects Kv8Context or no args".into()),
    };
    let n = super::bundle::react_runtime_bundle_smoke(&ctx)?;
    Ok(kv8_value_to_kabootar(&n))
}

fn kv8_load_react_dom_runtime_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ctx = match args.first() {
        Some(Value::Kv8Context(c)) => c.clone(),
        None => Kv8Context::default(),
        _ => return Err("kv8_load_react_dom_runtime(ctx?) expects Kv8Context or no args".into()),
    };
    let react_dom = super::bundle::load_react_dom_runtime(&ctx)?;
    Ok(kv8_value_to_kabootar(&react_dom))
}

fn kv8_load_react_umd_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    kv8_load_react_runtime_native(args, env)
}

fn kv8_react_umd_bundle_smoke_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    kv8_react_runtime_bundle_smoke_native(args, env)
}

fn kv8_load_react_dom_umd_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    kv8_load_react_dom_runtime_native(args, env)
}

fn expect_ctx(args: &[Value], i: usize) -> Result<Kv8Context, String> {
    match args.get(i) {
        Some(Value::Kv8Context(c)) => Ok(c.clone()),
        _ => Err("expected Kv8Context handle".into()),
    }
}

fn kv8_value_to_kabootar(v: &super::context::Kv8Value) -> Value {
    use super::context::Kv8Value;
    match v {
        Kv8Value::Undefined => Value::Undefined,
        Kv8Value::Null => Value::Null,
        Kv8Value::Bool(b) => Value::Bool(*b),
        Kv8Value::Num(n) => {
            if n.is_finite() && n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                Value::Number(*n as i64)
            } else {
                Value::Float(*n)
            }
        }
        Kv8Value::Str(s) => Value::String(s.clone()),
        Kv8Value::Dom(n) => Value::KabootarDom(n.clone()),
        Kv8Value::Fun { .. } | Kv8Value::Arrow { .. } | Kv8Value::AsyncFun { .. } => Value::String("<function>".into()),
        Kv8Value::Promise(_) => Value::String("<promise>".into()),
        Kv8Value::Symbol { key, .. } => Value::String(format!("Symbol({key})")),
        Kv8Value::Obj(m) => Value::from_object(
            m.iter()
                .filter(|(k, _)| *k != "__native")
                .map(|(k, v)| (k.clone(), kv8_value_to_kabootar(v)))
                .collect(),
        ),
    }
}

pub fn kv8_globals(env: &mut Environment) {
    env.set("kv8_info".into(), Value::NativeFunction(kv8_info_native));
    env.set("kv8_create".into(), Value::NativeFunction(kv8_create_native));
    env.set("kv8_eval".into(), Value::NativeFunction(kv8_eval_native));
    env.set("kv8_css".into(), Value::NativeFunction(kv8_css_native));
    env.set("kv8_dom".into(), Value::NativeFunction(kv8_dom_native));
    env.set("kv8_paint".into(), Value::NativeFunction(kv8_paint_native));
    env.set("kv8_computed_style".into(), Value::NativeFunction(kv8_computed_style_native));
    env.set("kv8_run_ui".into(), Value::NativeFunction(kv8_run_ui_native));
    env.set("kv8_run_html".into(), Value::NativeFunction(kv8_run_html_native));
    env.set("kv8_jit_info".into(), Value::NativeFunction(kv8_jit_info_native));
    env.set("kv8_opt_info".into(), Value::NativeFunction(kv8_opt_info_native));
    env.set("kv8_load_vfs".into(), Value::NativeFunction(kv8_load_vfs_native));
    env.set(
        "kv8_drain_timers".into(),
        Value::NativeFunction(kv8_drain_timers_native),
    );
    env.set(
        "kv8_drain_event_loop".into(),
        Value::NativeFunction(kv8_drain_event_loop_native),
    );
    env.set(
        "kv8_self_hosting_probe".into(),
        Value::NativeFunction(kv8_self_hosting_probe_native),
    );
    env.set(
        "kv8_minimum_app_shell".into(),
        Value::NativeFunction(kv8_minimum_app_shell_native),
    );
    env.set(
        "kv8_react_smoke".into(),
        Value::NativeFunction(kv8_react_smoke_native),
    );
    env.set(
        "kv8_react_bundle_smoke".into(),
        Value::NativeFunction(kv8_react_bundle_smoke_native),
    );
    env.set(
        "kv8_react_runtime_bundle_smoke".into(),
        Value::NativeFunction(kv8_react_runtime_bundle_smoke_native),
    );
    env.set(
        "kv8_react_umd_bundle_smoke".into(),
        Value::NativeFunction(kv8_react_umd_bundle_smoke_native),
    );
    env.set(
        "kv8_react_bundle_info".into(),
        Value::NativeFunction(kv8_react_bundle_info_native),
    );
    env.set(
        "kv8_load_react_runtime".into(),
        Value::NativeFunction(kv8_load_react_runtime_native),
    );
    env.set(
        "kv8_load_react_umd".into(),
        Value::NativeFunction(kv8_load_react_umd_native),
    );
    env.set(
        "kv8_load_react_dom_runtime".into(),
        Value::NativeFunction(kv8_load_react_dom_runtime_native),
    );
    env.set(
        "kv8_load_react_dom_umd".into(),
        Value::NativeFunction(kv8_load_react_dom_umd_native),
    );
}

pub fn kv8_register(env: &mut Environment) {
    kv8_globals(env);
}
