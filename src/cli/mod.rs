//! Kabootar CLI — `mod init`, `mod run`, `serve`, `run`, `compile`.

use crate::compile::{self};
use crate::evaluator::create_global_env;
use crate::project::manifest::load_manifest_cwd;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn format_user_error(e: &str) -> String {
    crate::runtime::stdlib::error::format_runtime_error(e)
}

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        return run_repl();
    }
    match args[0].as_str() {
        "run" => run_file_cmd(&args[1..]),
        "serve" => serve_cmd(&args[1..]),
        "shell" => shell_cmd(&args[1..]),
        "compile" => compile_cmd(&args[1..]),
        "fmt" => fmt_cmd(&args[1..]),
        "install" => install_cmd(&args[1..]),
        "publish" => publish_cmd(&args[1..]),
        "mod" => mod_cmd(&args[1..]),
        "--version" | "-V" => {
            println!("Kabootar v{VERSION}");
            0
        }
        "--help" | "-h" => {
            print_help();
            0
        }
        path if path.ends_with(".kab") || path.ends_with(".kabootar") => run_file_cmd(args),
        _ => {
            eprintln!("Unknown command: {}", args[0]);
            print_help();
            1
        }
    }
}

fn print_help() {
    println!(
        "Kabootar v{VERSION}

Usage:
  kabootar                         Interactive REPL
  kabootar run <file.kab>          Run a Kabootar script
  kabootar compile <file.kab> [--self-host|--rust]   Compile via self-host (default; Rust fallback)
  kabootar fmt <file.kab>          Format Kabootar source (basic)
  kabootar install [name@ver]      Install deps from local registry
  kabootar publish <file.kab>      Publish module to local registry
  kabootar serve [opts] <file>     Start HTTP server
  kabootar shell                   Open Kabootar OS desktop window
  kabootar mod init <web|api>  Create a new project
  kabootar mod run                 Run project entry (kabootar.toml)
  kabootar <file.kab>              Shorthand for run

Serve options:
  --port <n>       Port (default 8080 or kabootar.toml)
  --bind <addr>    Bind address (default 0.0.0.0)
  --watch          Hot reload on file changes

Examples:
  kabootar mod init api
  kabootar compile main.kab
  kabootar serve --watch main.kab
"
    );
}

pub fn run_repl() -> i32 {
    use crate::evaluator::eval_stmt;
    use crate::lexer::tokenize;
    use crate::parser::Parser;

    println!("Kabootar v{VERSION} (type :quit to exit)");
    let mut env = create_global_env();
    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();
        if input == ":quit" {
            break;
        }
        if input.is_empty() || input.starts_with("=>") {
            continue;
        }
        let tokens = match tokenize(input) {
            Ok(tokens) => tokens,
            Err(e) => {
                println!("Lexer error: {}", e);
                continue;
            }
        };
        let mut parser = Parser::with_eof(tokens);
        match parser.parse_program() {
            Ok(stmts) => {
                for stmt in stmts {
                    match eval_stmt(&stmt, &mut env) {
                        Ok(v) => println!("=> {:?}", v),
                        Err(e) => println!("Error: {}", e),
                    }
                }
            }
            Err(e) => println!("Parse error: {}", e),
        }
    }
    0
}

fn run_file_cmd(args: &[String]) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("Usage: kabootar run <file.kab>");
        return 1;
    };
    match run_file(path) {
        Ok(v) => {
            println!("=> {:?}", v);
            0
        }
        Err(e) => {
            eprintln!("Error: {}", format_user_error(&e));
            1
        }
    }
}

pub fn run_file(path: &str) -> Result<crate::value::Value, String> {
    let mut env = create_global_env();
    compile::eval_file_cached(path, &mut env)
}

fn compile_cmd(args: &[String]) -> i32 {
    let prefer = compile::CompilePrefer::from_args_and_env(args);
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str);
    let Some(path) = path else {
        eprintln!("Usage: kabootar compile <file.kab> [--self-host|--rust]");
        return 1;
    };
    match compile_file_report_with(path, prefer) {
        Ok((n, bytecode, backend)) => {
            if bytecode {
                println!("Compiled {path}: {n} statements (bytecode/{backend})");
            } else {
                println!("Compiled {path}: {n} statements (ast fallback/{backend})");
            }
            0
        }
        Err(e) => {
            eprintln!("Compile error: {e}");
            1
        }
    }
}

pub fn compile_file_report(path: &str) -> Result<(usize, bool), String> {
    let (n, bc, _) = compile_file_report_with(path, compile::CompilePrefer::SelfHostThenRust)?;
    Ok((n, bc))
}

pub fn compile_file_report_with(
    path: &str,
    prefer: compile::CompilePrefer,
) -> Result<(usize, bool, &'static str), String> {
    let (program, backend) = compile::compile_file_prefer(path, prefer)?;
    compile::write_compile_marker(path, &program)?;
    Ok((program.stmt_count, program.has_bytecode(), backend))
}

fn fmt_cmd(args: &[String]) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("Usage: kabootar fmt <file.kab>");
        return 1;
    };
    match fmt_file(path) {
        Ok(()) => {
            println!("Formatted {path}");
            0
        }
        Err(e) => {
            eprintln!("Fmt error: {e}");
            1
        }
    }
}

pub fn format_kabootar_source(source: &str) -> String {
    let mut out = String::new();
    let mut indent = 0usize;
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() {
            out.push('\n');
            continue;
        }
        if t.starts_with('}') {
            indent = indent.saturating_sub(1);
        }
        for _ in 0..indent {
            out.push_str("    ");
        }
        out.push_str(t);
        out.push('\n');
        if t.ends_with('{') {
            indent += 1;
        }
    }
    out
}

fn fmt_file(path: &str) -> Result<(), String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let formatted = format_kabootar_source(&raw);
    fs::write(path, formatted).map_err(|e| format!("write {path}: {e}"))
}

fn install_cmd(args: &[String]) -> i32 {
    let base = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    match install_packages_cmd(args, &base) {
        Ok(pkgs) => {
            if pkgs.is_empty() {
                println!("No packages to install");
            } else {
                for pkg in pkgs {
                    println!("Installed {}@{}", pkg.name, pkg.version);
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Install error: {e}");
            1
        }
    }
}

pub fn install_packages_cmd(
    args: &[String],
    base: &Path,
) -> Result<Vec<crate::registry::PackageInfo>, String> {
    if args.is_empty() {
        return crate::registry::install_manifest_deps(base);
    }
    let spec = &args[0];
    let (name, version) = crate::project::version::split_import_spec(spec);
    let constraint = version.as_deref().unwrap_or("0");
    let pkg = crate::registry::install_package(&name, constraint, base)?;
    Ok(vec![pkg])
}

fn publish_cmd(args: &[String]) -> i32 {
    let Some(target) = args.first() else {
        eprintln!("Usage: kabootar publish <file.kab|name>");
        return 1;
    };
    let base = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    let path = resolve_publish_path(target);
    match crate::registry::publish_file(&path, &base) {
        Ok(info) => {
            println!("Published {}@{} to local registry", info.name, info.version);
            0
        }
        Err(e) => {
            eprintln!("Publish error: {e}");
            1
        }
    }
}

fn resolve_publish_path(target: &str) -> PathBuf {
    if target.ends_with(".kab") || target.ends_with(".kabootar") {
        return PathBuf::from(target);
    }
    PathBuf::from(format!("lib/{target}.kab"))
}

struct WatchState {
    files: Vec<PathBuf>,
    snapshots: Vec<SystemTime>,
}

impl WatchState {
    fn new(files: Vec<PathBuf>) -> Self {
        let snapshots = files
            .iter()
            .filter_map(|p| fs::metadata(p).ok().and_then(|m| m.modified().ok()))
            .collect();
        Self { files, snapshots }
    }

    fn changed(&mut self) -> bool {
        let mut any = false;
        for (i, path) in self.files.iter().enumerate() {
            let mtime = fs::metadata(path).ok().and_then(|m| m.modified().ok());
            if mtime != self.snapshots.get(i).copied() {
                if let Some(t) = mtime {
                    if i < self.snapshots.len() {
                        self.snapshots[i] = t;
                    } else {
                        self.snapshots.push(t);
                    }
                }
                any = true;
            }
        }
        any
    }
}

fn collect_watch_files(entry: &str) -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from(entry)];
    if let Ok(manifest) = load_manifest_cwd() {
        for dep in manifest.dependencies.keys() {
            if let Some(p) = crate::modules::module_path(dep) {
                files.push(p);
            }
        }
    }
    if Path::new("lib").is_dir() {
        if let Ok(entries) = fs::read_dir("lib") {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("kab") {
                    files.push(p);
                }
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

fn reload_into_env(file: &str, env: &mut crate::value::Environment) -> Result<(), String> {
    compile::invalidate_file_cache(file);
    *env = create_global_env();
    compile::eval_file_cached(file, env)?;
    Ok(())
}

fn shell_cmd(_args: &[String]) -> i32 {
    match crate::shell::run_desktop() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn serve_cmd(args: &[String]) -> i32 {
    let mut port: Option<u16> = None;
    let mut bind = "0.0.0.0".to_string();
    let mut file: Option<String> = None;
    let mut watch = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = args.get(i).and_then(|s| s.parse().ok());
            }
            "--bind" => {
                i += 1;
                if let Some(b) = args.get(i) {
                    bind = b.clone();
                }
            }
            "--watch" => watch = true,
            arg if arg.ends_with(".kab") || arg.ends_with(".kabootar") => {
                file = Some(arg.to_string());
            }
            _ => {
                eprintln!("Unknown serve argument: {}", args[i]);
                return 1;
            }
        }
        i += 1;
    }

    let file = file.or_else(|| read_project_entry().ok());
    let Some(file) = file else {
        eprintln!("Usage: kabootar serve [--port N] [--bind ADDR] [--watch] <main.kab>");
        return 1;
    };

    let port = port
        .or_else(|| read_project_port().ok())
        .unwrap_or(8080);

    let mut env = create_global_env();
    if let Err(e) = reload_into_env(&file, &mut env) {
        eprintln!("Error loading {file}: {e}");
        return 1;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if watch {
            let watch_files = collect_watch_files(&file);
            let mut state = WatchState::new(watch_files);
            let entry = file.clone();
            let poll = |env: &mut crate::value::Environment| {
                if state.changed() {
                    reload_into_env(&entry, env).is_ok()
                } else {
                    false
                }
            };
            if let Err(e) =
                crate::runtime::http::http_serve_loop_with_poll(port, &bind, &mut env, Some(poll))
            {
                eprintln!("Server error: {e}");
                return 1;
            }
        } else if let Err(e) = crate::runtime::http::http_serve_loop(port, &bind, &mut env) {
            eprintln!("Server error: {e}");
            return 1;
        }
        0
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (port, bind, watch);
        eprintln!("kabootar serve is not available on wasm32");
        1
    }
}

fn mod_cmd(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("init") => mod_init(&args[1..]),
        Some("run") => mod_run(),
        _ => {
            eprintln!("Usage: kabootar mod init <web|api> | kabootar mod run");
            1
        }
    }
}

fn mod_init(args: &[String]) -> i32 {
    let template = args.first().map(String::as_str).unwrap_or("web");
    match templates::write_project(template, Path::new(".")) {
        Ok(()) => {
            println!("Created Kabootar {template} project in current directory.");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn mod_run() -> i32 {
    let entry = match read_project_entry() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    match run_file(&entry) {
        Ok(v) => {
            println!("=> {:?}", v);
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn read_project_entry() -> Result<String, String> {
    let manifest = load_manifest_cwd()?;
    Ok(manifest.entry.unwrap_or_else(|| "main.kab".to_string()))
}

fn read_project_port() -> Result<u16, String> {
    let manifest = load_manifest_cwd()?;
    manifest
        .port
        .ok_or_else(|| "No port in kabootar.toml".to_string())
}

pub mod templates {
    use super::*;
    use std::path::Path;

    pub fn write_project(template: &str, dir: &Path) -> Result<(), String> {
        let (toml, main_kab, extra): (&str, &str, Option<(&str, &str)>) = match template {
            "web" => (
                TEMPLATE_TOML_WEB,
                TEMPLATE_MAIN_WEB,
                Some(("index.html", TEMPLATE_INDEX_HTML)),
            ),
            "api" => (TEMPLATE_TOML_API, TEMPLATE_MAIN_API, None),
            _ => return Err(format!("Unknown template \"{template}\". Use web or api.")),
        };

        write_if_missing(&dir.join("kabootar.toml"), toml)?;
        write_if_missing(&dir.join("main.kab"), main_kab)?;
        fs::create_dir_all(dir.join("lib"))
            .map_err(|e| format!("Failed to create lib/: {e}"))?;

        if let Some((name, content)) = extra {
            write_if_missing(&dir.join(name), content)?;
        }
        Ok(())
    }

    fn write_if_missing(path: &Path, content: &str) -> Result<(), String> {
        if path.exists() {
            return Ok(());
        }
        fs::write(path, content).map_err(|e| format!("Failed to write {}: {e}", path.display()))
    }

    const TEMPLATE_TOML_WEB: &str = r#"version = "0.1.0"
template = "web"
entry = "main.kab"
port = 8080

[dependencies]
"#;

    const TEMPLATE_TOML_API: &str = r#"version = "0.1.0"
template = "api"
entry = "main.kab"
port = 8080

[dependencies]
"#;

    const TEMPLATE_MAIN_WEB: &str = r#"@version "0.1.0"
import "http"

http_route("GET", "/", home)

pub fn home() {
    return http_response(200, "Kabootar web — edit main.kab and run: kabootar serve --watch main.kab")
}
"#;

    const TEMPLATE_MAIN_API: &str = r#"@version "0.1.0"
import "http"

sql("CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, name TEXT)")

http_route("GET", "/api/items", list_items)
http_route("POST", "/api/items", create_item)

pub fn list_items() {
    let rows = sql("SELECT id, name FROM items")
    return http_response(200, rows)
}

pub fn create_item() {
    sql("INSERT INTO items (name) VALUES ($1)", req_body)
    return http_response(201, "created")
}
"#;

    const TEMPLATE_INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="sv">
<head>
  <meta charset="UTF-8" />
  <title>Kabootar Web</title>
</head>
<body>
  <h1>Kabootar</h1>
  <p>Backend körs med <code>kabootar serve --watch main.kab</code></p>
</body>
</html>
"#;
}
