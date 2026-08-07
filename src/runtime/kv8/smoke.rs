//! Self-hosting smoke probes — which Web APIs Kv8 can run without Chrome.

use super::context::{Kv8Context, Kv8Value};
use super::eval::eval_script;
use crate::value::Value;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct SmokeProbe {
    pub category: &'static str,
    pub api: &'static str,
    pub script: &'static str,
    /// P0 = needed for a minimal interactive app (React smoke path).
    pub priority: &'static str,
    /// Roadmap wave that should implement this (see docs/ROADMAP.md Våg C).
    pub wave: &'static str,
}

#[derive(Clone)]
pub struct SmokeResult {
    pub category: &'static str,
    pub api: &'static str,
    pub ok: bool,
    pub priority: &'static str,
    pub wave: &'static str,
    pub error: Option<String>,
}

pub const PROBES: &[SmokeProbe] = &[
    // --- DOM core (partial today) ---
    SmokeProbe {
        category: "DOM",
        api: "document.createElement",
        script: "document.createElement('div');",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "document.appendChild",
        script: "let n = document.createElement('div'); document.appendChild(n);",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "document.querySelector",
        script: "let n = document.createElement('div'); document.appendChild(n); document.querySelector('div');",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "element.appendChild",
        script: "let p = document.createElement('div'); let c = document.createElement('span'); p.appendChild(c);",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "element.textContent (write)",
        script: "let el = document.createElement('div'); el.textContent = 'hi';",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "element.style assignment",
        script: "let el = document.createElement('div'); el.style.color = '#ff0000';",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "document.getElementById",
        script: "document.getElementById('app');",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "document.querySelectorAll",
        script: "document.querySelectorAll('div');",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "element.innerHTML",
        script: "let el = document.createElement('div'); el.innerHTML = '<b>x</b>';",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "element.id assignment",
        script: "let el = document.createElement('div'); el.id = 'app';",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "element.setAttribute",
        script: "let el = document.createElement('div'); el.setAttribute('data-x', '1');",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "element.removeChild",
        script: "let p = document.createElement('div'); let c = document.createElement('span'); p.appendChild(c); p.removeChild(c);",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "element.firstChild",
        script: "let p = document.createElement('div'); let c = document.createElement('span'); p.appendChild(c); p.firstChild;",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "DOM",
        api: "document.body",
        script: "document.body.appendChild(document.createElement('div'));",
        priority: "P1",
        wave: "C1",
    },
    // --- Events ---
    SmokeProbe {
        category: "Events",
        api: "element.addEventListener",
        script: "let el = document.createElement('button'); el.addEventListener('click', () => {});",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "Events",
        api: "element.removeEventListener",
        script: "let el = document.createElement('button'); function f(){}; el.addEventListener('click', f); el.removeEventListener('click', f);",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "Events",
        api: "element.dispatchEvent",
        script: "let el = document.createElement('button'); el.dispatchEvent({ type: 'click' });",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "Events",
        api: "Event constructor",
        script: "new Event('click');",
        priority: "P1",
        wave: "C1",
    },
    // --- Timers / frame loop ---
    SmokeProbe {
        category: "Timers",
        api: "setTimeout",
        script: "setTimeout(() => {}, 0);",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "Timers",
        api: "setInterval",
        script: "setInterval(() => {}, 100);",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "Timers",
        api: "requestAnimationFrame",
        script: "requestAnimationFrame(() => {});",
        priority: "P1",
        wave: "C1",
    },
    // --- Network / storage ---
    SmokeProbe {
        category: "Network",
        api: "fetch",
        script: "fetch('http://localhost/');",
        priority: "P0",
        wave: "C1",
    },
    SmokeProbe {
        category: "Storage",
        api: "localStorage.setItem",
        script: "localStorage.setItem('k', 'v');",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "Storage",
        api: "sessionStorage",
        script: "sessionStorage.setItem('k', 'v');",
        priority: "P2",
        wave: "C1",
    },
    // --- Window / browser chrome ---
    SmokeProbe {
        category: "Window",
        api: "window",
        script: "window.fetch('http://127.0.0.1/');",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "Window",
        api: "window.location",
        script: "window.location.assign('kabootar://app');",
        priority: "P2",
        wave: "C1",
    },
    SmokeProbe {
        category: "Window",
        api: "navigator",
        script: "navigator.clipboard.readText();",
        priority: "P2",
        wave: "C1",
    },
    // --- JS runtime inside Kv8 ---
    SmokeProbe {
        category: "JS",
        api: "function declaration",
        script: "function f() { return 1; } f();",
        priority: "P0",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "arrow function",
        script: "let f = (x) => x + 1; f(2);",
        priority: "P0",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "for loop",
        script: "let s = 0; for (let i = 0; i < 3; i = i + 1) { s = s + i; } s;",
        priority: "P0",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "while loop",
        script: "let s = 0; let i = 0; while (i < 3) { s = s + i; i = i + 1; } s;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "break",
        script: "let s = 0; let i = 0; while (i < 10) { if (i == 3) { break; } s = s + i; i = i + 1; } s;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "continue",
        script: "let s = 0; let i = 0; while (i < 5) { i = i + 1; if (i == 3) { continue; } s = s + i; } s;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "function expression",
        script: "let f = function (x) { return x + 1; }; f(2);",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "var declaration",
        script: "var x = 1; x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "this",
        script: "let o = { x: 1, get: function () { return this.x; } }; o.get();",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "Symbol.for",
        script: "let s = Symbol.for('test'); typeof s;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "Object.assign",
        script: "let o = Object.assign({}, { a: 1 }); o.a;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "comma operator",
        script: "let x = (1, 2, 3); x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "assignment expression",
        script: "let n = 0; let x = (n = 5); x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "void",
        script: "void 0;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "undefined",
        script: "typeof undefined;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "throw",
        script: "function f() { if (0) { throw 1; } return 1; } f();",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "ternary",
        script: "let x = true ? 1 : 2; x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "switch/case",
        script: "let x = 1; switch (x) { case 1: x = 9; break; default: x = 0; } x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "for (C-style)",
        script: "let s = 0; for (var i = 0; i < 3; i++) { s = s + 1; } s;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "for-in",
        script: "let o = { a: 1 }; let k = ''; for (var p in o) { k = p; } k;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "try/catch",
        script: "let x = 0; try { throw Error('e'); } catch (e) { x = 1; } x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "regex literal",
        script: "let r = /ab+/g; typeof r;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "+=",
        script: "let n = 1; n += 2; n;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "++",
        script: "let n = 1; let x = n++; x + n;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "array literal",
        script: "let a = [1, 2]; a.length;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "bracket index",
        script: "let a = [10, 20]; a[1];",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "Array.isArray",
        script: "Array.isArray([1]);",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "Array()",
        script: "Array(2).length;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "member assign",
        script: "let o = {}; (o).x = 1; o.x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "arguments",
        script: "function f() { return arguments[0]; } f(42);",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "default param",
        script: "function g(x=1) { return x; } g();",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "method shorthand",
        script: "let o = { m() { return 1; } }; o.m();",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "for-in assign iterable",
        script: "let o = { a: 1 }; let k = ''; for (p in o) { k = p; } k;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "try/finally",
        script: "let x = 1; try { x = 2; } catch (e) {} finally { x = 3; } x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "unary minus",
        script: "let x = -1; x === -1;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "new member expr",
        script: "function E(t, o) { this.t = t; } let x = new E('a', {}); x.t;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "for comma init",
        script: "let a = 0; let i = 0; for (i = 0, a = 1; i < 2; i = i + 1) {} a;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "switch comma disc",
        script: "let s = 'a'; switch (1, s) { case 'a': 1; break; default: 0; }",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "function prototype",
        script: "function C(){} C.prototype.x = 1; C.prototype.x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "function scope",
        script: "function f(){var r={exports:{}}; return (0||(r.exports=function(){var r=1;}),r.exports=2,r.exports);} f();",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "bitwise and",
        script: "42 & 15;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "shift and ushr",
        script: "let x = 8 >> 1; let y = 8 >>> 1; x + y;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "try/finally only",
        script: "let x = 1; try { x = 2; } finally { x = 3; } x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "label break",
        script: "let x = 0; e: for (;;) { x = x + 1; if (x == 2) { break e; } } x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "do while",
        script: "let x = 0; do { x = x + 1; } while (x < 2); x;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "in operator",
        script: "Object.prototype.hasOwnProperty.call({ a: 1 }, 'a');",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "object spread",
        script: "let o = { ...{ a: 1 }, b: 2 }; o.b;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "member assign lhs order",
        script: "(function(e,n){(e=globalThis).r=n(e===globalThis?99:1);})(null,function(v){return v;});",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "string char index",
        script: "\"Ab\"[0].toUpperCase();",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "=== strict equality",
        script: "1 === 1;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "!== strict inequality",
        script: "1 !== 2;",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "typeof",
        script: "typeof 'x';",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "console.log",
        script: "console.log('kv8');",
        priority: "P0",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "JSON.parse",
        script: "JSON.parse('{\"a\":1}');",
        priority: "P1",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "Promise",
        script: "new Promise((resolve) => { resolve(1); });",
        priority: "P0",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "async/await",
        script: "async function f() { return await Promise.resolve(1); } f();",
        priority: "P0",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "Promise.then",
        script: "Promise.resolve(1).then((x) => x);",
        priority: "P1",
        wave: "C1",
    },
    SmokeProbe {
        category: "JS",
        api: "import statement",
        script: "import('./app.js');",
        priority: "P2",
        wave: "C2",
    },
    SmokeProbe {
        category: "JS",
        api: "class basic",
        script: "class A { constructor(x) { this.x = x; } get() { return this.x; } } let a = new A(42); a.get();",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "class extends + super",
        script: "class A { constructor(x) { this.x = x; } } class B extends A { constructor(x) { super(x); } } let b = new B(7); b.x;",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "class static method",
        script: "class M { static sq(x) { return x * x; } } M.sq(5);",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "for-of array",
        script: "let s = 0; for (let x of [1, 2, 3]) { s = s + x; } s;",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "optional chaining",
        script: "let o = null; let x = o?.missing; typeof x;",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "nullish coalescing",
        script: "let x = null ?? 42; x;",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "Object.keys",
        script: "let k = Object.keys({ a: 1, b: 2 }); k.length;",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "Object.assign obj_store",
        script: "let src = {}; src.version = '1.0'; let dst = Object.assign({}, src); dst.version;",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "WeakMap",
        script: "let m = new WeakMap(); let k = {}; m.set(k, 42); m.get(k);",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "Object.create",
        script: "let proto = { greet() { return 'hi'; } }; let o = Object.create(proto); o.greet();",
        priority: "P1",
        wave: "C3",
    },
    SmokeProbe {
        category: "JS",
        api: "template literal",
        script: "let n = 3; let s = `val=${n}`; s;",
        priority: "P1",
        wave: "C3",
    },
];

pub fn run_probe(_ctx: &Kv8Context, probe: &SmokeProbe) -> SmokeResult {
    let fresh = Kv8Context::default();
    match eval_script(&fresh, probe.script) {
        Ok(_) => SmokeResult {
            category: probe.category,
            api: probe.api,
            ok: true,
            priority: probe.priority,
            wave: probe.wave,
            error: None,
        },
        Err(e) => SmokeResult {
            category: probe.category,
            api: probe.api,
            ok: false,
            priority: probe.priority,
            wave: probe.wave,
            error: Some(e),
        },
    }
}

pub fn run_all_probes(ctx: &Kv8Context) -> Vec<SmokeResult> {
    PROBES.iter().map(|p| run_probe(ctx, p)).collect()
}

pub fn probe_report_value(results: &[SmokeResult]) -> Value {
    let ready: Vec<Value> = results
        .iter()
        .filter(|r| r.ok)
        .map(|r| Value::String(r.api.to_string()))
        .collect();
    let missing: Vec<Value> = results
        .iter()
        .filter(|r| !r.ok)
        .map(|r| Value::String(r.api.to_string()))
        .collect();
    let p0_missing: Vec<Value> = results
        .iter()
        .filter(|r| !r.ok && r.priority == "P0")
        .map(|r| Value::String(r.api.to_string()))
        .collect();
    let probes: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut m = HashMap::new();
            m.insert("category".into(), Value::String(r.category.to_string()));
            m.insert("api".into(), Value::String(r.api.to_string()));
            m.insert("ok".into(), Value::Bool(r.ok));
            m.insert("priority".into(), Value::String(r.priority.to_string()));
            m.insert("wave".into(), Value::String(r.wave.to_string()));
            if let Some(e) = &r.error {
                m.insert("error".into(), Value::String(e.clone()));
            }
            Value::from_object(m)
        })
        .collect();

    let mut root = HashMap::new();
    root.insert("engine".into(), Value::String("kv8".into()));
    root.insert(
        "goal".into(),
        Value::String("minimal web app without Chrome".into()),
    );
    root.insert("ready_count".into(), Value::Number(ready.len() as i64));
    root.insert("missing_count".into(), Value::Number(missing.len() as i64));
    root.insert("p0_missing_count".into(), Value::Number(p0_missing.len() as i64));
    root.insert("ready".into(), Value::from_array(ready));
    root.insert("missing".into(), Value::from_array(missing));
    root.insert("p0_missing".into(), Value::from_array(p0_missing));
    root.insert("probes".into(), Value::from_array(probes));
    Value::from_object(root)
}

/// Build a minimal static app shell using only APIs known to work today.
pub fn minimum_app_shell(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    eval_script(
        ctx,
        r#"
let root = document.createElement('div');
document.appendChild(root);
let title = document.createElement('h1');
title.textContent = 'Kabootar';
root.appendChild(title);
let btn = document.createElement('button');
btn.textContent = 'Hello Kv8';
btn.style.color = '#00ff88';
root.appendChild(btn);
return root;
"#,
    )
}

/// Minimal React-style render path: createElement + setAttribute + querySelectorAll + events.
pub fn react_smoke_path(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    eval_script(
        ctx,
        r#"
let root = document.createElement('div');
root.id = 'root';
document.body.appendChild(root);

function createElement(tag, attrs) {
  let el = document.createElement(tag);
  if (attrs) {
    if (attrs.id) { el.id = attrs.id; }
    if (attrs.className) { el.setAttribute('class', attrs.className); }
    if (attrs.text) { el.textContent = attrs.text; }
  }
  return el;
}

let count = 0;
let label = createElement('span', { text: 'Count: 0' });
let btn = createElement('button', { text: '+1' });
btn.addEventListener('click', () => {
  count = count + 1;
  label.textContent = 'Count: ' + count;
});
let app = createElement('div', { className: 'app' });
app.setAttribute('data-testid', 'app');
app.appendChild(label);
app.appendChild(btn);
root.appendChild(app);

let buttons = document.querySelectorAll('button');
return buttons.length;
"#,
    )
}

/// Load Kv8-compatible React shim + counter app bundle.
pub fn react_bundle_smoke_path(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    super::bundle::react_bundle_smoke(ctx)
}
