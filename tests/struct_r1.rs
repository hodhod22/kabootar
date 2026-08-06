//! R1 — struct + self receiver.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn struct_point_self_methods() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
struct Point {
    x: number;
    y: number;

    fn init(a, b) {
        self.x = a
        self.y = b
    }

    fn sum(self) {
        return self.x + self.y
    }
}

let p = Point(3, 4)
return p.sum()
"#,
        &mut env,
    )
    .expect("struct Point");
    assert!(matches!(v, Value::Number(7)), "got {v:?}");
}

#[test]
fn class_still_uses_this() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
class Point {
    x: number;
    y: number;

    fn init(a, b) {
        this.x = a
        this.y = b
    }

    fn sum() {
        return this.x + this.y
    }
}

let p = Point(1, 2)
return p.sum()
"#,
        &mut env,
    )
    .expect("class Point");
    assert!(matches!(v, Value::Number(3)), "got {v:?}");
}

#[test]
fn struct_ref_self_methods() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
struct Counter {
    n: number;

    fn init(v) {
        self.n = v
    }

    fn get(&self) {
        return self.n
    }

    fn bump(&mut self) {
        self.n = self.n + 1
        return self.n
    }
}

let c = Counter(10)
let a = c.get()
let b = c.bump()
return a + b
"#,
        &mut env,
    )
    .expect("struct &self");
    assert!(matches!(v, Value::Number(21)), "got {v:?}");
}

#[test]
fn class_rejects_self_receiver_message() {
    let mut env = create_global_env();
    let err = eval_source(
        r#"
class Bad {
    fn oops(self) {
        return 1
    }
}
"#,
        &mut env,
    )
    .expect_err("class should reject self");
    assert!(
        err.contains("this") || err.contains("self"),
        "{err}"
    );
}
