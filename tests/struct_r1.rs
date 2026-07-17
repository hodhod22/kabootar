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
