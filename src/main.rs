use kabootar_lib::cli;
use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    // Self-host compile/eval recurses too deep for the default main-thread
    // stack — run the CLI on a 64 MiB worker, same as the test harness.
    let code = std::thread::Builder::new()
        .name("kabootar-cli".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || cli::run(&args))
        .expect("spawn cli thread")
        .join()
        .expect("join cli thread");
    if code != 0 {
        std::process::exit(code);
    }
}
