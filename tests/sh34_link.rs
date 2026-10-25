//! Named AOT entry after multi-module init. Kept out of sh_wave so a
//! long sh_wave run does not rebuild when this gate is compiled.

use kabootar_lib::compile::{compile_file_cached, eval_program};
use kabootar_lib::evaluator::create_global_env;

#[test]
fn sh33_aot_link_call_smoke() {
    let path = format!(
        "{}/examples/sh33/sh33_aot_link_call_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh33-aot-link-call".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile aot link call smoke");
            let value = eval_program(&program, &mut env).expect("run aot link call smoke");
            assert!(
                matches!(value, kabootar_lib::value::Value::Number(42)),
                "aot link call smoke returned {value:?}"
            );
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// Program `return 20 + 1` through the native image. Matches the mirror.
/// Does not flip nollAotReady: compileIr is not running inside the image.
#[test]
fn sh34_aot_prog_smoke() {
    let path = format!(
        "{}/examples/sh34/sh34_aot_prog_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh34-aot-prog".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile aot prog smoke");
            let value = eval_program(&program, &mut env).expect("run aot prog smoke");
            assert!(
                matches!(value, kabootar_lib::value::Value::Number(42)),
                "aot prog smoke returned {value:?}"
            );
        })
        .expect("spawn")
        .join()
        .expect("join");
}
