//! Built-in Kabootar modules (`import "name"`).

use crate::evaluator::{create_global_env, eval_source};
use crate::runtime::{codai_register, docai_register, http_module, kv8_register, science_register};
use crate::value::{Environment, PromiseValue, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

#[derive(Clone)]
struct CachedModuleExports {
    source_mtime: SystemTime,
    exported_names: Vec<String>,
    bindings: HashMap<String, Value>,
}

thread_local! {
    static MODULE_EXPORT_CACHE: RefCell<HashMap<String, CachedModuleExports>> =
        RefCell::new(HashMap::new());
    /// Active import nesting depth (root import = 1).
    static IMPORT_DEPTH: RefCell<usize> = RefCell::new(0);
}

fn import_depth_limit() -> Option<usize> {
    std::env::var("KABOOTAR_IMPORT_MAX")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n > 0)
}

fn import_warn_depth() -> usize {
    std::env::var("KABOOTAR_IMPORT_WARN")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(48)
}

/// Drop cached exports for a module (e.g. after editing `lib/kv8/eval.kab` in tests).
pub fn invalidate_module_export_cache(module_path: &str) {
    let key = module_path.replace('\\', "/");
    MODULE_EXPORT_CACHE.with(|cache| {
        cache.borrow_mut().remove(&key);
    });
}

fn try_restore_cached_module(
    cache_key: &str,
    source_mtime: SystemTime,
    importer: &mut Environment,
) -> Option<Vec<String>> {
    MODULE_EXPORT_CACHE.with(|cache| {
        let cache = cache.borrow();
        let cached = cache.get(cache_key)?;
        if source_mtime > cached.source_mtime {
            return None;
        }
        for name in &cached.exported_names {
            if let Some(v) = cached.bindings.get(name) {
                importer.set(name.clone(), v.clone());
            }
        }
        Some(cached.exported_names.clone())
    })
}

fn store_cached_module(
    cache_key: &str,
    source_mtime: SystemTime,
    module_env: &Environment,
    exported: &[String],
) {
    let mut bindings = HashMap::new();
    for name in exported {
        if let Some(v) = module_env.get(name) {
            bindings.insert(name.clone(), v.clone());
        }
    }
    MODULE_EXPORT_CACHE.with(|cache| {
        cache.borrow_mut().insert(
            cache_key.to_string(),
            CachedModuleExports {
                source_mtime,
                exported_names: exported.to_vec(),
                bindings,
            },
        );
    });
}

#[derive(Debug, Clone)]
pub struct ImportMeta {
    pub url: String,
    pub path: String,
}

thread_local! {
    static CURRENT_IMPORT_META: RefCell<Option<ImportMeta>> = RefCell::new(None);
}

pub fn with_import_meta<F, T>(meta: ImportMeta, f: F) -> T
where
    F: FnOnce() -> T,
{
    CURRENT_IMPORT_META.with(|slot| {
        let prev = slot.borrow().clone();
        *slot.borrow_mut() = Some(meta);
        let out = f();
        *slot.borrow_mut() = prev;
        out
    })
}

pub fn current_import_meta() -> ImportMeta {
    CURRENT_IMPORT_META.with(|slot| {
        slot.borrow().clone().unwrap_or_else(|| ImportMeta {
            url: "kabootar:///main".into(),
            path: "main".into(),
        })
    })
}

pub fn import_meta_object() -> Value {
    let meta = current_import_meta();
    let mut map = HashMap::new();
    map.insert("url".into(), Value::String(meta.url));
    map.insert("path".into(), Value::String(meta.path));
    Value::from_object(map)
}

pub fn load_module_namespace(spec: &str, _env: &Environment) -> Result<Value, String> {
    let mut module_env = create_global_env();
    let mut loaded = HashSet::new();
    let (module_name, requested_version) = crate::project::version::split_import_spec(spec);
    let exported = import_module_inner(
        &module_name,
        requested_version.as_deref(),
        &mut module_env,
        &mut loaded,
    )?;
    let mut ns = HashMap::new();
    for name in exported {
        if let Some(v) = module_env.get(&name) {
            ns.insert(name, v);
        }
    }
    Ok(Value::from_object(ns))
}

pub fn dynamic_import(spec: &Value, env: &mut Environment) -> Result<Value, String> {
    let name = match spec {
        Value::String(s) => s.as_str(),
        _ => return Err("import() specifier must be a string".into()),
    };
    let ns = load_module_namespace(name, env)?;
    Ok(Value::Promise(Rc::new(RefCell::new(PromiseValue::Resolved(ns)))))
}

fn import_meta_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(import_meta_object())
}

fn dynamic_import_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let spec = args.first().ok_or("dynamic_import(spec)")?;
    dynamic_import(spec, env)
}

pub fn register_import_builtins(env: &mut Environment) {
    env.set("import_meta".to_string(), Value::NativeFunction(import_meta_native));
    env.set(
        "dynamic_import".to_string(),
        Value::NativeFunction(dynamic_import_native),
    );
}

fn export_module_bindings(module_env: &Environment, importer: &mut Environment) -> Vec<String> {
    use crate::value::Value;
    let mut imported = Vec::new();
    for name in module_env.exported_names() {
        if let Some(val) = module_env.get(&name) {
            let exported = if let Value::BytecodeFn(func) = val {
                Value::BytecodeFn(crate::bytecode::prepare_exported_bytecode_fn(
                    &name,
                    func,
                    module_env,
                ))
            } else {
                val
            };
            importer.set(name.clone(), exported);
            imported.push(name);
        }
    }
    imported
}

fn dependency_version(module_name: &str) -> Option<String> {
    crate::project::manifest::load_manifest_cwd()
        .ok()
        .and_then(|m| m.dependencies.get(module_name).cloned())
}

fn check_module_version(
    module_name: &str,
    module_version: Option<&str>,
    requested: Option<&str>,
) -> Result<(), String> {
    let constraint = requested
        .map(String::from)
        .or_else(|| dependency_version(module_name));
    let Some(constraint) = constraint else {
        return Ok(());
    };
    let Some(found) = module_version else {
        return Err(format!(
            "Module \"{}\" requires @version (needed: {})",
            module_name, constraint
        ));
    };
    if !crate::project::manifest::version_matches(found, &constraint) {
        return Err(format!(
            "Module \"{}\" version {} does not match required {}",
            module_name, found, constraint
        ));
    }
    Ok(())
}

fn eval_file_module(source: &str, path: &std::path::Path, importer: &mut Environment) -> Result<Vec<String>, String> {
    let cache_key = path
        .to_string_lossy()
        .replace('\\', "/");
    let source_mtime = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok());
    if let Some(mtime) = source_mtime {
        if let Some(imported) = try_restore_cached_module(&cache_key, mtime, importer) {
            return Ok(imported);
        }
    }

    let mut module_env = create_global_env();
    let program = crate::compile::load_program_for_file(&cache_key, source)?;
    let path_str = path.to_string_lossy().replace('\\', "/");
    let url = format!("file://{path_str}");
    with_import_meta(
        ImportMeta {
            url,
            path: path_str,
        },
        || crate::compile::eval_program(&program, &mut module_env),
    )?;
    let imported = export_module_bindings(&module_env, importer);
    if let Some(mtime) = source_mtime {
        store_cached_module(&cache_key, mtime, &module_env, &imported);
    }
    Ok(imported)
}

fn import_registry_spec(
    name: &str,
    requested_version: Option<&str>,
    importer: &mut Environment,
) -> Result<Vec<String>, String> {
    let full = if let Some(v) = requested_version {
        format!("{name}@{v}")
    } else {
        name.to_string()
    };
    let path = crate::runtime::npm_ts::ensure_registry_module(&full, None)?;
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read module {}: {}", path.display(), e))?;
    let source = match path.extension().and_then(|e| e.to_str()) {
        Some("ts") => crate::runtime::npm_ts::ts_strip_types(&raw),
        _ => raw,
    };
    eval_file_module(&source, &path, importer)
}

fn module_not_found_message(name: &str) -> String {
    let builtins: Vec<String> = list_builtins().into_iter().map(String::from).collect();
    let lower = name.to_lowercase();
    let hint = builtins
        .iter()
        .find(|b| b.to_lowercase() == lower || b.to_lowercase().contains(&lower))
        .map(|b| format!(" Did you mean built-in `import \"{}\"`?", b))
        .unwrap_or_default();
    format!(
        "Module not found: \"{}\".{} Look for {}.kab or lib/{}.kab on disk.",
        name, hint, name.replace('/', "/"), name.replace('/', "/")
    )
}

fn resolve_module_path(name: &str, requested_version: Option<&str>) -> Option<PathBuf> {
    let rel = name.replace('/', std::path::MAIN_SEPARATOR_STR);
    let mut candidates = vec![
        format!("{rel}.kab"),
        format!("{rel}.kabootar"),
        format!("lib/{rel}.kab"),
        format!("lib/{rel}.kabootar"),
    ];
    if let Ok(cwd) = std::env::current_dir() {
        let constraint = requested_version
            .map(String::from)
            .or_else(|| dependency_version(name));
        if let Some(installed) =
            crate::registry::resolve_installed_path(name, constraint.as_deref(), &cwd)
        {
            candidates.push(installed.to_string_lossy().to_string());
        }
    }
    if let Ok(extra) = std::env::var("KABOOTAR_PATH") {
        for base in extra.split(';').chain(extra.split(':')) {
            let base = base.trim();
            if base.is_empty() {
                continue;
            }
            candidates.push(format!("{base}/{rel}.kab"));
            candidates.push(format!("{base}/{rel}.kabootar"));
        }
    }
    candidates
        .into_iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

pub fn import_module(name: &str, env: &mut Environment) -> Result<(), String> {
    let _ = import_module_exported(name, env)?;
    Ok(())
}

pub fn import_module_exported(name: &str, env: &mut Environment) -> Result<Vec<String>, String> {
    let mut loaded = HashSet::new();
    let (module_name, requested_version) = crate::project::version::split_import_spec(name);
    import_module_inner(
        &module_name,
        requested_version.as_deref(),
        env,
        &mut loaded,
    )
}

fn import_module_inner(
    name: &str,
    requested_version: Option<&str>,
    env: &mut Environment,
    loaded: &mut HashSet<String>,
) -> Result<Vec<String>, String> {
    if !loaded.insert(name.to_string()) {
        return Err(format!("Circular import detected: \"{}\"", name));
    }

    let depth = IMPORT_DEPTH.with(|d| {
        *d.borrow_mut() += 1;
        *d.borrow()
    });
    struct DepthGuard;
    impl Drop for DepthGuard {
        fn drop(&mut self) {
            IMPORT_DEPTH.with(|d| {
                let mut n = d.borrow_mut();
                *n = n.saturating_sub(1);
            });
        }
    }
    let _depth_guard = DepthGuard;

    if let Some(max) = import_depth_limit() {
        if depth > max {
            return Err(format!(
                "Import depth {} exceeds KABOOTAR_IMPORT_MAX={} while loading \"{}\". Prefer leaf imports (see docs/GAME.md).",
                depth, max, name
            ));
        }
    } else if depth == import_warn_depth() {
        eprintln!(
            "kabootar: import depth {depth} while loading \"{name}\" (set KABOOTAR_IMPORT_MAX to hard-fail; prefer leaf imports)"
        );
    }

    if name == "science" {
        science_register(env);
        return Ok(Vec::new());
    }
    if name == "docai" {
        docai_register(env);
        return Ok(Vec::new());
    }
    if name == "codai" {
        codai_register(env);
        return Ok(Vec::new());
    }
    if name == "kv8" {
        kv8_register(env);
        return Ok(Vec::new());
    }
    if name == "http" {
        http_module::register(env);
        return Ok(Vec::new());
    }

    // Self-host tests preload Rust-compiled bytecode; re-importing source OOMs on Windows.
    if name == "self_host/emit" && env.get("emit").is_some() {
        return Ok(vec!["emit".to_string()]);
    }
    if name == "self_host/lexer" && env.get("tokenize").is_some() {
        return Ok(vec![
            "tokenize".to_string(),
            "token_type_name".to_string(),
            "token_value".to_string(),
        ]);
    }
    if name == "self_host/parser" && env.get("parseTokens").is_some() {
        return Ok(vec!["parseTokens".to_string()]);
    }

    if name.starts_with("npm:") || name.starts_with("jsr:") {
        return import_registry_spec(name, requested_version, env);
    }

    if name.starts_with("node:") {
        return crate::runtime::node_compat::import_node_module(name, env);
    }

    if let Some(source) = builtin_source(name) {
        if source.is_empty() {
            return Ok(Vec::new());
        }
        let mut module_env = create_global_env();
        with_import_meta(
            ImportMeta {
                url: format!("kabootar:///{name}"),
                path: name.to_string(),
            },
            || eval_source(source, &mut module_env),
        )?;
        return Ok(export_module_bindings(&module_env, env));
    }

    let path = resolve_module_path(name, requested_version).ok_or_else(|| module_not_found_message(name))?;
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read module {}: {}", path.display(), e))?;
    let (module_version, source) = crate::project::version::strip_version_directive(&raw);
    check_module_version(
        name,
        module_version.as_deref(),
        requested_version,
    )?;
    eval_file_module(&source, &path, env)
}

pub fn module_path(name: &str) -> Option<PathBuf> {
    resolve_module_path(name, None)
}

static BUILTIN_STD: &str = r#"
// Aggregator: also pulls lib/std/* helpers (G3).
pub import "std/array"
pub import "std/object"
pub import "std/math"
pub import "std/string"

pub fn parse(text) {
    return json_parse(text)
}
pub fn stringify(v) {
    return json_stringify(v)
}
pub fn info() {
    return std_info()
}
pub fn array_sum(arr) {
    return reduce(arr, (a, b) => a + b, 0)
}
"#;

static BUILTIN_JSON: &str = r#"
pub fn parse(text) {
    return json_parse(text)
}
pub fn dump(v) {
    return json_stringify(v)
}
"#;

static BUILTIN_COLLECTIONS: &str = r#"
pub fn map_new_empty() {
    return map_new()
}
pub fn set_new_empty() {
    return set_new()
}
pub fn from_pairs(pairs) {
    let m = map_new()
    for row in pairs {
        map_set(m, row[0], row[1])
    }
    return m
}
"#;

static BUILTIN_STRINGS: &str = r#"
pub fn clean(s) {
    return trim(s)
}
pub fn parts(s, sep) {
    return split(s, sep)
}
pub fn has_prefix(s, p) {
    return starts_with(s, p)
}
"#;

static BUILTIN_MATH: &str = r#"
pub fn add(a, b) {
    return a + b
}
pub fn mul(a, b) {
    return a * b
}
pub fn add_pub(a, b) {
    return a + b
}
"#;

/// Stub source for LSP / go-to-definition only — natives registered in `http_module::register`.
static BUILTIN_HTTP: &str = r#"
fn ok(body) { return null }
fn created(body) { return null }
fn no_content() { return null }
fn not_found() { return null }
fn method_not_allowed() { return null }
fn method_get() { return null }
fn method_post() { return null }
fn method_put() { return null }
fn method_patch() { return null }
fn method_delete() { return null }
fn method_head() { return null }
fn method_options() { return null }
fn route_get(path, handler) { return null }
fn route_post(path, handler) { return null }
fn route_put(path, handler) { return null }
fn route_patch(path, handler) { return null }
fn route_delete(path, handler) { return null }
fn route_head(path, handler) { return null }
fn route_options(path, handler) { return null }
fn request_get(path) { return null }
fn request_post(path, body) { return null }
fn request_put(path, body) { return null }
fn request_patch(path, body) { return null }
fn request_delete(path) { return null }
fn request_head(path) { return null }
fn request_options(path) { return null }
fn request_get_async(path) { return null }
fn request_post_async(path, body) { return null }
fn request_put_async(path, body) { return null }
fn request_patch_async(path, body) { return null }
fn request_delete_async(path) { return null }
fn request_head_async(path) { return null }
fn request_options_async(path) { return null }
fn fetch_get(url) { return null }
fn fetch_post(url, body) { return null }
fn fetch_put(url, body) { return null }
fn fetch_patch(url, body) { return null }
fn fetch_delete(url) { return null }
fn fetch_head(url) { return null }
fn fetch_options(url) { return null }
fn fetch_get_headers(url, headers) { return null }
fn fetch_post_headers(url, body, headers) { return null }
fn fetch_put_headers(url, body, headers) { return null }
fn fetch_patch_headers(url, body, headers) { return null }
fn fetch_delete_headers(url, headers) { return null }
"#;

static BUILTIN_CRYPTO: &str = r#"
fn sha256(data) {
    return crypto_sha3_256(data)
}
fn secure(data) {
    return crypto_secure(data)
}
"#;

/// Stub source for LSP / go-to-definition only — not evaluated at runtime.
static BUILTIN_SCIENCE: &str = r#"
fn cplx(re, im) { return null }
fn c_add(a, b) { return null }
fn c_sub(a, b) { return null }
fn c_mul(a, b) { return null }
fn c_div(a, b) { return null }
fn c_conj(z) { return null }
fn c_abs(z) { return null }
fn c_arg(z) { return null }
fn c_exp(z) { return null }
fn c_sqrt(z) { return null }
fn c_polar(r, theta) { return null }
fn sqrt(x) { return null }
fn pow(x, y) { return null }
fn fact(n) { return null }
fn gcd(a, b) { return null }
fn lcm(a, b) { return null }
fn sin(x) { return null }
fn cos(x) { return null }
fn tan(x) { return null }
fn ln(x) { return null }
fn log10(x) { return null }
fn deg2rad(d) { return null }
fn rad2deg(r) { return null }
fn quadratic(a, b, c) { return null }
fn kinetic_energy(m, v) { return null }
fn potential_energy(m, g, h) { return null }
fn force(m, a) { return null }
fn ohms_v(i, r) { return null }
fn ohms_p(v, i) { return null }
fn wavelength(f) { return null }
fn photon_energy(f) { return null }
fn relativity_e(m) { return null }
fn ph(h_plus) { return null }
fn h_plus(ph_val) { return null }
fn molarity(moles, volume_l) { return null }
fn ideal_gas_p(n, temp_k, volume_l) { return null }
fn dilution(c1, v1, c2) { return null }
fn compound(principal, rate, years) { return null }
fn present_value(fv, rate, years) { return null }
fn break_even(fixed, price, variable) { return null }
fn roi(gain, cost) { return null }
fn margin(revenue, cost) { return null }
fn bit_and(a, b) { return null }
fn bit_or(a, b) { return null }
fn bit_xor(a, b) { return null }
fn bit_not(a) { return null }
fn shl(a, n) { return null }
fn shr(a, n) { return null }
fn hex(s) { return null }
fn bin(s) { return null }
fn hamming_weight(n) { return null }
fn stat_mean(data) { return null }
fn stat_std(data) { return null }
fn stat_linreg(x, y) { return null }
fn mat(rows, cols) { return null }
fn mat_mul(a, b) { return null }
fn mat_det(m) { return null }
fn mat_inv(m) { return null }
fn num_trapz(xs, ys) { return null }
fn num_solve(a, b) { return null }
fn num_interp_linear(xs, ys, x) { return null }
"#;

static BUILTIN_DOCAI: &str = r#"
fn doc_ask(query) { return null }
fn doc_search(query) { return null }
fn doc_sources(query) { return null }
fn doc_topics() { return null }
"#;

static BUILTIN_CODAI: &str = r#"
fn code_utils() { return null }
fn code_util(id) { return null }
fn code_suggest(query) { return null }
fn code_compose(ids) { return null }
fn code_complete(partial) { return null }
fn code_explain(code) { return null }
fn code_help(topic) { return null }
fn code_categories() { return null }
fn code_projects() { return null }
fn code_project_suggest(query) { return null }
fn code_project_tree(id) { return null }
fn code_project_plan(id) { return null }
fn code_project_scaffold(id) { return null }
fn code_project_progress(id) { return null }
fn code_project_sync(path) { return null }
"#;

pub fn list_builtins() -> Vec<&'static str> {
    vec![
        "std",
        "json",
        "collections",
        "strings",
        "math",
        "http",
        "crypto",
        "science",
        "docai",
        "codai",
        "kv8",
    ]
}

pub fn builtin_source(name: &str) -> Option<&'static str> {
    match name {
        "std" => Some(BUILTIN_STD),
        "json" => Some(BUILTIN_JSON),
        "collections" => Some(BUILTIN_COLLECTIONS),
        "strings" => Some(BUILTIN_STRINGS),
        "math" => Some(BUILTIN_MATH),
        "http" => Some(BUILTIN_HTTP),
        "crypto" => Some(BUILTIN_CRYPTO),
        "science" => Some(BUILTIN_SCIENCE),
        "docai" => Some(BUILTIN_DOCAI),
        "codai" => Some(BUILTIN_CODAI),
        _ => None,
    }
}

pub fn module_registry() -> HashMap<&'static str, &'static str> {
    list_builtins()
        .into_iter()
        .filter_map(|name| builtin_source(name).map(|src| (name, src)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::{create_global_env, eval_source};
    use crate::value::Value;

    #[test]
    fn import_math_module() {
        let mut env = create_global_env();
        import_module("math", &mut env).unwrap();
        let add = env.get("add").expect("add should be defined");
        assert!(matches!(
            add,
            crate::value::Value::Function { .. } | crate::value::Value::BytecodeFn(_)
        ));
    }

    #[test]
    fn import_science_complex() {
        let mut env = create_global_env();
        import_module("science", &mut env).unwrap();
        let val = eval_source("c_abs(cplx(3, 4))", &mut env).unwrap();
        assert!(matches!(val, crate::value::Value::Float(f) if (f - 5.0).abs() < 1e-9));
    }

    #[test]
    fn science_not_in_global_env_until_import() {
        let env = create_global_env();
        assert!(env.get("cplx").is_none());
    }

    #[test]
    fn import_docai_module() {
        let mut env = create_global_env();
        import_module("docai", &mut env).unwrap();
        let val = eval_source("doc_topics()", &mut env).unwrap();
        assert!(matches!(val, crate::value::Value::Array(items) if !items.is_empty()));
    }

    #[test]
    fn import_codai_module() {
        let mut env = create_global_env();
        import_module("codai", &mut env).unwrap();
        let val = eval_source("code_utils()", &mut env).unwrap();
        assert!(matches!(val, crate::value::Value::Array(items) if !items.is_empty()));
    }

    #[test]
    fn import_http_builtin_loads() {
        let mut env = create_global_env();
        import_module("http", &mut env).unwrap();
        assert!(env.get("route_put").is_some());
        assert!(env.get("request_delete").is_some());
    }

    #[test]
    fn import_http_verbs() {
        let mut env = create_global_env();
        eval_source(
            r#"fn users_list() { return http_response(200, "[]") }"#,
            &mut env,
        )
        .unwrap();
        import_module("http", &mut env).unwrap();
        eval_source(
            r#"
            http_route("GET", "/api/users", users_list)
            http_body(request_get("/api/users"))
            "#,
            &mut env,
        )
        .unwrap();
        let val = eval_source(
            r#"http_body(request_get("/api/users"))"#,
            &mut env,
        )
        .unwrap();
        assert!(matches!(val, Value::String(s) if s == "[]"));
    }
}
