//! DX0–DX2 — session / REPL helpers / notebook smoke.

use kabootar_lib::notebook::{parse_notebook, run_notebook};
use kabootar_lib::session::{needs_more_input, strip_continuations, Session};
use kabootar_lib::value::Value;

#[test]
fn session_persistent_and_underscore() {
    let mut s = Session::new();
    let v = s.eval_cell("1 + 2").expect("eval");
    assert!(matches!(v, Value::Number(3)));
    let u = s.eval_cell("_").expect("underscore");
    assert!(matches!(u, Value::Number(3)));
    let w = s.eval_cell("let x = _ + 7\nx").expect("bind");
    assert!(matches!(w, Value::Number(10)));
}

#[test]
fn multiline_detection() {
    assert!(needs_more_input("fn f() {"));
    assert!(needs_more_input("let a = [1,"));
    assert!(!needs_more_input("1 + 2"));
    assert_eq!(strip_continuations("let a = 1 \\\n+ 2"), "let a = 1\n+ 2");
}

#[test]
fn notebook_cells_share_env() {
    let nb = parse_notebook(
        r#"{
          "version": 1,
          "cells": [
            { "id": "a", "source": "let n = 10" },
            { "id": "b", "source": "n * 2" }
          ]
        }"#,
    )
    .expect("parse");
    let (session, results) = run_notebook(&nb, false).expect("run");
    assert_eq!(results.len(), 2);
    assert!(results[1].ok);
    assert_eq!(results[1].output, "20");
    assert!(matches!(session.env.get("n"), Some(Value::Number(10))));
}

#[test]
fn explore_smoke_knb_with_science() {
    let path = format!("{}/examples/explore_smoke.knb", env!("CARGO_MANIFEST_DIR"));
    let nb = kabootar_lib::notebook::load_notebook(std::path::Path::new(&path)).expect("load");
    let (_s, results) = run_notebook(&nb, true).expect("run science notebook");
    assert!(results.last().unwrap().ok);
    assert_eq!(results.last().unwrap().output, "true");
}
