//! SH18: shared object mutation regression gate — aliases and fn-param
//! writes must be visible through every binding of the object (Kab-VM path).

use kabootar_lib::evaluator::create_global_env;

#[test]
fn sh18_shared_obj_mut_exec_smoke() {
    let path = format!(
        "{}/examples/sh18_shared_obj_mut_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh18-shared-obj-mut".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile shared-obj-mut smoke");
            let value = eval_program(&program, &mut env).expect("run shared-obj-mut smoke");
            assert!(
                matches!(value, kabootar_lib::value::Value::Bool(true)),
                "shared object mutation must be visible through all bindings, got {value:?}"
            );
        })
        .expect("spawn")
        .join()
        .expect("join");
}
