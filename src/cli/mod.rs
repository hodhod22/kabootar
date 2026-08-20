//! Kabootar CLI — `mod init`, `mod run`, `serve`, `run`, `compile`, REPL, notebook.

mod doc;
mod registry_web;
mod repl;
mod test_runner;

pub use doc::{extract_kab_docs, DocItem};
pub use registry_web::render_index as registry_render_index;

use crate::compile::{self};
use crate::evaluator::create_global_env;
use crate::project::manifest::load_manifest_cwd;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn format_user_error(e: &str) -> String {
    crate::runtime::stdlib::error::format_runtime_error(e)
}

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        return repl::run_repl();
    }
    match args[0].as_str() {
        "run" => run_file_cmd(&args[1..]),
        "serve" => serve_cmd(&args[1..]),
        "shell" => shell_cmd(&args[1..]),
        "notebook" | "nb" => notebook_cmd(&args[1..]),
        "compile" => compile_cmd(&args[1..]),
        "fmt" => fmt_cmd(&args[1..]),
        "doc" => doc_cmd(&args[1..]),
        "test" => test_cmd(&args[1..]),
        "repl" => repl::run_repl(),
        "registry" => registry_cmd(&args[1..]),
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
        path if path.ends_with(".knb") => notebook_run_file(path, false),
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
  kabootar                         Interactive exploration REPL
  kabootar repl                    Same as bare kabootar (explicit REPL)
  kabootar run <file.kab>          Run a Kabootar script
  kabootar notebook run <file.knb> [--science]   Run notebook cells
  kabootar compile <file.kab> [--self-host|--rust]   Compile via self-host (default; Rust fallback)
  kabootar fmt [--check] <file.kab>   Format Kabootar source (basic)
  kabootar doc [path] [--out FILE] Extract /// docs to Markdown
  kabootar test [path] [--coverage]  Run *_test.kab (default: tests)
  kabootar registry web [--port N] Browse local package registry in browser
  kabootar install [name@ver]      Install deps from local registry
  kabootar publish <file.kab>      Publish module to local registry
  kabootar serve [opts] <file>     Start HTTP server
  kabootar shell                   Open Kabootar OS desktop window
  kabootar mod init <web|api|game|game3d|science-ai>
  kabootar mod run                 Run project entry (kabootar.toml)
  kabootar <file.kab|.knb>         Shorthand for run / notebook run

Kab modules: import \"cli\" | \"log\" | \"validate\" | \"auth\" | \"test\" | \"test/mock\"

Serve options:
  --port <n>       Port (default 8080 or kabootar.toml)
  --bind <addr>    Bind address (default 0.0.0.0)
  --watch          Hot reload on file changes

Examples:
  kabootar
  kabootar repl
  kabootar doc lib/data --out docs/api-data.md
  kabootar test tests --coverage
  kabootar registry web --port 8787
  kabootar notebook run examples/explore_smoke.knb --science
  kabootar mod init science-ai
  kabootar serve --watch main.kab
"
    );
}

fn notebook_cmd(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("run") => {
            let mut science = false;
            let mut path: Option<&str> = None;
            for a in &args[1..] {
                if a == "--science" || a == "-s" {
                    science = true;
                } else if !a.starts_with('-') {
                    path = Some(a.as_str());
                }
            }
            match path {
                Some(p) => notebook_run_file(p, science),
                None => {
                    eprintln!("Usage: kabootar notebook run <file.knb> [--science]");
                    1
                }
            }
        }
        Some("--help") | Some("-h") | None => {
            eprintln!("Usage: kabootar notebook run <file.knb> [--science]");
            0
        }
        Some(other) if other.ends_with(".knb") => {
            let science = args.iter().any(|a| a == "--science" || a == "-s");
            notebook_run_file(other, science)
        }
        Some(other) => {
            eprintln!("Unknown notebook subcommand: {other}");
            eprintln!("Usage: kabootar notebook run <file.knb> [--science]");
            1
        }
    }
}

fn notebook_run_file(path: &str, preload_science: bool) -> i32 {
    match crate::notebook::run_notebook_file(Path::new(path), preload_science) {
        Ok(v) => {
            println!("=> {}", crate::value::format_value(&v));
            0
        }
        Err(e) => {
            eprintln!("{}", format_user_error(&e));
            1
        }
    }
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
        eprintln!("Usage: kabootar compile <file.kab> [--self-host|--rust|--native]");
        return 1;
    };
    if let Err(e) = compile::refuse_app_rust_compile(path, prefer) {
        eprintln!("Compile error: {e}");
        return 1;
    }
    match compile_file_report_with(path, prefer) {
        Ok((n, bytecode, backend)) => {
            if args.iter().any(|a| a == "--native") {
                let marker = format!("{path}.kbn");
                let _ = std::fs::write(
                    &marker,
                    format!("kabootar-native/1\nsource={path}\nkernel=native_add_loop\n"),
                );
                println!("Compiled {path}: {n} statements (native-stub/{backend} → {marker})");
            } else if bytecode {
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
    let prefer = compile::CompilePrefer::from_args_and_env(&[]);
    compile::refuse_app_rust_compile(path, prefer)?;
    let (n, bc, _) = compile_file_report_with(path, prefer)?;
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
    let check = args.iter().any(|a| a == "--check");
    let path = args.iter().find(|a| !a.starts_with('-')).map(String::as_str);
    let Some(path) = path else {
        eprintln!("Usage: kabootar fmt [--check] <file.kab>");
        return 1;
    };
    match fmt_file(path, check) {
        Ok(FmtOutcome::Wrote) => {
            println!("Formatted {path}");
            0
        }
        Ok(FmtOutcome::CheckedOk) => {
            println!("Already formatted: {path}");
            0
        }
        Ok(FmtOutcome::CheckFailed) => {
            eprintln!("Needs formatting: {path}");
            1
        }
        Err(e) => {
            eprintln!("Fmt error: {e}");
            1
        }
    }
}

enum FmtOutcome {
    Wrote,
    CheckedOk,
    CheckFailed,
}

pub fn format_kabootar_source(source: &str) -> String {
    let mut out = String::new();
    let mut indent = 0usize;
    let mut in_block_comment = false;
    for line in source.lines() {
        let raw = line;
        let t = raw.trim();
        if t.is_empty() {
            out.push('\n');
            continue;
        }
        // Preserve full-line comments as-is (trimmed indent only).
        if !in_block_comment && (t.starts_with("//") || t.starts_with("///")) {
            for _ in 0..indent {
                out.push_str("    ");
            }
            out.push_str(t);
            out.push('\n');
            continue;
        }
        if t.contains("/*") {
            in_block_comment = true;
        }
        if in_block_comment {
            for _ in 0..indent {
                out.push_str("    ");
            }
            out.push_str(t);
            out.push('\n');
            if t.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }
        if t.starts_with('}') || t.starts_with("} else") || t.starts_with("} else if") {
            indent = indent.saturating_sub(1);
        }
        for _ in 0..indent {
            out.push_str("    ");
        }
        // Collapse internal runs of spaces outside strings (light polish).
        out.push_str(&collapse_spaces_outside_strings(t));
        out.push('\n');
        if t.ends_with('{') {
            indent += 1;
        }
    }
    out
}

fn collapse_spaces_outside_strings(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut in_str = false;
    let mut quote = '"';
    let mut prev_space = false;
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == quote {
                in_str = false;
            }
            prev_space = false;
            continue;
        }
        if c == '"' || c == '\'' {
            in_str = true;
            quote = c;
            out.push(c);
            prev_space = false;
            continue;
        }
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

fn fmt_file(path: &str, check: bool) -> Result<FmtOutcome, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let formatted = format_kabootar_source(&raw);
    if check {
        if normalize_newlines(&raw) == normalize_newlines(&formatted) {
            Ok(FmtOutcome::CheckedOk)
        } else {
            Ok(FmtOutcome::CheckFailed)
        }
    } else {
        fs::write(path, formatted).map_err(|e| format!("write {path}: {e}"))?;
        Ok(FmtOutcome::Wrote)
    }
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn doc_cmd(args: &[String]) -> i32 {
    let mut out_path: Option<String> = None;
    let mut path = "lib".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    out_path = Some(p.clone());
                }
            }
            a if !a.starts_with('-') => path = a.to_string(),
            other => {
                eprintln!("Unknown doc argument: {other}");
                eprintln!("Usage: kabootar doc [path] [--out FILE]");
                return 1;
            }
        }
        i += 1;
    }
    match doc::generate_docs(Path::new(&path)) {
        Ok((n, md)) => {
            if let Some(out) = out_path {
                if let Some(parent) = Path::new(&out).parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = fs::create_dir_all(parent);
                    }
                }
                if let Err(e) = fs::write(&out, &md) {
                    eprintln!("doc write error: {e}");
                    return 1;
                }
                println!("Wrote {n} doc items → {out}");
            } else {
                print!("{md}");
                eprintln!("({n} doc items)");
            }
            0
        }
        Err(e) => {
            eprintln!("doc error: {e}");
            1
        }
    }
}

fn test_cmd(args: &[String]) -> i32 {
    let coverage = args.iter().any(|a| a == "--coverage");
    let path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("tests");
    let root = Path::new(path);
    let tests = match test_runner::discover_tests(root) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("test error: {e}");
            return 1;
        }
    };
    if tests.is_empty() {
        eprintln!("No *_test.kab files under {path}");
        return 1;
    }
    let (pass, fail, results) = test_runner::run_tests(&tests);
    for r in &results {
        if r.ok {
            println!("ok {}", r.path);
        } else {
            println!("FAIL {} — {}", r.path, r.message);
        }
    }
    println!("{pass} passed, {fail} failed");
    if coverage {
        let cov_roots = if root.is_dir() && path == "tests" {
            vec![PathBuf::from("lib")]
        } else {
            vec![PathBuf::from("lib")]
        };
        match test_runner::coverage_for(&cov_roots, &tests) {
            Ok(rep) => print!("{}", test_runner::format_coverage(&rep)),
            Err(e) => eprintln!("coverage error: {e}"),
        }
    }
    if fail > 0 {
        1
    } else {
        0
    }
}

fn registry_cmd(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("web") => registry_web_cmd(&args[1..]),
        Some("list") => registry_list_cmd(),
        Some("--help") | Some("-h") | None => {
            eprintln!("Usage: kabootar registry web [--port N] [--bind ADDR]");
            eprintln!("       kabootar registry list");
            0
        }
        Some(other) => {
            eprintln!("Unknown registry subcommand: {other}");
            1
        }
    }
}

fn registry_list_cmd() -> i32 {
    let base = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    match crate::registry::list_registry(&base) {
        Ok(pkgs) => {
            if pkgs.is_empty() {
                println!("(empty registry)");
            } else {
                for p in pkgs {
                    println!("{}@{}", p.name, p.version);
                }
            }
            0
        }
        Err(e) => {
            eprintln!("registry list error: {e}");
            1
        }
    }
}

fn registry_web_cmd(args: &[String]) -> i32 {
    let mut port: u16 = 8787;
    let mut bind = "127.0.0.1".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = args
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(port);
            }
            "--bind" => {
                i += 1;
                if let Some(b) = args.get(i) {
                    bind = b.clone();
                }
            }
            other => {
                eprintln!("Unknown argument: {other}");
                return 1;
            }
        }
        i += 1;
    }
    let base = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Err(e) = registry_web::serve_registry_web(&base, &bind, port) {
            eprintln!("registry web error: {e}");
            return 1;
        }
        0
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (base, bind, port);
        eprintln!("registry web not available on wasm32");
        1
    }
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
            eprintln!("Usage: kabootar mod init <web|api|game|game3d|science-ai> | kabootar mod run");
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
        let (toml, main_kab, extras): (&str, &str, &[(&str, &str)]) = match template {
            "web" => (
                TEMPLATE_TOML_WEB,
                TEMPLATE_MAIN_WEB,
                &[("index.html", TEMPLATE_INDEX_HTML)],
            ),
            "api" => (TEMPLATE_TOML_API, TEMPLATE_MAIN_API, &[]),
            "game" => (TEMPLATE_TOML_GAME, TEMPLATE_MAIN_GAME, &[]),
            "game3d" => (
                TEMPLATE_TOML_GAME3D,
                TEMPLATE_MAIN_GAME3D,
                &[("shaders/solid.wgsl", TEMPLATE_SOLID_WGSL)],
            ),
            "science-ai" => (
                TEMPLATE_TOML_SCIENCE_AI,
                TEMPLATE_MAIN_SCIENCE_AI,
                &[],
            ),
            _ => {
                return Err(format!(
                    "Unknown template \"{template}\". Use web, api, game, game3d, or science-ai."
                ))
            }
        };

        write_if_missing(&dir.join("kabootar.toml"), toml)?;
        write_if_missing(&dir.join("main.kab"), main_kab)?;
        fs::create_dir_all(dir.join("lib"))
            .map_err(|e| format!("Failed to create lib/: {e}"))?;

        for (name, content) in extras {
            let path = dir.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
            }
            write_if_missing(&path, content)?;
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

    const TEMPLATE_TOML_GAME: &str = r#"version = "0.1.0"
template = "game"
entry = "main.kab"

[dependencies]
"#;

    const TEMPLATE_TOML_GAME3D: &str = r#"version = "0.1.0"
template = "game3d"
entry = "main.kab"

[dependencies]
"#;

    const TEMPLATE_MAIN_GAME: &str = r##"@version "0.1.0"
import "game/input"
import "game/time"
import "game/physics"

platform_use("kabootar")
let surf = game_surface_create(320, 240)
let ctx = surf["ctx"]
let player = { x: 40.0, y: 100.0, w: 24.0, h: 24.0 }
let wall = { x: 200.0, y: 80.0, w: 40.0, h: 80.0 }
let actions = createActions({ left: ["ArrowLeft", "KeyA"], right: ["ArrowRight", "KeyD"] })
let clock = createFixed(1.0 / 60.0)

fn onFixed(dt) {
    let dx = 0.0
    if actionPressed(actions, "left") { dx = dx - 120.0 * dt }
    if actionPressed(actions, "right") { dx = dx + 120.0 * dt }
    player["x"] = player["x"] + dx
    if aabbOverlap(player, wall) {
        player = resolveAabb(player, wall)
    }
}

fn game_loop(dtMs) {
    fixedTick(clock, dtSec(dtMs), onFixed)
    ctx.fillStyle = "#101820"
    ctx.fillRect(0, 0, 320, 240)
    ctx.fillStyle = "#44cc88"
    ctx.fillRect(player["x"], player["y"], player["w"], player["h"])
    ctx.fillStyle = "#cc5544"
    ctx.fillRect(wall["x"], wall["y"], wall["w"], wall["h"])
    surf.present()
    requestAnimationFrame(game_loop)
}

requestAnimationFrame(game_loop)
"##;

    const TEMPLATE_MAIN_GAME3D: &str = r#"@version "0.1.0"
import "game/render"
import "game/shader"

platform_use("kabootar")
let surf = game_surface_create_3d(320, 240)
let gl = surf["gl"]
loadSolidFromFile("shaders/solid.wgsl")
gl.lookAt(0, 0, 3, 0, 0, 0, 0, 1, 0)
setColor(gl, 0.2, 0.7, 1.0, 1.0)
let mesh = createMesh(gl, [
    -0.5, -0.5, 0.5,
     0.5, -0.5, 0.5,
     0.0,  0.5, 0.5
])
drawMesh(mesh)
surf.present()
"game3d-ok"
"#;

    const TEMPLATE_SOLID_WGSL: &str = r#"struct FrameUniforms {
    view_proj: mat4x4<f32>,
}

struct MaterialUniforms {
    model: mat4x4<f32>,
    color: vec4<f32>,
    uv_xform: vec4<f32>,
}

@group(0) @binding(0) var<uniform> frame: FrameUniforms;
@group(1) @binding(0) var<uniform> mat: MaterialUniforms;

struct VertexIn {
    @location(0) position: vec3<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = frame.view_proj * mat.model * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return mat.color;
}
"#;

    const TEMPLATE_TOML_SCIENCE_AI: &str = r#"version = "0.1.0"
template = "science-ai"
entry = "main.kab"

[dependencies]
"#;

    const TEMPLATE_MAIN_SCIENCE_AI: &str = r#"@version "0.1.0"
import "science"
import "science/nd"
import "science/ml"

// Train y ≈ 2x + 1 with SGD (SC2 subset).
let params = [0.0, 0.0]
let i = 0
while i < 200 {
    params = linregStep(params, [2.0], 5.0, 0.05)
    i = i + 1
}
let a = nd_from([[1.0, 2.0], [3.0, 4.0]])
let b = nd_from([[1.0, 0.0], [0.0, 1.0]])
let c = matmul(a, b)
nd_get(c, [0, 1]) == 2.0 && params[0] > 1.5
"#;
}
