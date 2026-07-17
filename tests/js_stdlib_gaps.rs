//! JS stdlib gaps — String.at / well-formed / Math constants / Object prototype aliases.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn string_at_and_method() {
    let mut env = create_global_env();
    let v = eval_source(r#"str_at("abc", -1)"#, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "c"));
    let v = eval_source(r#""hi".at(0)"#, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "h"));
}

#[test]
fn string_well_formed() {
    let mut env = create_global_env();
    let v = eval_source(r#"is_well_formed("ok")"#, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
    let v = eval_source(r#"to_well_formed("ok")"#, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "ok"));
}

#[test]
fn string_concat_and_raw() {
    let mut env = create_global_env();
    let v = eval_source(r#"string_concat("a", "b", 1)"#, &mut env).unwrap();
    assert!(matches!(v, Value::String(s) if s == "ab1"));
    let v = eval_source(
        r#"string_raw({ raw: ["a", "b"] }, "X")"#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::String(s) if s == "aXb"));
}

#[test]
fn string_normalize_nfkc() {
    let mut env = create_global_env();
    let v = eval_source(r#"str_normalize("é", "NFKC")"#, &mut env).unwrap();
    assert!(matches!(v, Value::String(_)));
}

#[test]
fn math_constants_and_namespace() {
    let mut env = create_global_env();
    let v = eval_source(r#"Math.LN2"#, &mut env).unwrap();
    assert!(matches!(v, Value::Float(f) if (f - 0.693).abs() < 0.01));
    let v = eval_source(r#"Math.floor(3.9)"#, &mut env).unwrap();
    assert!(matches!(v, Value::Number(3)));
    let v = eval_source(r#"ln2()"#, &mut env).unwrap();
    assert!(matches!(v, Value::Float(_)));
}

#[test]
fn number_namespace_constants() {
    let mut env = create_global_env();
    let v = eval_source(r#"Number.EPSILON > 0"#, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
    let v = eval_source(r#"Number.isInteger(3)"#, &mut env).unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn object_parent_model() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
let base = { x: 7 }
let o = Object.create(base)
let p = Object.getParent(o)
p.x
"#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(7)));

    let v = eval_source(
        r#"
let base = { y: 3 }
let o = Object.create(null)
o = Object.setParent(o, base)
Reflect.getParent(o).y
"#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(3)));
}

#[test]
fn object_define_properties_and_descriptors() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
let o = {}
o = Object.defineProperties(o, {
  a: { value: 2, writable: true, enumerable: true, configurable: true }
})
Object.getOwnPropertyDescriptors(o).a.value
"#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn logical_assign_operators() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
let a = 0
a ||= 5
let b = 1
b &&= 9
let c = null
c ??= 3
return a + b + c
"#,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Number(17)), "got {v:?}");
}

#[test]
fn promise_try_namespace() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
Promise.try(() => 7)
"#,
        &mut env,
    )
    .unwrap();
    match v {
        Value::Promise(p) => {
            kabootar_lib::evaluator::drain_all_microtasks(&mut env).unwrap();
            assert!(matches!(
                *p.borrow(),
                kabootar_lib::value::PromiseValue::Resolved(Value::Number(7))
            ));
        }
        other => panic!("expected Promise, got {other:?}"),
    }
}

