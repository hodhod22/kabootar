//! Native Kabootar generics (v1) — parse, compile, run.

use kabootar_lib::ast::{Expr, KabType, Stmt};
use kabootar_lib::bytecode::{compile_source, run_module, try_compile};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

fn parse_fn(source: &str) -> Expr {
    let program = compile_source(source).expect("parse");
    match program.stmts.last() {
        Some(Stmt::Expr(expr)) => expr.clone(),
        other => panic!("expected fn stmt, got {other:?}"),
    }
}

#[test]
fn generic_fn_parse_type_params_and_annotations() {
    let expr = parse_fn("fn id<T>(x: T) -> T { return x }");
    let Expr::Function {
        name,
        type_params,
        params,
        return_type,
        ..
    } = expr
    else {
        panic!("expected Function");
    };
    assert_eq!(name, "id");
    assert_eq!(type_params, vec!["T"]);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].type_ann, Some(KabType::Named("T".into())));
    assert_eq!(return_type, Some(KabType::Named("T".into())));
}

#[test]
fn generic_call_parse_type_args() {
    let program = compile_source("let y = id<Number>(42)").expect("parse");
    let Stmt::Let { init, .. } = &program.stmts[0] else {
        panic!("expected let");
    };
    let Some(init) = init else {
        panic!("expected let init");
    };
    let Expr::Call {
        func,
        type_args,
        args,
    } = init
    else {
        panic!("expected call");
    };
    assert!(matches!(func.as_ref(), Expr::Variable(s) if s == "id"));
    assert_eq!(type_args, &vec!["Number"]);
    assert_eq!(args.len(), 1);
}

#[test]
fn generic_id_number_and_string() {
    let src = r#"
fn id<T>(x: T) -> T {
    return x
}
return id(42)
"#;
    let mut env = create_global_env();
    let v = eval_source(src, &mut env).unwrap();
    assert!(matches!(v, Value::Number(42)));

    let src2 = r#"
fn id<T>(x: T) -> T {
    return x
}
return id("hi")
"#;
    let mut env2 = create_global_env();
    let v2 = eval_source(src2, &mut env2).unwrap();
    assert!(matches!(v2, Value::String(ref s) if s == "hi"));
}

#[test]
fn generic_pair_two_type_params() {
    let src = r#"
fn pair<A, B>(a: A, b: B) -> Array {
    return [a, b]
}
return len(pair(1, "x"))
"#;
    let mut env = create_global_env();
    let v = eval_source(src, &mut env).unwrap();
    assert!(matches!(v, Value::Number(2)));
}

#[test]
fn generic_monomorphized_symbols_in_bytecode() {
    let src = r#"
fn id<T>(x: T) -> T {
    return x
}
return id(42)
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(
        module.functions.iter().any(|f| f.name == "id$Number"),
        "expected specialization id$Number, got {:?}",
        module
            .functions
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(42)));
}

#[test]
fn generic_explicit_type_arg() {
    let src = r#"
fn id<T>(x: T) -> T {
    return x
}
return id<Number>(42)
"#;
    let mut env = create_global_env();
    let v = eval_source(src, &mut env).unwrap();
    assert!(matches!(v, Value::Number(42)));
}

#[test]
fn generic_swap_example_from_docs() {
    let src = r#"
fn swap<T>(a: T, b: T) -> Array {
    return [b, a]
}
return len(swap(1, 2)) + len(swap("a", "b"))
"#;
    let mut env = create_global_env();
    let v = eval_source(src, &mut env).unwrap();
    assert!(matches!(v, Value::Number(4)));
}

#[test]
fn generic_infer_from_local_variable() {
    let src = r#"
fn id<T>(x: T) -> T {
    return x
}
let n = 42
return id(n)
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(
        module.functions.iter().any(|f| f.name == "id$Number"),
        "expected id$Number specialization"
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(42)));
}

#[test]
fn generic_infer_pair_from_two_variables() {
    let src = r#"
fn pair<A, B>(a: A, b: B) -> Array {
    return [a, b]
}
let x = 1
let s = "a"
return len(pair(x, s))
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(
        module
            .functions
            .iter()
            .any(|f| f.name == "pair$Number_String"),
        "expected pair$Number_String, got {:?}",
        module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(2)));
}

#[test]
fn generic_infer_from_class_instance_variable() {
    let src = r#"
class Box {
    value: number;

    fn init(v) {
        self.value = v
    }
}

fn id<T>(x: T) -> T {
    return x
}

let b = Box(42)
return id(b)
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(
        module.functions.iter().any(|f| f.name == "id$Box"),
        "expected id$Box specialization"
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::ClassInstance(_)));
}

#[test]
fn generic_class_method_parse_type_params() {
    let src = r#"
class Holder {
    fn echo<T>(x) { return x }
}
"#;
    let program = compile_source(src).expect("parse");
    let Stmt::Class { methods, .. } = &program.stmts[0] else {
        panic!("expected class");
    };
    assert_eq!(methods[0].name, "echo");
    assert_eq!(methods[0].type_params, vec!["T"]);
}

#[test]
fn generic_class_method_monomorphize() {
    let src = r#"
class Holder {
    fn init() {
    }

    fn echo<T>(x) {
        return x
    }
}

let h = Holder()
return h.echo(42)
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    let holder = module
        .classes
        .iter()
        .find(|c| c.name == "Holder")
        .expect("Holder class");
    assert!(
        holder.methods.iter().any(|m| m.name == "echo$Number"),
        "expected echo$Number on class, got {:?}",
        holder
            .methods
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        !holder.methods.iter().any(|m| m.name == "echo"),
        "generic echo template should not appear as plain echo"
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(42)));
}

#[test]
fn generic_class_method_two_specializations() {
    let src = r#"
class Holder {
    fn init() {
    }

    fn echo<T>(x) {
        return x
    }
}

let h = Holder()
let a = h.echo(1)
let b = h.echo("x")
return a + len(b)
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    let holder = module.classes.iter().find(|c| c.name == "Holder").unwrap();
    assert!(holder.methods.iter().any(|m| m.name == "echo$Number"));
    assert!(holder.methods.iter().any(|m| m.name == "echo$String"));
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(2)));
}

#[test]
fn generic_class_parse_type_params() {
    let src = r#"
class Box<T> {
    value: number;
}
"#;
    let program = compile_source(src).expect("parse");
    let Stmt::Class { name, type_params, .. } = &program.stmts[0] else {
        panic!("expected class");
    };
    assert_eq!(name, "Box");
    assert_eq!(type_params, &vec!["T"]);
}

#[test]
fn generic_class_ctor_monomorphize_infer() {
    let src = r#"
class Box<T> {
    value: number;

    fn init(v) {
        self.value = 0
    }
}

let b = Box(42)
return b.value
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(
        module.classes.iter().any(|c| c.name == "Box$Number"),
        "expected Box$Number class, got {:?}",
        module.classes.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()
    );
    assert!(
        !module.classes.iter().any(|c| c.name == "Box"),
        "generic Box template should not appear as plain Box"
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(0)));
}

#[test]
fn generic_class_two_specializations_same_module() {
    let src = r#"
class Box<T> {
    tag: number;

    fn init(v) {
        self.tag = 1
    }
}

let a = Box(42)
let b = Box("x")
return a.tag + b.tag
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(module.classes.iter().any(|c| c.name == "Box$Number"));
    assert!(module.classes.iter().any(|c| c.name == "Box$String"));
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(2)));
}

#[test]
fn generic_class_explicit_type_arg() {
    let src = r#"
class Box<T> {
    tag: number;

    fn init(v) {
        self.tag = 1
    }
}

let b = Box<String>("hi")
return b.tag
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(module.classes.iter().any(|c| c.name == "Box$String"));
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(1)));
}

#[test]
fn generic_method_on_specialized_class() {
    let src = r#"
class Box<T> {
    fn init(v) {
    }

    fn echo<U>(x) {
        return x
    }
}

let b = Box(42)
return b.echo(1)
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    let box_num = module
        .classes
        .iter()
        .find(|c| c.name == "Box$Number")
        .expect("Box$Number class");
    assert!(
        box_num.methods.iter().any(|m| m.name == "echo$Number"),
        "expected echo$Number on Box$Number, got {:?}",
        box_num
            .methods
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        !box_num.methods.iter().any(|m| m.name == "echo"),
        "generic echo template should not appear as plain echo"
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(1)));
}

#[test]
fn generic_enum_parse_type_params() {
    let src = r#"
enum Option<T> {
    Some(T),
    None
}
"#;
    let program = compile_source(src).expect("parse");
    let Stmt::Enum {
        name,
        type_params,
        variants,
    } = &program.stmts[0]
    else {
        panic!("expected enum");
    };
    assert_eq!(name, "Option");
    assert_eq!(type_params, &vec!["T"]);
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0].name, "Some");
    assert_eq!(variants[0].fields, vec!["T"]);
}

#[test]
fn generic_enum_some_monomorphize_infer() {
    let src = r#"
enum Option<T> {
    Some(T),
    None
}

Option.Some(42)
return 1
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(
        module.enums.iter().any(|e| e.name == "Option$Number"),
        "expected Option$Number enum, got {:?}",
        module.enums.iter().map(|e| e.name.as_str()).collect::<Vec<_>>()
    );
    assert!(
        !module.enums.iter().any(|e| e.name == "Option"),
        "generic Option template should not appear as plain Option"
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(1)));
}

#[test]
fn generic_enum_two_specializations_same_module() {
    let src = r#"
enum Option<T> {
    Some(T),
    None
}

Option.Some(1)
Option.Some("x")
return 1
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(module.enums.iter().any(|e| e.name == "Option$Number"));
    assert!(module.enums.iter().any(|e| e.name == "Option$String"));
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(1)));
}

#[test]
fn generic_enum_explicit_type_arg_none() {
    let src = r#"
enum Option<T> {
    Some(T),
    None
}

let x = Option<Number>.None
return 1
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    assert!(module.enums.iter().any(|e| e.name == "Option$Number"));
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(1)));
}

#[test]
fn generic_enum_match_some_in_bytecode() {
    let src = r#"
enum Option<T> {
    Some(T),
    None
}

let x = Option.Some(42)
let y = match x {
    Option.Some(n) => n
    Option.None => 0
}
return y
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    use kabootar_lib::bytecode::Opcode;
    assert!(
        module
            .main_code
            .iter()
            .any(|op| matches!(op, Opcode::JumpUnlessEnumVariant(_, _, _))),
        "expected enum variant match opcode"
    );
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(42)));
}

#[test]
fn generic_class_inheritance_resolves_extends() {
    let src = r#"
class Base<T> {
    tag: Number

    fn init(v) {
        self.tag = 1
    }
}

class Child<T> extends Base<T> {
}

let c = Child(42)
return c.tag
"#;
    let program = compile_source(src).expect("parse");
    let module = try_compile(&program.stmts).expect("bytecode compile");
    let child = module
        .classes
        .iter()
        .find(|c| c.name == "Child$Number")
        .expect("Child$Number");
    assert_eq!(child.extends.as_deref(), Some("Base$Number"));
    let mut env = create_global_env();
    let result = run_module(&module, &mut env).expect("run");
    assert!(matches!(result, Value::Number(1)));
}
