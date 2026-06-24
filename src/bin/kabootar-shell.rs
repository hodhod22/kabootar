fn main() {
    if let Err(e) = kabootar::shell::run_desktop() {
        eprintln!("kabootar-shell: {e}");
        std::process::exit(1);
    }
}
