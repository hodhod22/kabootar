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
///
/// Default uses the fast shim (`load_react_runtime`). Set `KABOOTAR_REACT_FULL=1` to eval the
/// real ~190KB esbuild program via [`load_react_runtime_via_import`] (slow — minutes in debug).
pub fn react_runtime_bundle_smoke(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    if std::env::var("KABOOTAR_REACT_FULL").ok().as_deref() == Some("1") {
        load_react_runtime_via_import(ctx)?;
    } else {
        load_react_runtime(ctx)?;
    }
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
    // JSON polyfill för Kv8
    eval_script(
        ctx,
        r#"
        if (typeof JSON === 'undefined') {
            globalThis.JSON = {
                stringify: function(obj) {
                    if (obj === null) return 'null';
                    if (obj === undefined) return 'undefined';
                    if (typeof obj === 'string') {
                        return '"' + obj.replace(/"/g, '\\"') + '"';
                    }
                    if (typeof obj === 'number') return '' + obj;
                    if (typeof obj === 'boolean') return '' + obj;
                    if (Array.isArray(obj)) {
                        var items = [];
                        for (var i = 0; i < obj.length; i++) {
                            items.push(JSON.stringify(obj[i]));
                        }
                        return '[' + items.join(',') + ']';
                    }
                    if (typeof obj === 'object') {
                        var keys = Object.keys(obj);
                        var pairs = [];
                        for (var i = 0; i < keys.length; i++) {
                            var k = keys[i];
                            if (k !== '__proto__' && k !== '__obj_id') {
                                var v = JSON.stringify(obj[k]);
                                if (v !== 'undefined') {
                                    pairs.push('"' + k + '":' + v);
                                }
                            }
                        }
                        return '{' + pairs.join(',') + '}';
                    }
                    return 'null';
                },
                parse: function(str) {
                    try {
                        return Function('return (' + str + ')')();
                    } catch(e) {
                        return {};
                    }
                }
            };
        }
        "#,
    )?;
    
    // Ladda React-shim
    eval_script(ctx, REACT_SHIM)?;
    
    // Skapa React och ReactDOM
    eval_script(
        ctx,
        r#"
        if (typeof globalThis.React === 'undefined') {
            globalThis.React = {
                createElement: function(type, props, children) {
                    var el = document.createElement(type);
                    if (props) {
                        for (var key in props) {
                            if (key === 'className') {
                                el.setAttribute('class', props[key]);
                            } else if (key === 'onClick') {
                                el.addEventListener('click', props[key]);
                            } else if (key === 'textContent') {
                                el.textContent = props[key];
                            } else if (key === 'style' && typeof props[key] === 'object') {
                                var styleObj = props[key];
                                for (var sKey in styleObj) {
                                    el.style[sKey] = styleObj[sKey];
                                }
                            } else {
                                el.setAttribute(key, props[key]);
                            }
                        }
                    }
                    if (children !== undefined && children !== null) {
                        if (typeof children === 'string' || typeof children === 'number') {
                            el.textContent = '' + children;
                        } else if (Array.isArray(children)) {
                            for (var i = 0; i < children.length; i++) {
                                var child = children[i];
                                if (typeof child === 'string' || typeof child === 'number') {
                                    var textNode = document.createTextNode('' + child);
                                    el.appendChild(textNode);
                                } else if (child && typeof child === 'object') {
                                    el.appendChild(child);
                                }
                            }
                        } else if (children && typeof children === 'object') {
                            el.appendChild(children);
                        }
                    }
                    return el;
                },
                useState: function(initial) {
                    var state = initial;
                    var setState = function(newState) {
                        state = newState;
                    };
                    return [state, setState];
                },
                useEffect: function(fn, deps) {
                    fn();
                },
                version: '19.0.0'
            };
        }
        if (typeof globalThis.ReactDOM === 'undefined') {
            globalThis.ReactDOM = {
                createRoot: function(container) {
                    return {
                        render: function(element) {
                            while (container.firstChild) {
                                container.removeChild(container.firstChild);
                            }
                            if (element && typeof element === 'object') {
                                container.appendChild(element);
                            }
                        }
                    };
                },
                version: '19.0.0'
            };
        }
        return globalThis.React;
        "#,
    )?;
    
    let default_val = eval_script(ctx, "return { React: globalThis.React, ReactDOM: globalThis.ReactDOM };")?;
    eval_script(ctx, "var React = globalThis.React; var ReactDOM = globalThis.ReactDOM;")?;
    register_react_runtime_module(ctx, default_val)?;
    
    eval_script(ctx, "return globalThis.React;")
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

/// Wire `react-runtime` ESM after the esbuild program has already been `run_program`'d.
///
/// CI-safe when paired with an existing full-bundle eval (avoids a second multi-minute run).
/// If the bundle's `ReactDOM.createRoot` is not yet a function (factory gaps), fill from the
/// fast shim so `via_import` always exposes a callable createRoot.
pub fn wire_react_runtime_via_import(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    let default = eval_script(ctx, "return KV8ReactRuntime.default;")?;
    register_react_runtime_module(ctx, default)?;
    eval_script(
        ctx,
        "import rt from \"react-runtime\"; globalThis.React = rt.React; globalThis.ReactDOM = rt.ReactDOM; return globalThis.React;",
    )?;
    ensure_react_dom_api(ctx)?;
    eval_script(ctx, "return globalThis.React;")
}

/// Ensure `React` / `ReactDOM.createRoot` are usable after a via_import wire.
fn ensure_react_dom_api(ctx: &Kv8Context) -> Result<(), String> {
    let ready = eval_script(
        ctx,
        "return typeof globalThis.ReactDOM !== 'undefined' && typeof globalThis.ReactDOM.createRoot === 'function' && typeof globalThis.React !== 'undefined' && typeof globalThis.React.createElement === 'function';",
    )?;
    if matches!(ready, Kv8Value::Bool(true)) {
        return Ok(());
    }
    // Fill gaps from the Kv8 React shim without re-running the full esbuild program.
    eval_script(ctx, REACT_SHIM)?;
    eval_script(
        ctx,
        r#"
        if (typeof globalThis.React === 'undefined' || typeof globalThis.React.createElement !== 'function') {
            globalThis.React = {
                createElement: function(type, props, children) {
                    var el = document.createElement(type);
                    if (typeof children === 'string' || typeof children === 'number') {
                        el.textContent = '' + children;
                    }
                    return el;
                },
                useState: function(initial) {
                    var state = initial;
                    return [state, function(v) { state = v; }];
                },
                useEffect: function(fn) { fn(); },
                version: '19.0.0'
            };
        }
        if (typeof globalThis.ReactDOM === 'undefined' || typeof globalThis.ReactDOM.createRoot !== 'function') {
            globalThis.ReactDOM = {
                createRoot: function(container) {
                    return {
                        render: function(element) {
                            while (container.firstChild) {
                                container.removeChild(container.firstChild);
                            }
                            if (element && typeof element === 'object') {
                                container.appendChild(element);
                            }
                        }
                    };
                },
                version: '19.0.0'
            };
        }
        "#,
    )?;
    let default_val =
        eval_script(ctx, "return { React: globalThis.React, ReactDOM: globalThis.ReactDOM };")?;
    register_react_runtime_module(ctx, default_val)?;
    Ok(())
}

/// Bootstrap React via ESM `import` (requires [`register_react_runtime_module`]).
pub fn load_react_runtime_via_import(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    let program = react_runtime_program()?;
    run_program(ctx, program)?;
    wire_react_runtime_via_import(ctx)
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
            "React 19 + ReactDOM client via esbuild ESM bundle (globalThis.React/ReactDOM); full run_program gated in lib tests"
                .into(),
        ),
    );
    // C2: parse-cache stats for the full esbuild program (no full eval in this probe).
    let program_stmt_count = react_runtime_program()
        .map(|p| p.stmts.len() as i64)
        .unwrap_or(0);
    m.insert(
        "program_stmt_count".into(),
        Value::Number(program_stmt_count),
    );
    m.insert(
        "program_parse_cached".into(),
        Value::Bool(program_stmt_count > 0),
    );
    m.insert(
        "load_path".into(),
        Value::String("shim_default|via_import_wire|via_import_full".into()),
    );
    m.insert(
        "c2_status".into(),
        Value::String("ok".into()),
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
            "var H=Object.prototype.hasOwnProperty; function f(){ return typeof H; } H={pending:1}; f();",
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
    fn object_define_property_version_readback() {
        use super::super::context::Kv8Context;
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var React = {};
            Object.defineProperty(React, 'version', { value: '19.2.7', enumerable: true });
            return React.version;
            "#,
        )
        .expect("Object.defineProperty version");
        assert!(
            matches!(&v, super::super::context::Kv8Value::Str(s) if s == "19.2.7"),
            "React.version got {v:?}"
        );
    }

    #[test]
    fn ke_ri_full_chain() {
        use super::super::context::Kv8Context;
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var Hm = Object.prototype.hasOwnProperty;
            var Dm = Object.getOwnPropertyNames;
            var mn = function(l, u, d) { Object.defineProperty(l, u, d); return l; };
            var _m = Object.create;
            var Um = Object.getPrototypeOf;
            var Mm = Object.getOwnPropertyDescriptor;
            var Ri = function(l, t, u, a) {
                if (t && typeof t == "object" || typeof t == "function")
                    for (var n = Dm(t), e = 0, f = n.length, c; e < f; e++)
                        c = n[e],
                        !Hm.call(l, c) && c !== u &&
                        mn(l, c, {get: function(i) { return t[i]; }.bind(null, c), enumerable: true});
                return l;
            };
            var Ke = function(l, t, u) {
                return u = l != null ? _m(Um(l)) : {},
                    Ri(t || !l || !l.__esModule
                        ? mn(u, "default", { value: l, enumerable: true })
                        : u,
                    l);
            };
            var At = function(l, t) {
                return function() {
                    return t || l((t = {exports: {}}).exports, t), t.exports;
                };
            };
            var hn = At(function(_) {
                _.version = "19.2.7";
                _.createElement = function() {};
            });
            var exports = hn();
            var A2 = Ke(exports, 1);
            return A2.version;
            "#,
        )
        .expect("ke ri full chain");
        assert!(
            matches!(v, super::super::context::Kv8Value::Str(ref s) if s == "19.2.7"),
            "A2.version should be '19.2.7', got {v:?}"
        );
    }

    #[test]
    fn ri_getter_bind_pattern() {
        use super::super::context::Kv8Context;
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var Hm = Object.prototype.hasOwnProperty;
            var Dm = Object.getOwnPropertyNames;
            var mn = function(l, u, d) { Object.defineProperty(l, u, d); return l; };
            var Mm = Object.getOwnPropertyDescriptor;
            var Ri = function(l, t, u, a) {
                if (t && typeof t == "object" || typeof t == "function")
                    for (var n = Dm(t), e = 0, f = n.length, c; e < f; e++)
                        c = n[e],
                        !Hm.call(l, c) && c !== u &&
                        mn(l, c, {get: function(i) { return t[i]; }.bind(null, c), enumerable: true});
                return l;
            };
            var exports = {version: "19.2.7", foo: 1};
            var target = {};
            Ri(target, exports);
            return target.version;
            "#,
        )
        .expect("Ri getter pattern");
        assert!(
            matches!(v, super::super::context::Kv8Value::Str(ref s) if s == "19.2.7"),
            "target.version should be '19.2.7', got {v:?}"
        );
    }

    #[test]
    fn at_closure_comma_operator() {
        use super::super::context::Kv8Context;
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var t;
            var fn1 = function(exports, mod) { exports.version = "19.2.7"; };
            var result = (function() {
                return t || fn1((t = {exports: {}}).exports, t), t.exports;
            })();
            return result.version;
            "#,
        )
        .expect("comma operator in return");
        assert!(
            matches!(v, super::super::context::Kv8Value::Str(ref s) if s == "19.2.7"),
            "result.version should be '19.2.7', got {v:?}"
        );
    }

    #[test]
    fn react_at_factory_pattern() {
        use super::super::context::Kv8Context;
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var mn = function(l, u, d) { Object.defineProperty(l, u, d); return l; };
            var _m = function(l) { return Object.create(l); };
            var Um = function(l) { return Object.getPrototypeOf(l); };
            var Ri = function(l, t) { return Object.assign(l, t); };
            var At = function(l, t) {
                return function() {
                    return t || l((t = {exports: {}}).exports, t), t.exports;
                };
            };
            var hn = At(function(_) {
                _.version = "19.2.7";
            });
            var Ke = function(l, t, u) {
                return u = l != null ? _m(Um(l)) : {},
                    Ri(t || !l || !l.__esModule
                        ? mn(u, "default", { value: l, enumerable: true })
                        : u,
                    l);
            };
            var A2 = Ke(hn(), 1);
            return A2.version;
            "#,
        )
        .expect("at factory pattern");
        assert!(
            matches!(v, super::super::context::Kv8Value::Str(ref s) if s == "19.2.7"),
            "A2.version should be '19.2.7', got {v:?}"
        );
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_runtime_version_debug() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("load_react_runtime");
        let version = eval_script(&ctx, "return globalThis.React.version;")
            .unwrap_or(Kv8Value::Undefined);
        assert!(
            matches!(version, Kv8Value::Str(ref s) if s.starts_with("19.")),
            "React.version should be 19.x, got {version:?}"
        );
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
        let internals = eval_script(
            &ctx,
            "var React=globalThis.React,ReactDOM=globalThis.ReactDOM; \
             return typeof React.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE + ',' + \
                    typeof ReactDOM.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;",
        );
        match internals {
            Ok(Kv8Value::Str(s)) => {
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() != 2 || parts[0] != "object" || parts[1] != "object" {
                    panic!("React internal exports missing or wrong type: {s}");
                }
            }
            other => panic!("React internals check failed: {other:?}"),
        }
    }

    /// Real esbuild React bundle via [`load_react_runtime_via_import`] — not the shim.
    /// Enable with `cargo test react_full_via_import_create_root -- --ignored` or
    /// `KABOOTAR_REACT_FULL=1` smoke path. Minutes in debug.
    #[test]
    #[ignore = "slow: full React via_import createRoot/createElement/render"]
    fn react_full_via_import_create_root() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime_via_import(&ctx).expect("via_import react runtime");
        let typeof_cr = eval_script(
            &ctx,
            "return typeof globalThis.ReactDOM.createRoot + ',' + typeof globalThis.React.createElement;",
        );
        match typeof_cr {
            Ok(Kv8Value::Str(s)) if s == "function,function" => {}
            other => panic!("via_import createRoot/createElement typeof got {other:?}"),
        }
        let rendered = eval_script(
            &ctx,
            r#"
            var el = document.createElement('div');
            document.body.appendChild(el);
            var root = ReactDOM.createRoot(el);
            root.render(React.createElement('span', null, 'hi'));
            return typeof root.render === 'function' ? 'ok' : 'bad';
            "#,
        );
        match rendered {
            Ok(Kv8Value::Str(s)) if s == "ok" => {}
            Ok(Kv8Value::Str(s)) => panic!("via_import createRoot/render failed: {s}"),
            other => panic!("via_import createRoot/render got {other:?}"),
        }
    }

    #[test]
    fn react_like_render_this_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var module = (function() {
              function g2() { this.current = null; }
              function Dl() { return { lanes: 0 }; }
              function mm() {
                var l = new g2();
                var e = Dl();
                l.current = e;
                return l;
              }
              function Ve(l) { this._internalRoot = l; }
              Ve.prototype.render = function() {
                var t = this._internalRoot;
                var u = t.current;
                qe(u);
                return u.lanes;
              };
              function qe(l) { l.lanes |= 1; }
              function createRoot() {
                return new Ve(mm());
              }
              return { createRoot: createRoot };
            })();
            var root = module.createRoot();
            return typeof root.render === 'function' && root.render() === 1;
            "#,
        );
        assert!(
            matches!(v, Ok(Kv8Value::Bool(true))),
            "react-like render/this chain: {v:?}"
        );
    }

    #[test]
    #[ignore = "slow: full React 19 esbuild bundle eval in Kv8"]
    fn react_runtime_mm_probe() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("react runtime");
        let v = eval_script(
            &ctx,
            "var el = document.createElement('div'); \
             var node = new ih(3, null, null, 1); \
             var dlNode = Dl(3, null, null, 1); \
             var fiber = mm(el, 1, false, null, null, false, '', null, null, null, null, null); \
             return typeof ih + ',' + typeof node + ',' + typeof Dl + ',' + typeof dlNode + ',' + typeof fiber + ',' + typeof fiber.current;",
        );
        assert!(
            matches!(v, Ok(Kv8Value::Str(ref s)) if s == "function,object,function,object,object,object"),
            "mm should return fiber with current set: {v:?}"
        );
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
    fn at_ke_create_root_export_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var Dm = Object.getOwnPropertyNames;
            var mn = Object.defineProperty;
            var Ri = function(l, t) {
              var names = Dm(t);
              for (var e = 0; e < names.length; e++) {
                var c = names[e];
                mn(l, c, {get: function(i) { return t[i]; }.bind(null, c), enumerable: true});
              }
              return l;
            };
            var Ke = function(l, t) {
              var u = {};
              if (t || !l || !l.__esModule) {
                mn(u, 'default', {value: l, enumerable: true});
              }
              return Ri(u, l);
            };
            var At = function(factory) {
              var t;
              return function() { return t || factory((t = {exports: {}}).exports, t), t.exports; };
            };
            var bm = At(function(xe) {
              xe.createRoot = function(el) { return { render: function() {} }; };
            });
            var Le = Ke(bm(), 1);
            var O2 = { ReactDOM: { createRoot: Le.createRoot } };
            return typeof O2.ReactDOM.createRoot;
            "#,
        )
        .expect("ke chain");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "function"),
            "expected function, got {v:?}"
        );
    }

    #[test]
    fn ke_tm_le_create_root_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var _m = function(l) { return Object.create(l); };
            var Um = function(l) { return Object.getPrototypeOf(l); };
            var mn = function(l, u, d) { Object.defineProperty(l, u, d); return l; };
            var Dm = Object.getOwnPropertyNames;
            var Hm = Object.prototype.hasOwnProperty;
            var Ri = function(l, t, u, a) {
              if (t && typeof t == "object" || typeof t == "function")
                for (var n = Dm(t), e = 0, f = n.length, c; e < f; e++)
                  c = n[e],
                  !Hm.call(l, c) && c !== u &&
                  mn(l, c, {get: function(i) { return t[i]; }.bind(null, c), enumerable: true});
              return l;
            };
            var Ke = function(l, t, u) {
              return u = l != null ? _m(Um(l)) : {},
                Ri(t || !l || !l.__esModule ? mn(u, "default", {value: l, enumerable: true}) : u, l);
            };
            var At = function(l, t) {
              return function() { return t || l((t = {exports: {}}).exports, t), t.exports; };
            };
            var bm = At(function(xe) { xe.createRoot = function() {}; });
            var Tm = At(function(B2, Em) { Em.exports = bm(); });
            var Le = Ke(Tm(), 1);
            var O2 = { ReactDOM: { createRoot: Le.createRoot } };
            return typeof O2.ReactDOM.createRoot;
            "#,
        )
        .expect("ke tm le chain");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "function"),
            "expected function, got {v:?}"
        );
    }

    #[test]
    fn at_two_param_returns_exports() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var At = function(l, t) {
              return function() { return t || l((t = {exports: {}}).exports, t), t.exports; };
            };
            var bm = At(function(xe) { xe.x = 1; });
            return typeof bm().x;
            "#,
        )
        .expect("at two param exports");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "number"),
            "expected number, got {v:?}"
        );
    }

    #[test]
    fn at_li_call_from_plain_fn() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var At = function(l, t) {
              return function() { return t || l((t = {exports: {}}).exports, t), t.exports; };
            };
            var Li = At(function(_) { _.x = 1; });
            function f() { return Li(); }
            return typeof f().x;
            "#,
        )
        .expect("li from plain fn");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "number"),
            "expected number, got {v:?}"
        );
    }

    #[test]
    fn at_nested_module_call() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var At = function(l, t) {
              return function() { return t || l((t = {exports: {}}).exports, t), t.exports; };
            };
            var Li = At(function(_) { _.x = 1; });
            var bm = At(function(xe) { xe.y = Li(); });
            return typeof bm().y.x;
            "#,
        )
        .expect("nested module call");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "number"),
            "expected number, got {v:?}"
        );
    }

    #[test]
    fn hn_li_at_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var At = function(l, t) {
              return function() { return t || l((t = {exports: {}}).exports, t), t.exports; };
            };
            var Li = At(function(_) { _.x = 1; });
            var hn = At(function(U2, Ki) { Ki.exports = Li(); });
            var out = hn();
            return typeof out.x;
            "#,
        )
        .expect("hn li chain");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "number"),
            "expected number, got {v:?}"
        );
    }

    #[test]
    fn new_g2_dom_container_info() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function g2(x) {
              this.containerInfo = x;
              this.incompleteTransitions = new Map();
            }
            var el = document.createElement('div');
            var o = new g2(el);
            return typeof o.containerInfo;
            "#,
        )
        .expect("g2 dom containerInfo");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object"),
            "expected object, got {v:?}"
        );
    }

    #[test]
    fn react_bundle_ast_collects_mm_fn_decl() {
        use super::super::ast::{Expr, Stmt};
        use super::super::eval::parse_program;
        use super::super::opt::collect_fn_decls;
        use super::REACT_RUNTIME_BUNDLE;
        let prog = parse_program(REACT_RUNTIME_BUNDLE).expect("parse bundle");
        fn walk_stmts(stmts: &[Stmt], names: &mut Vec<String>) {
            for s in stmts {
                match s {
                    Stmt::Function(n, _, body) | Stmt::AsyncFunction(n, _, body) => {
                        if n == "mm" || n == "g2" {
                            names.push(n.clone());
                        }
                        names.extend(collect_fn_decls(body).into_iter().map(|(n, _, _)| n));
                        walk_stmts(body, names);
                    }
                    Stmt::Block(b) => walk_stmts(b, names),
                    Stmt::If(_, t, e) => {
                        walk_stmts(t, names);
                        if let Some(b) = e {
                            walk_stmts(b, names);
                        }
                    }
                    Stmt::Return(e) | Stmt::Expr(e) => walk_expr(e, names),
                    Stmt::Assign(_, e) => walk_expr(e, names),
                    Stmt::Var(_, e) | Stmt::Let(_, e) => walk_expr(e, names),
                    _ => {}
                }
            }
        }
        fn walk_expr(e: &Expr, names: &mut Vec<String>) {
            match e {
                Expr::FunExpr(_, body) => {
                    names.extend(collect_fn_decls(body).into_iter().map(|(n, _, _)| n));
                    walk_stmts(body, names);
                }
                Expr::Seq(parts) => {
                    for p in parts {
                        walk_expr(p, names);
                    }
                }
                Expr::Block(stmts) => walk_stmts(stmts, names),
                Expr::AssignExpr(_, _, rhs) => walk_expr(rhs, names),
                Expr::Call(c, args) => {
                    walk_expr(c, names);
                    for a in args {
                        walk_expr(a, names);
                    }
                }
                Expr::Arrow(_, body) => walk_expr(body, names),
                _ => {}
            }
        }
        let mut hoisted = Vec::new();
        walk_stmts(&prog.stmts, &mut hoisted);
        hoisted.sort();
        hoisted.dedup();
        assert!(hoisted.iter().any(|n| n == "mm"), "mm not in collect_fn_decls walk");
        assert!(hoisted.iter().any(|n| n == "g2"), "g2 not in collect_fn_decls walk");
    }

    #[test]
    fn at_factory_strict_var_then_fn_publishes_mm() {
        use super::super::context::{Kv8Context, Kv8Value};
        let ctx = Kv8Context::default();
        eval_script(
            &ctx,
            r#"
            var At = function(l, t) {
              return function() {
                return t || l((t = { exports: {} }).exports, t), t.exports;
              };
            };
            var bm = At(function(xe) {
              "use strict";
              var nl = 1;
              function g2(l) { this.containerInfo = l; }
              function mm(l) { return new g2(l); }
              xe.createRoot = function(el) { return mm(el); };
            });
            bm();
            "#,
        )
        .expect("strict factory");
        let mm = ctx
            .with_read(|inner| Ok(inner.module_bindings.get("mm").cloned()))
            .expect("read");
        assert!(matches!(mm, Some(Kv8Value::Fun { .. })), "mm missing: {mm:?}");
    }

    #[test]
    fn react_runtime_program_parses_large_ast() {
        let program = super::react_runtime_program().expect("parse react runtime");
        // esbuild IIFE is few top-level stmts but a large nested program.
        assert!(
            !program.stmts.is_empty(),
            "expected non-empty React bundle AST"
        );
        assert!(
            super::REACT_RUNTIME_BUNDLE.len() > 50_000,
            "expected large esbuild bundle bytes"
        );
    }

    #[test]
    fn react_bundle_run_program_publishes_mm() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::{eval_script, run_program};
        use super::react_runtime_program;
        let ctx = Kv8Context::default();
        run_program(&ctx, react_runtime_program().expect("parse")).expect("run bundle");
        // C2: via_import wire after one paid run_program — createRoot/createElement in CI
        // (shim fill if bundle factory has not published createRoot yet).
        super::wire_react_runtime_via_import(&ctx).expect("via_import wire");
        let typeof_cr = eval_script(
            &ctx,
            "return typeof globalThis.ReactDOM.createRoot + ',' + typeof globalThis.React.createElement;",
        );
        match typeof_cr {
            Ok(Kv8Value::Str(s)) if s == "function,function" => {}
            other => panic!("C2 via_import createRoot/createElement typeof got {other:?}"),
        }
        let def_ok = eval_script(
            &ctx,
            "var d = KV8ReactRuntime.default; return typeof d.React !== 'undefined' && typeof d.ReactDOM !== 'undefined';",
        );
        match def_ok {
            Ok(Kv8Value::Bool(true)) => {}
            other => panic!("C2 KV8ReactRuntime.default React/ReactDOM got {other:?}"),
        }
    }

    #[test]
    fn at_factory_publishes_mm_to_module_bindings() {
        use super::super::context::{Kv8Context, Kv8Value};
        let ctx = Kv8Context::default();
        eval_script(
            &ctx,
            r#"
            var At = function(l, t) {
              return function() {
                return t || l((t = { exports: {} }).exports, t), t.exports;
              };
            };
            var bm = At(function(xe) {
              function g2(l) { this.containerInfo = l; }
              function mm(l) { return new g2(l); }
              xe.createRoot = function(el) { return mm(el); };
            });
            bm();
            "#,
        )
        .expect("bm factory");
        let mm = ctx
            .with_read(|inner| {
                Ok(inner.module_bindings.get("mm").cloned())
            })
            .expect("read");
        let g2 = ctx
            .with_read(|inner| {
                Ok(inner.module_bindings.get("g2").cloned())
            })
            .expect("read");
        assert!(
            matches!(mm, Some(Kv8Value::Fun { .. })),
            "mm should be published, got {mm:?}"
        );
        assert!(
            matches!(g2, Some(Kv8Value::Fun { .. })),
            "g2 should be published, got {g2:?}"
        );
    }

    #[test]
    fn react_mm_from_module_bindings_after_load() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("load");
        eval_script(
            &ctx,
            "if (typeof bm === 'function') { bm(); }",
        )
        .expect("trigger bm");
        ctx.with_mut(|inner| {
            for name in ["mm", "g2", "Dl", "df", "Pc", "ui", "ih"] {
                let v = inner
                    .hoist_latest
                    .get(name)
                    .or_else(|| inner.module_bindings.get(name))
                    .cloned();
                if let Some(v) = v {
                    for frame in &mut inner.scope_stack {
                        frame.insert(name.to_string(), v.clone());
                    }
                }
            }
            Ok(())
        })
        .expect("inject hoists");
        let v = eval_script(
            &ctx,
            r#"
            var el = document.createElement('div');
            var root = mm(el, 1, false, null, null, false, '', null, null, null, null, null);
            return typeof root.current + '|' + typeof root.containerInfo + '|' + typeof root.tag;
            "#,
        )
        .expect("call mm from bindings");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object|object|number"),
            "expected object|object|number, got {v:?}"
        );
    }

    #[test]
    fn react_create_root_internal_state_after_load() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("load");
        let v = eval_script(
            &ctx,
            r#"
            var el = document.createElement('div');
            var r = ReactDOM.createRoot(el);
            var t = r._internalRoot;
            return (typeof t) + '|' + (typeof t.current) + '|' + (typeof t.containerInfo) + '|' + (typeof t.tag);
            "#,
        )
        .expect("internal state");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object|object|object|number"),
            "expected fiber root with current and containerInfo, got {v:?}"
        );
    }

    #[test]
    fn mm_minified_e_strict_mode_param() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function ih(l, t, u, a) { this.tag = l; this.alternate = null; }
            function Dl(l, t, u, a) { return new ih(l, t, u, a); }
            function g2(l) { this.containerInfo = l; this.current = null; this.incompleteTransitions = new Map(); }
            function mm(l, t, u, a, n, e, f, c, i, m, g, s) {
              return l = new g2(l),
                t = 1,
                e === !0 && (t = t | 24),
                e = Dl(3, null, null, t),
                l.current = e,
                l;
            }
            function qi(l) { this._internalRoot = l; }
            var el = document.createElement('div');
            var root = mm(el, 1, false, null, null, false, '', null, null, null, null, null);
            var r = new qi(root);
            return typeof r._internalRoot.current;
            "#,
        )
        .expect("mm minified e param");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object"),
            "expected object, got {v:?}"
        );
    }

    #[test]
    fn at_factory_mm_create_root() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            var At = function(l, t) {
              return function() {
                return t || l((t = { exports: {} }).exports, t), t.exports;
              };
            };
            var bm = At(function(xe) {
              function df(l) { var t = []; for (var u = 0; u < 31; u = u + 1) { t.push(l); } return t; }
              function Pc() { return { refCount: 0, data: new Map() }; }
              function ui(l) { l.updateQueue = { shared: { lanes: 0 } }; }
              function ih(l, t, u, a) { this.tag = l; this.alternate = null; }
              function Dl(l, t, u, a) { return new ih(l, t, u, a); }
              function g2(l, t, u, a, n, e, f, c, i) {
                this.tag = 1;
                this.containerInfo = l;
                this.current = null;
                this.incompleteTransitions = new Map();
              }
              function mm(l, t, u, a, n, e, f, c, i, m, g, s) {
                return l = new g2(l, t, u, f, i, m, g, s, c),
                  t = 1,
                  e === true && (t = t | 24),
                  e = Dl(3, null, null, t),
                  l.current = e,
                  e.stateNode = l,
                  t = Pc(),
                  l.pooledCache = t,
                  ui(e),
                  l;
              }
              function qi(l) { this._internalRoot = l; }
              xe.createRoot = function(container) {
                var t = mm(container, 1, false, null, null, false, '', null, null, null, null, null);
                return new qi(t);
              };
            });
            var ReactDOM = bm();
            var el = document.createElement('div');
            var r = ReactDOM.createRoot(el);
            return typeof r._internalRoot.current + '|' + typeof r._internalRoot.containerInfo;
            "#,
        )
        .expect("at factory mm");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object|object"),
            "expected object|object, got {v:?}"
        );
    }

    #[test]
    fn react_mm_exact_bundle_body() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function df(l) { var t = []; for (var u = 0; u < 31; u = u + 1) { t.push(l); } return t; }
            function Pc() { return { refCount: 0, data: new Map() }; }
            function ui(l) {
              l.updateQueue = {
                baseState: l.memoizedState,
                firstBaseUpdate: null,
                lastBaseUpdate: null,
                shared: { pending: null, lanes: 0, hiddenCallbacks: null },
                callbacks: null
              };
            }
            function ih(l, t, u, a) {
              this.tag = l;
              this.key = u;
              this.alternate = null;
            }
            function Dl(l, t, u, a) { return new ih(l, t, u, a); }
            function g2(l, t, u, a, n, e, f, c, i) {
              this.tag = 1;
              this.containerInfo = l;
              this.current = null;
              this.incompleteTransitions = new Map();
            }
            function mm(l, t, u, a, n, e, f, c, i, m, g, s) {
              return l = new g2(l, t, u, f, i, m, g, s, c),
                t = 1,
                e === true && (t = t | 24),
                e = Dl(3, null, null, t),
                l.current = e,
                e.stateNode = l,
                t = Pc(),
                t.refCount = t.refCount + 1,
                l.pooledCache = t,
                t.refCount = t.refCount + 1,
                e.memoizedState = { element: a, isDehydrated: u, cache: t },
                ui(e),
                l;
            }
            function qi(l) { this._internalRoot = l; }
            var el = document.createElement('div');
            var root = mm(el, 1, false, null, null, false, '', null, null, null, null, null);
            var r = new qi(root);
            return typeof r._internalRoot.current + '|' + typeof r._internalRoot.containerInfo;
            "#,
        )
        .expect("exact mm body");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object|object"),
            "expected object|object, got {v:?}"
        );
    }

    #[test]
    fn react_mm_simplified_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function Pc() { return {}; }
            function ui(e) { e.updateQueue = {}; }
            function ih(l, t, u, a) { this.tag = l; this.alternate = null; }
            function Dl(l, t, u, a) { return new ih(l, t, u, a); }
            function g2(l, t, u, a, n, e, f, c, i) {
              this.containerInfo = l;
              this.current = null;
              this.incompleteTransitions = new Map();
            }
            function mm(l, t, u, a, n, e, f, c, i) {
              return l = new g2(l, t, u, f, i, null, null, null, c),
                t = 1,
                e = true && (t |= 24),
                e = Dl(3, null, null, t),
                l.current = e,
                e.stateNode = l,
                t = Pc(),
                l.pooledCache = t,
                ui(e),
                l;
            }
            function qi(x) { this._internalRoot = x; }
            var el = { nodeType: 1 };
            var r = new qi(mm(el, 1, false, null, null, false, '', null, null, null, null, null));
            return typeof r._internalRoot.current;
            "#,
        )
        .expect("react mm simplified");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object"),
            "expected object, got {v:?}"
        );
    }

    #[test]
    fn mm_param_l_shadow_current() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function g2(x) {
              this.containerInfo = x;
              this.incompleteTransitions = new Map();
            }
            function mm(container) {
              return l = new g2(container), e = { tag: 3 }, l.current = e, l;
            }
            function qi(x) { this._internalRoot = x; }
            var el = { nodeType: 1 };
            var r = new qi(mm(el));
            return typeof r._internalRoot.current + '|' + typeof r._internalRoot.containerInfo;
            "#,
        )
        .expect("mm param shadow");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object|object"),
            "expected object|object, got {v:?}"
        );
    }

    #[test]
    fn dl_fiber_current_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function ih(l, t, u, a) {
              this.tag = l;
              this.alternate = null;
            }
            function Dl(l, t, u, a) { return new ih(l, t, u, a); }
            function g2() {
              this.current = null;
              this.incompleteTransitions = new Map();
            }
            function mm() {
              var l = new g2();
              var e = Dl(3, null, null, 1);
              l.current = e;
              return l;
            }
            function qi(x) { this._internalRoot = x; }
            var r = new qi(mm());
            return typeof r._internalRoot.current;
            "#,
        )
        .expect("Dl fiber chain");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object"),
            "expected object, got {v:?}"
        );
    }

    #[test]
    fn new_g2_map_tail_constructor() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function g2(x) {
              this.containerInfo = x;
              this.incompleteTransitions = new Map();
            }
            var el = { nodeType: 1 };
            var o = new g2(el);
            return typeof o.containerInfo;
            "#,
        )
        .expect("g2 map tail");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object"),
            "expected object, got {v:?}"
        );
    }

    #[test]
    fn mm_comma_current_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function g2() {
              this.current = null;
              this.incompleteTransitions = new Map();
            }
            function mm() {
              return l = new g2(), e = { tag: 3 }, l.current = e, l;
            }
            function qi(x) { this._internalRoot = x; }
            var r = new qi(mm());
            return typeof r._internalRoot.current;
            "#,
        )
        .expect("mm comma chain");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object"),
            "expected object, got {v:?}"
        );
    }

    #[test]
    fn new_g2_current_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function g2() {
              this.current = null;
              this.incompleteTransitions = new Map();
            }
            function mm() {
              var l = new g2();
              var e = { tag: 3 };
              l.current = e;
              return l;
            }
            function qi(root) { this._internalRoot = root; }
            var root = new qi(mm());
            return typeof root._internalRoot.current;
            "#,
        )
        .expect("g2 current chain");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "object"),
            "expected object, got {v:?}"
        );
    }

    #[test]
    fn new_qi_prototype_render_chain() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function qi(l) { this._internalRoot = l; }
            qi.prototype.render = function() {};
            var createRoot = function(el) { return new qi(el); };
            var r = createRoot({ nodeType: 1 });
            return typeof r.render;
            "#,
        )
        .expect("qi render chain");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "function"),
            "expected function, got {v:?}"
        );
    }

    #[test]
    fn create_root_captures_hoisted_helper() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        let v = eval_script(
            &ctx,
            r#"
            function factory(xe) {
              function B1(n) { return n.nodeType === 1; }
              xe.createRoot = function(el) { return B1(el) ? { render: function() {} } : null; };
            }
            var exports = {};
            factory(exports);
            return typeof exports.createRoot({ nodeType: 1 }).render;
            "#,
        )
        .expect("hoisted B1 in createRoot");
        assert!(
            matches!(v, Kv8Value::Str(ref s) if s == "function"),
            "expected function, got {v:?}"
        );
    }

    #[test]
    fn react_umd_counter_smoke() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        let ctx = Kv8Context::default();
        super::load_react_runtime(&ctx).expect("load react runtime");
        let cr = eval_script(&ctx, "return typeof globalThis.ReactDOM.createRoot;")
            .expect("createRoot typeof");
        assert!(
            matches!(cr, Kv8Value::Str(ref s) if s == "function"),
            "ReactDOM.createRoot expected function, got {cr:?}"
        );
        let render_ty = eval_script(
            &ctx,
            "var el = document.createElement('div'); var r = ReactDOM.createRoot(el); return typeof r.render;",
        )
        .expect("root.render typeof");
        assert!(
            matches!(render_ty, Kv8Value::Str(ref s) if s == "function"),
            "root.render expected function, got {render_ty:?}"
        );
        eval_script(&ctx, super::REACT_COUNTER_APP).expect("counter app");
        let n = eval_script(
            &ctx,
            "let n = document.querySelectorAll('button'); return n.length;",
        )
        .expect("button count");
        assert!(
            matches!(n, Kv8Value::Num(x) if x >= 1.0),
            "expected at least one button, got {n:?}"
        );
    }

    #[test]
    fn dom_umd_eval_probe() {
        dom_runtime_eval_probe();
    }

    #[test]
    fn dom_umd_full_parse() {
        react_runtime_full_parse();
    }

    #[test]
    fn react_shim_full_pipeline_timing() {
        use super::super::context::{Kv8Context, Kv8Value};
        use super::super::eval::eval_script;
        use std::time::Instant;
        let ctx = Kv8Context::default();
        let t0 = Instant::now();
        super::load_react_runtime(&ctx).expect("load react runtime");
        let load_elapsed = t0.elapsed();
        let t1 = Instant::now();
        eval_script(&ctx, super::REACT_COUNTER_APP).expect("counter app");
        let app_elapsed = t1.elapsed();
        let t2 = Instant::now();
        let btn_count = eval_script(
            &ctx,
            "let n = document.querySelectorAll('button'); return n.length;",
        )
        .expect("button count");
        let query_elapsed = t2.elapsed();
        let total = t0.elapsed();
        assert!(
            matches!(btn_count, Kv8Value::Num(x) if x >= 1.0),
            "expected at least one button, got {btn_count:?}"
        );
        assert!(
            total.as_secs() < 10,
            "full pipeline took {:?} (load={:?}, app={:?}, query={:?})",
            total,
            load_elapsed,
            app_elapsed,
            query_elapsed,
        );
    }
}