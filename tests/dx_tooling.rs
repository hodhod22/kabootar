//! DX tooling — cli/log/validate/auth/test/mock + doc/fmt helpers.

use kabootar_lib::cli::{self, extract_kab_docs, format_kabootar_source, registry_render_index};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn env_host() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

fn eval(code: &str) -> Value {
    env_host();
    let mut env = create_global_env();
    eval_source(code, &mut env).expect("eval")
}

#[test]
fn dx_kab_modules_smoke() {
    let v = eval(
        r#"
        import "cli"
        import "log"
        import "validate"
        import "auth"
        import "test"
        import "test/mock"
        let p = parseArgs(["--x"])
        let log = create("error")
        let m = mockFn("f")
        m = returns(m, 1)
        let s = createSession("u", {})
        let jwt = issueJwt({ "sub": "u", "role": "dev" }, "s3cr3t")
        let claims = verifyJwt(jwt, "s3cr3t")
        let a = hasFlag(p, "x")
        let b = debug(log, "n") == false
        let c = call(m, []) == 1
        let d = verifySession(s, s["token"])
        let e = is_email("t@e.com")
        let f = claims != null && claims["sub"] == "u"
        a && b && c && d && e && f
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn dx_doc_extract_and_fmt() {
    let src = "/// Adds one\npub fn add1(x) {\nreturn x + 1\n}\n";
    let items = extract_kab_docs(src, "t.kab");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "add1");
    let formatted = format_kabootar_source("fn x() {\nreturn 1\n}\n");
    assert!(formatted.contains("    return 1"));
    let spaced = format_kabootar_source("// keep\nfn y() {  return   2  }\n");
    assert!(spaced.contains("// keep"));
    assert!(spaced.contains("return 2"));
    let html = registry_render_index(&[]);
    assert!(html.contains("Kabootar local registry"));
}

#[test]
fn dx_cli_help_ok() {
    assert_eq!(cli::run(&["--help".into()]), 0);
}
