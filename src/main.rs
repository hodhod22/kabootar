use kabootar_lib::cli;
use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = cli::run(&args);
    if code != 0 {
        std::process::exit(code);
    }
}
