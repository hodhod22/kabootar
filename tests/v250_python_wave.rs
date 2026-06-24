//! Python wave (no elif): pass, raise, is/is not, assert, with, range, Counter, defaultdict, iterators.

use kabootar::evaluator::{create_global_env, eval_source};
use kabootar::runtime::stdlib::error::{take_throw_value, THROW_MARKER};
use kabootar::value::format_value;

fn evaluate(code: &str) -> String {
    let mut env = create_global_env();
    format_value(&eval_source(code, &mut env).unwrap())
}

#[test]
fn pass_and_assert() {
    assert_eq!(
        evaluate(
            r#"
            let n = 0
            pass
            assert n == 0
            n
        "#
        ),
        "0"
    );
}

#[test]
fn is_identity_and_not() {
    assert_eq!(
        evaluate(
            r#"
            let a = null
            let b = null
            let r1 = a is null
            let r2 = a is not undefined
            let r3 = a is b
            r1 && r2 && r3
        "#
        ),
        "true"
    );
    assert_eq!(
        evaluate(
            r#"
            let m1 = map_new()
            let m2 = map_new()
            let same = m1 is m1
            let diff = m1 is not m2
            same && diff
        "#
        ),
        "true"
    );
}

#[test]
fn raise_alias_throw() {
    let err = eval_source("raise \"boom\"", &mut create_global_env()).unwrap_err();
    assert_eq!(err, THROW_MARKER);
    assert_eq!(
        format_value(&take_throw_value(&err).unwrap()),
        "boom"
    );
}

#[test]
fn range_for_of() {
    assert_eq!(
        evaluate(
            r#"
            let sum = 0
            for i of range(3) { sum = sum + i }
            sum
        "#
        ),
        "3"
    );
    assert_eq!(
        evaluate(
            r#"
            iterator_chain(range(1, 3), [10])
        "#
        ),
        "[1, 2, 10]"
    );
}

#[test]
fn counter_and_defaultdict() {
    assert_eq!(
        evaluate(
            r#"
            let c = counter_new()
            counter_inc(c, "a")
            counter_inc(c, "a", 2)
            counter_get(c, "a")
        "#
        ),
        "3"
    );
    assert_eq!(
        evaluate(
            r#"
            let d = defaultdict_new([])
            let v1 = defaultdict_get(d, "x")
            defaultdict_set(d, "x", [1])
            let v2 = defaultdict_get(d, "y")
            len(v1) + len(v2)
        "#
        ),
        "0"
    );
}

#[test]
fn iterator_accumulate_pairwise() {
    assert_eq!(
        evaluate(
            r#"
            for_of_items(iterator_accumulate([1, 2, 3, 4], (a, b) => a + b))
        "#
        ),
        "[1, 3, 6, 10]"
    );
    assert_eq!(
        evaluate(
            r#"
            for_of_items(iterator_pairwise([1, 2, 3]))
        "#
        ),
        "[[1, 2], [2, 3]]"
    );
}

#[test]
fn with_binds_resource() {
    assert_eq!(
        evaluate(
            r#"
            let resource = { value: 7 }
            with resource as r { r.value }
        "#
        ),
        "7"
    );
}
