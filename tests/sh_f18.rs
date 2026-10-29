use kabootar_lib::compile::{compile_file_cached, eval_program};
use kabootar_lib::evaluator::create_global_env;

#[test]
fn ft_f18_measure_smoke() {
    let path = format!(
        "{}/examples/ft/ft_f18_measure_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("f18-measure".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile f18 measure");
            let value = eval_program(&program, &mut env).expect("run f18 measure");
            assert!(
                matches!(value, kabootar_lib::value::Value::Number(42)),
                "f18 measure returned {value:?}"
            );
        })
        .expect("spawn")
        .join()
        .expect("join");
}
