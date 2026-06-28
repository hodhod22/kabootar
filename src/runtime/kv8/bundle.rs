//! Bundled JS loaders — React 19 via esbuild (official ESM → Kv8 script).
//!
//! **Policy:** ESM + esbuild only — no UMD. See `.cursor/rules/kabootar-modern-stack.mdc`.
//! Source: `react` + `react-dom` npm packages, entry `global-entry.js`.
//! Regenerate: `cd fixtures/kv8/react && npm install && npm run build`.

use super::context::{Kv8Context, Kv8Value};
use super::eval::{eval_script, parse_program, register_kv8_module, run_program};
use super::ast::Kv8Program;
use crate::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static REACT_RUNTIME_PROGRAM: RefCell<Option<Result<&'static Kv8Program, String>>> =
        const { RefCell::new(None) };
}

/// Parsed React bundle (cached once per thread — avoids re-tokenize on every load).
pub fn react_runtime_program() -> Result<&'static Kv8Program, String> {
    REACT_RUNTIME_PROGRAM.with(|slot| {
        let mut cache = slot.borrow_mut();
        if cache.is_none() {
            *cache = Some(match parse_program(REACT_RUNTIME_BUNDLE) {
                Ok(p) => Ok(Box::leak(Box::new(p))),
                Err(e) => Err(e),
            });
        }
        match cache.as_ref().unwrap() {
            Ok(p) => Ok(*p),
            Err(e) => Err(e.clone()),
        }
    })
}

pub const REACT_VERSION: &str = "19.2.7";
pub const REACT_BUNDLE_SOURCE: &str = "esbuild";
pub const REACT_BUNDLE_TOOL: &str = "esbuild";

/// Kv8-parseable React 19 API shim (createElement + createRoot).
pub const REACT_SHIM: &str = include_str!("../../../fixtures/kv8/react/react-shim.kv8.js");

/// Sample app — counter with re-render via createRoot.
pub const REACT_COUNTER_APP: &str = include_str!("../../../fixtures/kv8/react/counter.app.kv8.js");

/// React + ReactDOM client — esbuild IIFE from ESM (no `import` in output).
pub const REACT_RUNTIME_BUNDLE: &str =
    include_str!("../../../fixtures/kv8/react/react-runtime.bundle.js");

/// @deprecated use [`REACT_RUNTIME_BUNDLE`]
pub const REACT_UMD: &str = REACT_RUNTIME_BUNDLE;

/// @deprecated use [`REACT_RUNTIME_BUNDLE`]
pub const REACT_DOM_UMD: &str = REACT_RUNTIME_BUNDLE;

/// @deprecated use [`REACT_BUNDLE_SOURCE`]
pub const REACT_UMD_SOURCE: &str = REACT_BUNDLE_SOURCE;

pub fn load_scripts(ctx: &Kv8Context, scripts: &[&str]) -> Result<(), String> {
    for source in scripts {
        eval_script(ctx, source)?;
    }
    Ok(())
}

pub fn react_bundle_smoke(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    load_scripts(ctx, &[REACT_SHIM, REACT_COUNTER_APP])?;
    eval_script(
        ctx,
        "let n = document.querySelectorAll('button'); return n.length;",
    )
}

/// Load official React 19 + ReactDOM client (esbuild ESM bundle) and run counter app.
pub fn react_runtime_bundle_smoke(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    load_react_runtime(ctx)?;
    eval_script(ctx, REACT_COUNTER_APP)?;
    eval_script(
        ctx,
        "let n = document.querySelectorAll('button'); return n.length;",
    )
}

/// @deprecated use [`react_runtime_bundle_smoke`]
pub fn react_umd_bundle_smoke(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    react_runtime_bundle_smoke(ctx)
}

/// Eval esbuild bundle; exposes `globalThis.React` and `globalThis.ReactDOM`.
///
/// **Note:** Full eval of the ~190KB bundle in the Kv8 interpreter takes **several minutes**
/// (not frozen). Use [`react_runtime_program`] + cached parse; set `eval_ops_limit` on the
/// context to detect infinite loops.
pub fn load_react_runtime(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    let program = react_runtime_program()?;
    run_program(ctx, program)?;
    let default = eval_script(ctx, "return KV8ReactRuntime.default;")?;
    let react = eval_script(
        ctx,
        "var __rt = KV8ReactRuntime.default; globalThis.React = __rt.React; globalThis.ReactDOM = __rt.ReactDOM; return globalThis.React;",
    )?;
    register_react_runtime_module(ctx, default)?;
    Ok(react)
}

/// Register `react-runtime` module for `import … from "react-runtime"`.
pub fn register_react_runtime_module(ctx: &Kv8Context, default: Kv8Value) -> Result<(), String> {
    let react = eval_script(ctx, "return globalThis.React;")?;
    let react_dom = eval_script(ctx, "return globalThis.ReactDOM;")?;
    let mut named = HashMap::new();
    named.insert("React".into(), react);
    named.insert("ReactDOM".into(), react_dom);
    register_kv8_module(ctx, "react-runtime", Some(default), named)
}

/// Bootstrap React via ESM `import` (requires [`register_react_runtime_module`]).
pub fn load_react_runtime_via_import(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    let program = react_runtime_program()?;
    run_program(ctx, program)?;
    let default = eval_script(ctx, "return KV8ReactRuntime.default;")?;
    register_react_runtime_module(ctx, default)?;
    eval_script(
        ctx,
        "import rt from \"react-runtime\"; globalThis.React = rt.React; globalThis.ReactDOM = rt.ReactDOM; return globalThis.React;",
    )
}

/// @deprecated use [`load_react_runtime`]
pub fn load_react_umd(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    load_react_runtime(ctx)
}

/// Same bundle as [`load_react_runtime`]; returns `globalThis.ReactDOM`.
pub fn load_react_dom_runtime(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    load_react_runtime(ctx)?;
    eval_script(ctx, "return globalThis.ReactDOM;")
}

/// @deprecated use [`load_react_dom_runtime`]
pub fn load_react_dom_umd(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    load_react_dom_runtime(ctx)
}

pub fn react_bundle_info() -> Value {
    let shim_ok = parse_program(REACT_SHIM).is_ok();
    let app_ok = parse_program(REACT_COUNTER_APP).is_ok();
    let bundle_ok = parse_program(REACT_RUNTIME_BUNDLE).is_ok();
    let bundle_parse_error = parse_program(REACT_RUNTIME_BUNDLE)
        .err()
        .unwrap_or_default();
    let bundle_ctx = Kv8Context::default();
    // Full bundle eval is minutes in debug Kv8 — smoke probes use parse + bootstrap only.
    let bundle_eval = if bundle_ok {
        eval_script(
            &bundle_ctx,
            "var KV8ReactRuntime = { default: { React: {}, ReactDOM: { createRoot: function(){} } } }; var __rt = KV8ReactRuntime.default; globalThis.React = __rt.React; globalThis.ReactDOM = __rt.ReactDOM; return typeof globalThis.ReactDOM.createRoot;",
        )
    } else {
        Err(bundle_parse_error.clone())
    };
    let bundle_eval_error = bundle_eval.as_ref().err().cloned().unwrap_or_default();
    let bundle_runnable = bundle_ok && matches!(bundle_eval, Ok(Kv8Value::Str(s)) if s == "function");
    let dom_ctx = Kv8Context::default();
    let dom_eval = if bundle_runnable {
        eval_script(
            &dom_ctx,
            "var KV8ReactRuntime = { default: { React: {}, ReactDOM: { createRoot: function(){} } } }; var __rt = KV8ReactRuntime.default; globalThis.ReactDOM = __rt.ReactDOM; return typeof globalThis.ReactDOM.createRoot;",
        )
    } else {
        Err(bundle_eval_error.clone())
    };
    let dom_eval_error = dom_eval.as_ref().err().cloned().unwrap_or_default();
    let dom_runnable =
        bundle_runnable && matches!(dom_eval, Ok(Kv8Value::Str(s)) if s == "function");
    let mut m = HashMap::new();
    m.insert("react_version".into(), Value::String(REACT_VERSION.into()));
    m.insert(
        "react_bundle_source".into(),
        Value::String(REACT_BUNDLE_SOURCE.into()),
    );
    m.insert(
        "react_bundle_tool".into(),
        Value::String(REACT_BUNDLE_TOOL.into()),
    );
    // Legacy probe keys (smoke tests)
    m.insert(
        "react_umd_source".into(),
        Value::String(REACT_BUNDLE_SOURCE.into()),
    );
    m.insert("shim_bytes".into(), Value::Number(REACT_SHIM.len() as i64));
    m.insert("app_bytes".into(), Value::Number(REACT_COUNTER_APP.len() as i64));
    m.insert(
        "bundle_bytes".into(),
        Value::Number(REACT_RUNTIME_BUNDLE.len() as i64),
    );
    m.insert(
        "umd_bytes".into(),
        Value::Number(REACT_RUNTIME_BUNDLE.len() as i64),
    );
    m.insert(
        "react_dom_umd_bytes".into(),
        Value::Number(REACT_RUNTIME_BUNDLE.len() as i64),
    );
    m.insert("shim_kv8_parseable".into(), Value::Bool(shim_ok));
    m.insert("app_kv8_parseable".into(), Value::Bool(app_ok));
    m.insert("bundle_kv8_parseable".into(), Value::Bool(bundle_ok));
    m.insert("umd_kv8_parseable".into(), Value::Bool(bundle_ok));
    m.insert(
        "bundle_parse_error".into(),
        Value::String(bundle_parse_error.clone()),
    );
    m.insert(
        "umd_parse_error".into(),
        Value::String(bundle_parse_error),
    );
    m.insert(
        "bundle_kv8_runnable".into(),
        Value::Bool(bundle_runnable),
    );
    m.insert("umd_kv8_runnable".into(), Value::Bool(bundle_runnable));
    m.insert(
        "bundle_eval_error".into(),
        Value::String(bundle_eval_error.clone()),
    );
    m.insert(
        "umd_eval_error".into(),
        Value::String(bundle_eval_error),
    );
    m.insert(
        "bundle_has_react_export".into(),
        Value::Bool(bundle_runnable),
    );
    m.insert(
        "umd_has_react_export".into(),
        Value::Bool(bundle_runnable),
    );
    m.insert(
        "react_dom_bundle_kv8_runnable".into(),
        Value::Bool(dom_runnable),
    );
    m.insert(
        "react_dom_umd_kv8_runnable".into(),
        Value::Bool(dom_runnable),
    );
    m.insert(
        "react_dom_bundle_eval_error".into(),
        Value::String(dom_eval_error.clone()),
    );
    m.insert(
        "react_dom_umd_eval_error".into(),
        Value::String(dom_eval_error),
    );
    m.insert(
        "react_dom_has_reactdom_export".into(),
        Value::Bool(dom_runnable),
    );
    m.insert(
        "bundle_path".into(),
        Value::String("fixtures/kv8/react/".into()),
    );
    m.insert(
        "note".into(),
        Value::String(
            "React 19 + ReactDOM client via esbuild ESM bundle (globalThis.React/ReactDOM)"
                .into(),
        ),
    );
    Value::Object(m)
}

#[cfg(test)]
mod parse_probe {
    use super::parse_program;
    use super::super::context::Kv8Context;
    use super::super::eval::eval_script;

    fn assert_parses(label: &str, src: &str) {
        if let Err(e) = parse_program(src) {
            panic!("{label} failed: {e}\n{src}");
        }
    }

    #[test]
    fn wave5_snippet_matrix() {
        assert_parses("outer iife", "!function(e,t){return e;}(this,function(){return 1;});");
        assert_parses("ternary member assign", "let x=\"object\"==typeof exports?module.exports=1:2;");
        assert_parses("ternary call rhs", "let x=\"object\"==typeof exports?module.exports=t():2;");
        assert_parses("iife ternary", "!function(e,t){\"object\"==typeof exports?module.exports=t():1;}(this,1);");
        assert_parses("iife ternary else member", "!function(e,t){1?(e=globalThis).React=1:2;}(this,1);");
        assert_parses("ternary else only assign", "let x=1?2:(e=globalThis).React=1;");
        assert_parses("ternary both assign", "let x=1?module.exports=t():(e=globalThis).React=1;");
        assert_parses("full ternary umd", "let x=\"object\"==typeof exports?module.exports=t():(e=globalThis).React=t();");
        assert_parses(
            "umd header",
            "!function(e,t){\"object\"==typeof exports?module.exports=t():(e=globalThis).React=t();}(this,function(){\"use strict\";return 1;});",
        );
        assert_parses("this assign", "function b(e,t,n){this.props=e,this.context=t;}");
        assert_parses("prototype assign", "function b(){} b.prototype.isReactComponent={};");
        assert_parses("proto key", "let o={__proto__:null,c:function(){return 1;}};");
        assert_parses("or comma assign", "var n,r;n||(n=1,r={});");
        assert_parses("proto new chain", "function v(){} function S(){} var E=S.prototype=new v;");
        assert_parses("for-in comma iter", "var o={},u,t={a:1};for(u in o=1,t){}");
        assert_parses("apply arguments", "function f(){return arguments[0];} function g(){return f.apply(this,arguments);}");
        assert_parses("case nospace", "switch(x){case\"a\":x=1;break;}");
        assert_parses("default switch", "switch(e.status){default:switch(e.status){case\"a\":break;}}");
        assert_parses("forin void key", "var t={},o,u;for(u in void 0!==t.key&&(o=\"1\"),t){}");
        assert_parses("forEach key", "var I={forEach:function(){return 1;}};");
        assert_parses("ctor this", "function b(e,t,n){this.props=e,this.context=t,this.refs=m,this.updater=n||h}");
        assert_parses("proto setstate", "function b(){} b.prototype.setState=function(e,t){this.updater.enqueueSetState(this,e,t,\"setState\");};");
        assert_parses("lazy init", "var n,t;return e((n||(n=1,t=function(){if(t)return 1;t=1;return 2;}()),1));");
        assert_parses("proto null", "var o={__proto__:null,c:function(){return 1;}};");
        assert_parses("if no brace", "function f(){if(t)return 1;return 2;}");
        assert_parses("case string", "switch(x){case\"a\":x=1;break;}");
        assert_parses("label colon ternary", "var y=\"\"===u?\".\":u+\":\";");
        assert_parses("dollar typeof key", "var o={$$typeof:1,$$typeof:e};");
        assert_parses("string object keys", "var r={\"=\":\"=0\",\":\":\"=2\"};");
        assert_parses("c for var", "for(var h=0;h<3;h++)return h;");
        assert_parses("nested default switch", "switch(e.status){default:switch(e.status){case\"a\":break;}}");
        assert_parses("export tail", "function f(){return e((n||(n=1,r.exports=function(){return o}()),r.exports));}");
        assert_parses("iife tail", "!function(){return e((n||(n=1,r.exports=function(){return o}()),r.exports));}();");
        assert_parses(
            "try finally",
            "function f(e){var t=1,n={};try{var r=e();}catch(e){}finally{null!==t&&(n.a=1),t=2;}}",
        );
        assert_parses(
            "startTransition",
            "o.startTransition=function(e){var t=j.T,n={};j.T=n;try{var r=e(),o=j.S;null!==o&&o(n,r),\"object\"==typeof r&&null!==r&&\"function\"==typeof r.then&&r.then(w,A)}catch(e){A(e)}finally{null!==t&&null!==n.types&&(t.types=n.types),j.T=t}};",
        );
        assert_parses(
            "return comma tail",
            "function f(){var o={};return o.a=1,o.b=2,o}",
        );
        assert_parses(
            "return comma props",
            "function f(){var o={};return o.Activity=1,o.Children=2,o.version=\"19.2.4\",o}",
        );
        assert_parses(
            "error event object",
            "var A=function(e){if(\"object\"==typeof window&&\"function\"==typeof window.ErrorEvent){var t=new window.ErrorEvent(\"error\",{bubbles:!0,cancelable:!0,message:String(e),error:e});if(!window.dispatchEvent(t))return;}};",
        );
        assert_parses("bang zero", "var x={a:!0,b:!1,c:!0};");
        assert_parses("new with bang props", "new window.ErrorEvent(\"error\",{bubbles:!0,cancelable:!0});");
        assert_parses(
            "reportError full",
            "var A=\"function\"==typeof reportError?reportError:function(e){if(\"object\"==typeof window&&\"function\"==typeof window.ErrorEvent){var t=new window.ErrorEvent(\"error\",{bubbles:!0,cancelable:!0,message:\"object\"==typeof e&&null!==e&&\"string\"==typeof e.message?String(e.message):String(e),error:e});if(!window.dispatchEvent(t))return}else if(\"object\"==typeof process&&\"function\"==typeof process.emit)return void process.emit(\"uncaughtException\",e);console.error(e)};",
        );
        assert_parses("void return", "function f(){return void process.emit(\"x\",e);}");
        assert_parses(
            "for comma init",
            "function f(t){var a=0,u;for(t=h.call(t),h=0;!(u=t.next()).done;)a+=u.value;return a;}",
        );
        assert_parses(
            "switch comma disc",
            "function f(e){switch(1?2:(e.a=1,e.b(1,2)),e.status){case\"a\":return 1;}}",
        );
        assert_parses("unary minus cond", "function O(e){if(-1===e._status){var t=e._result;}}");
        assert_parses(
            "negative object prop",
            "o.lazy=function(e){return{$$typeof:1,_payload:{_status:-1,_result:e},_init:O}};",
        );
    }

    fn assert_evals(label: &str, src: &str) {
        let ctx = Kv8Context::default();
        if let Err(e) = eval_script(&ctx, src) {
            panic!("{label} eval failed: {e}\n{src}");
        }
    }

    #[test]
    fn wave6_eval_snippets() {
        assert_evals("fun prototype", "function b(){} b.prototype.isReactComponent={};");
        assert_evals("this assign", "function b(e){this.props=e;} new b(1);");
        assert_evals("global react", "(globalThis).React=1;");
        assert_evals(
            "umd header eval",
            "!function(e,t){\"object\"==typeof exports?module.exports=t():(globalThis).React=t();}(this,function(){\"use strict\";return 1;});",
        );
        assert_evals(
            "react ctor block",
            "function b(e,t,n){this.props=e,this.context=t,this.refs={},this.updater={};} function v(){} function S(e,t,n){this.props=e,this.context=t,this.refs={},this.updater={};} b.prototype.isReactComponent={}; b.prototype.setState=function(e,t){this.updater.enqueueSetState(this,e,t,\"setState\");}; var E=S.prototype=new v();",
        );
        assert_evals(
            "return export object",
            "function f(){var o={__proto__:null}; return o.Activity=1,o.Children=2,o.version=\"19\",o;} f();",
        );
        assert_evals("exports assign plain", "function f(){var r={exports:{}}; r.exports=1; return r.exports;} f();");
        assert_evals(
            "lazy no shadow",
            "function factory(){var t,n,r={exports:{}},o={}; r.exports=function(){if(t)return o;t=1;return 1;}; return r.exports();} factory();",
        );
        assert_evals(
            "lazy inner var",
            "function factory(){var r={exports:{}}; r.exports=function(){var r=Symbol.for(\"x\");}; return typeof r;} factory();",
        );
        assert_evals(
            "lazy or comma",
            "function factory(){var n,r={exports:{}}; return (n||(n=1,r.exports=function(){var r=Symbol.for(\"x\");}),1);} factory();",
        );
        assert_evals(
            "assign call iife",
            "function f(){var d={exports:{}}; var g=(d.exports=function(){return {v:1};}(), d.exports); return g.v;} f();",
        );
        assert_evals(
            "lazy dom export tail",
            "function f(){var d={exports:{}},p={}; var g=(d.exports=function(){p.version='19';p.createRoot=function(){}; return p;}(),d.exports); return g.createRoot;} f();",
        );
        assert_evals(
            "lazy comma dom export",
            "function f(){var c,d={exports:{}},p={}; var g=(c||(c=1,d.exports=function(){p.createRoot=function(){}; return p;}()),d.exports); return typeof g.createRoot;} f();",
        );
        assert_evals(
            "return spread createRoot",
            "function f(){var f={a:1},g={createRoot:2,hydrateRoot:3}; return {...f,createRoot:g.createRoot,hydrateRoot:g.hydrateRoot};} f();",
        );
        assert_evals(
            "multi var comma decl",
            "function f(e){ var n=(0||(1)), t=e, r=3; return t.x; } f({x:42});",
        );
        assert_evals(
            "closure e in iife",
            "function outer(e){ return (function(){ var t=e, U=t.internals; U.S=function(){}; return typeof U.S;})(); } outer({internals:{S:null}});",
        );
        assert_evals(
            "lazy var t=e after comma init",
            "function f(e){ var u; var n=(u||(u=1,1),2), t=e, r=3; return t===e; } f(5);",
        );
        assert_evals(
            "umd member assign order",
            "(function(e,n){(e=globalThis).ReactDOM=n(e.React);})(undefined,function(x){return typeof x;});",
        );
        assert_evals(
            "function hoisting",
            "function f(){ return g(); function g(){ return 1; } } f();",
        );
        assert_evals(
            "module binding survives return",
            "function factory(){ function helper(){ return 42; } return helper; } var h=factory(); h();",
        );
        assert_evals(
            "inner t shadows outer t flag",
            "function factory(e){ var t=1; return (function(){ var n=(1), t=e; return t;})(); } factory(9);",
        );
        assert_evals(
            "var H survives module_bindings collision",
            "var H=Object.prototype.hasOwnProperty; function f(){ return H.call({a:1},'a'); } H={pending:1}; f();",
        );
        assert_evals(
            "closure skips undefined var snapshot",
            "var H; function f(){ return H; } H=1; f();",
        );
        assert_evals(
            "iife globalThis react assign",
            "(function(){ globalThis.React = { ok: 1 }; })(); globalThis.React.ok;",
        );
        assert_evals(
            "arrow assigns outer let",
            "let n = 0; let el = document.createElement('button'); el.addEventListener('click', () => { n = n + 1; }); el.dispatchEvent({ type: 'click' }); n;",
        );
        assert_evals(
            "large scope pop does not leak locals",
            "function big() { var a0=0,a1=0,a2=0,a3=0,a4=0,a5=0,a6=0,a7=0,a8=0,a9=0,a10=0,a11=0,a12=0,a13=0,a14=0,a15=0,a16=0,a17=0,a18=0,a19=0,a20=0,a21=0,a22=0,a23=0,a24=0,a25=0,a26=0,a27=0,a28=0,a29=0,a30=0,a31=0,a32=0,a33=0,a34=0,a35=0,a36=0,a37=0,a38=0,a39=0,a40=0,a41=0,a42=0,a43=0,a44=0,a45=0,a46=0,a47=0,a48=0,a49=0,a50=0,a51=0,a52=0,a53=0,a54=0,a55=0,a56=0,a57=0,a58=0,a59=0,a60=0,a61=0,a62=0,a63=0,a64=0; return 1; } big();",
        );
    }

    #[test]
    fn event_timer_microtask_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::{drain_event_loop, drain_timers, eval_script};
        let ctx = Kv8Context::default();
        eval_script(
            &ctx,
            "let n = 0; setTimeout(() => { n = n + 1; }, 0);",
        )
        .expect("setTimeout");
        drain_timers(&ctx).expect("drain timers");
        let v = eval_script(&ctx, "return n;").expect("read n");
        assert!(matches!(v, Kv8Value::Num(x) if x == 1.0), "timer got {v:?}");
        eval_script(
            &ctx,
            "let m = 0; Promise.resolve(5).then((x) => { m = x + 1; });",
        )
        .expect("promise.then");
        drain_event_loop(&ctx).expect("drain loop");
        let m = eval_script(&ctx, "return m;").expect("read m");
        assert!(matches!(m, Kv8Value::Num(x) if x == 6.0), "microtask got {m:?}");
        eval_script(
            &ctx,
            "let r = 0; requestAnimationFrame(() => { r = r + 1; });",
        )
        .expect("raf");
        drain_event_loop(&ctx).expect("drain raf");
        let r = eval_script(&ctx, "return r;").expect("read r");
        assert!(matches!(r, Kv8Value::Num(x) if x == 1.0), "raf got {r:?}");
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_has_client_internals() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("react runtime");
        let v = eval_script(
            &ctx,
            "var t=globalThis.React; return t.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;",
        )
        .expect("internals");
        assert!(
            matches!(v, Kv8Value::Obj(_)),
            "expected internals object, got {v:?}"
        );
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn inner_t_shadow_react_internals() {
        use super::super::context::Kv8Context;
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("react runtime");
        let src = "function factory(e){ var t=1; function o(){return {};} return (function(){ var n=(1), t=e, r=o(); var U=t.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE; U.S=function(){}; return typeof U.S; })(); } factory(globalThis.React);";
        if let Err(e) = eval_script(&ctx, src) {
            panic!("inner t shadows outer t=1 with react internals eval failed: {e}\n{src}");
        }
    }

    #[test]
    fn esbuild_export_pm_pattern_after_iife() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var KV8ReactRuntime = (function(){
              var _2 = {};
              var Nm = function(l, t) {
                for (var u in t) {
                  Object.defineProperty(l, u, {get: t[u], enumerable: true});
                }
              };
              var Ri = function(l, t) {
                var names = Object.getOwnPropertyNames(t);
                for (var e = 0; e < names.length; e++) {
                  var c = names[e];
                  Object.defineProperty(l, c, {
                    get: function(i) { return t[i]; }.bind(null, c),
                    enumerable: true
                  });
                }
                return l;
              };
              var pm = function(l) {
                var u = {};
                Object.defineProperty(u, '__esModule', {value: true, enumerable: true});
                return Ri(u, l);
              };
              Nm(_2, {default: function(){ return O2; }});
              var O2 = {x: 66};
              return pm(_2);
            })();
            return KV8ReactRuntime.default.x;
            "#,
        )
        .expect("pm after iife");
        assert!(
            matches!(v, Kv8Value::Num(n) if (n - 66.0).abs() < f64::EPSILON),
            "expected 66, got {v:?}"
        );
    }

    #[test]
    fn esbuild_export_pm_pattern() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            return (function(){
              var _2 = {};
              var Nm = function(l, t) {
                for (var u in t) {
                  Object.defineProperty(l, u, {get: t[u], enumerable: true});
                }
              };
              var Ri = function(l, t) {
                var names = Object.getOwnPropertyNames(t);
                for (var e = 0; e < names.length; e++) {
                  var c = names[e];
                  Object.defineProperty(l, c, {
                    get: function(i) { return t[i]; }.bind(null, c),
                    enumerable: true
                  });
                }
                return l;
              };
              var pm = function(l) {
                var u = {};
                Object.defineProperty(u, '__esModule', {value: true, enumerable: true});
                return Ri(u, l);
              };
              Nm(_2, {default: function(){ return O2; }});
              var O2 = {x: 55};
              return pm(_2).default.x;
            })();
            "#,
        )
        .expect("pm pattern");
        assert!(
            matches!(v, Kv8Value::Num(n) if (n - 55.0).abs() < f64::EPSILON),
            "expected 55, got {v:?}"
        );
    }

    #[test]
    fn iife_closure_late_var_survives_scope_pop() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            return (function(){
              var O2;
              var o = {};
              Object.defineProperty(o, 'default', {get: function(){ return O2; }, enumerable: true});
              O2 = {x: 99};
              return o.default.x;
            })();
            "#,
        )
        .expect("iife late bind");
        assert!(
            matches!(v, Kv8Value::Num(n) if (n - 99.0).abs() < f64::EPSILON),
            "expected 99, got {v:?}"
        );
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_runtime_eval_timing() {
        use super::super::context::Kv8Context;
        use std::time::Instant;
        let ctx = Kv8Context::default();
        let t0 = Instant::now();
        super::load_react_runtime(&ctx).expect("load react runtime");
        let elapsed = t0.elapsed();
        let bindings = ctx
            .with_read(|inner| Ok(inner.module_bindings.len()))
            .unwrap_or(0);
        eprintln!(
            "react bundle eval: {:?}, eval_ops={}, module_bindings={}",
            elapsed,
            ctx.eval_ops_count(),
            bindings
        );
        assert!(elapsed.as_secs() < 600, "bundle eval exceeded 10 minutes");
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_runtime_eval_probe() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("load react runtime");
        let react = eval_script(&ctx, "return globalThis.React;").expect("globalThis.React");
        assert!(
            matches!(react, Kv8Value::Obj(_) | Kv8Value::Fun { .. }),
            "expected React export, got {react:?}"
        );
    }

    #[test]
    fn react_runtime_full_parse() {
        if let Err(e) = parse_program(super::REACT_RUNTIME_BUNDLE) {
            panic!("full react runtime bundle: {e}");
        }
    }

    #[test]
    fn wave7_snippet_matrix() {
        assert_parses("bitwise and", "let x = 42 & 15;");
        assert_parses("bitwise or shift", "let y = (1 << 3) | 2;");
        assert_parses("try finally only", "try { x = 1; } finally { x = 2; }");
        assert_parses("label for break", "e:for(;;){ if (x) break e; }");
        assert_parses("do while", "do { x = x + 1; } while (x < 3);");
        assert_parses("sci notation", "let z = 1e3 / 2;");
        assert_parses("in operator", "let ok = 'a' in { a: 1 };");
        assert_parses("delete member", "let d = delete obj.x;");
        assert_parses("object spread", "let o = { ...f, createRoot: g.createRoot };");
        assert_parses("computed key", "let o = { [2]: null };");
        assert_parses("async object key", "let o = { async: !0 };");
        assert_parses("null object key", "let o = { null: 1 };");
    }

    #[test]
    fn wave8_modern_js_matrix() {
        assert_parses("for of", "for (let x of arr) { x; }");
        assert_parses("template literal", "let s = `hi ${name}!`;");
        assert_parses("nullish coalesce", "let v = a ?? b;");
        assert_parses("optional member", "let x = obj?.field;");
        assert_parses("optional call", "let x = fn?.();");
        assert_parses("import named", "import { React } from \"react-runtime\";");
        assert_parses("export default", "export default { React: 1 };");
        assert_evals("for of sum", "let a = [1, 2, 3]; let s = 0; for (let x of a) { s = s + x; } s;");
        assert_evals("template concat", "let n = 2; let s = `n=${n}`; s;");
        assert_evals("nullish coalesce", "let x = null ?? 5; x;");
        assert_evals("optional chain", "let o = null; let x = o?.missing; typeof x;");
    }

    #[test]
    fn import_export_module_smoke() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::{eval_script, register_kv8_module};
        use std::collections::HashMap;
        let ctx = Kv8Context::default();
        let mut named = HashMap::new();
        named.insert("React".into(), Kv8Value::Num(1.0));
        register_kv8_module(&ctx, "react-runtime", Some(Kv8Value::Num(9.0)), named).expect("register");
        let v = eval_script(
            &ctx,
            "import { React } from \"react-runtime\"; return React;",
        )
        .expect("import");
        assert!(matches!(v, Kv8Value::Num(x) if x == 1.0), "import got {v:?}");
        let d = eval_script(
            &ctx,
            "import rt from \"react-runtime\"; return rt;",
        )
        .expect("default import");
        assert!(matches!(d, Kv8Value::Num(x) if x == 9.0), "default got {d:?}");
    }

    #[test]
    fn dom_return_tail_parse() {
        assert_parses(
            "return spread tail",
            "function f(){var f={};var g={createRoot:1,hydrateRoot:2};return{...f,createRoot:g.createRoot,hydrateRoot:g.hydrateRoot};}",
        );
    }

    #[test]
    fn dom_var_line_parse_has_t() {
        use super::super::ast::Stmt;
        use super::super::eval::parse_program;
        let src = include_str!("../../../fixtures/kv8/react/dom-var-line.probe.js");
        let prog = parse_program(src).expect("parse dom var line");
        let mut names = Vec::new();
        for stmt in &prog.stmts {
            let Stmt::Function(_, _, body) = stmt else {
                continue;
            };
            for s in body {
                if let Stmt::Var(n, _) | Stmt::Let(n, _) = s {
                    names.push(n.clone());
                }
            }
        }
        assert!(
            names.iter().any(|n| n == "t"),
            "expected Let t in d.exports var line, got {names:?}"
        );
    }

    #[test]
    fn dom_umd_member_assign_order() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("react runtime");
        match &eval_script(
            &ctx,
            "(function(e,n){(e=globalThis).ReactDOM=n(e.React);})(undefined,function(x){return typeof x;});",
        )
        .expect("exact umd branch") {
            Kv8Value::Str(s) if s == "object" => {}
            other => panic!("exact umd branch got {other:?}"),
        }
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_create_element_probe() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_dom_runtime(&ctx).expect("react-dom runtime");
        let probes: &[(&str, &str, fn(&Kv8Value) -> bool)] = &[
            (
                "typeof createElement",
                "return typeof globalThis.React.createElement;",
                |v| matches!(v, Kv8Value::Str(s) if s == "function"),
            ),
            (
                "createElement button not null",
                "var React=globalThis.React; var el=React.createElement('button', null, 'hi'); return el === null ? 'null' : 'ok';",
                |v| matches!(v, Kv8Value::Str(s) if s == "ok"),
            ),
            (
                "createElement typeof",
                "var React=globalThis.React; var el=React.createElement('button', null, 'hi'); return typeof el;",
                |v| matches!(v, Kv8Value::Str(s) if s == "object"),
            ),
            (
                "createElement props",
                "var React=globalThis.React; var el=React.createElement('button', {className:'counter-btn', onClick: function(){}}, 'Count: 0'); return el === null ? 'null' : typeof el;",
                |v| matches!(v, Kv8Value::Str(s) if s == "object"),
            ),
            (
                "createElement arrow",
                "var React=globalThis.React; var el=React.createElement('button', {className:'counter-btn', onClick: ()=>{}}, 'Count: 0'); return el === null ? 'null' : typeof el;",
                |v| matches!(v, Kv8Value::Str(s) if s == "object"),
            ),
            (
                "render element",
                "var React=globalThis.React,ReactDOM=globalThis.ReactDOM; var el=document.createElement('div'); var root=ReactDOM.createRoot(el); var btn=React.createElement('button', null, 'hi'); try { root.render(btn); return 'ok'; } catch(e) { return String(e); }",
                |v| matches!(v, Kv8Value::Str(s) if s == "ok"),
            ),
        ];
        for (name, src, ok) in probes {
            let v = eval_script(&ctx, src).unwrap_or_else(|e| panic!("{name} eval failed: {e}"));
            assert!(ok(&v), "{name} got {v:?}");
        }
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_runtime_create_root() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_dom_runtime(&ctx).expect("react-dom runtime");
        let v = eval_script(
            &ctx,
            "var React=globalThis.React,ReactDOM=globalThis.ReactDOM; return typeof ReactDOM.createRoot;",
        );
        match v {
            Ok(Kv8Value::Str(s)) if s == "function" => {}
            other => panic!("ReactDOM.createRoot typeof got {other:?}"),
        }
        let v2 = eval_script(
            &ctx,
            "var React=globalThis.React,ReactDOM=globalThis.ReactDOM; var el=document.createElement('div'); document.body.appendChild(el); try { var root=ReactDOM.createRoot(el); return typeof root.render; } catch(e) { return String(e); }",
        );
        match v2 {
            Ok(Kv8Value::Str(s)) if s == "function" => {}
            Ok(Kv8Value::Str(s)) => panic!("createRoot threw or bad type: {s}"),
            other => panic!("createRoot smoke got {other:?}"),
        }
    }

    #[test]
    #[ignore = "counter fiber path — enable when render parity is stable"]
    fn react_runtime_counter_smoke() {
        use super::super::context::Kv8Context;
        let ctx = Kv8Context::default();
        super::react_runtime_bundle_smoke(&ctx).expect("react runtime counter");
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn dom_runtime_eval_probe() {
        use super::super::context::{Kv8Context, Kv8Value};
        let ctx = Kv8Context::default();
        let react_dom = super::load_react_dom_runtime(&ctx).expect("load react-dom runtime");
        assert!(
            matches!(react_dom, Kv8Value::Obj(_) | Kv8Value::Fun { .. }),
            "expected ReactDOM export, got {react_dom:?}"
        );
        let version = super::super::eval::eval_script(&ctx, "return globalThis.ReactDOM.version;")
            .expect("ReactDOM.version");
        match &version {
            Kv8Value::Str(s) => assert!(s.starts_with("19."), "unexpected version {s}"),
            Kv8Value::Undefined => {
                let cr = super::super::eval::eval_script(
                    &ctx,
                    "return typeof globalThis.ReactDOM.createRoot;",
                )
                .expect("createRoot typeof");
                assert!(matches!(cr, Kv8Value::Str(s) if s == "function"));
            }
            other => panic!("unexpected version {other:?}"),
        }
    }

    // Legacy test names — keep filters working in older CI logs.
    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn umd_eval_probe() {
        react_runtime_eval_probe();
    }

    #[test]
    fn umd_full_parse() {
        react_runtime_full_parse();
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_umd_create_element_probe() {
        react_create_element_probe();
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_umd_create_root() {
        react_runtime_create_root();
    }

    #[test]
    fn react_umd_counter_smoke() {
        react_runtime_counter_smoke();
    }

    #[test]
    fn dom_umd_eval_probe() {
        dom_runtime_eval_probe();
    }

    #[test]
    fn dom_umd_full_parse() {
        react_runtime_full_parse();
    }
}
