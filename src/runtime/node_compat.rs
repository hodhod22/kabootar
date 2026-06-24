//! Node.js built-in module compatibility (`import "node:fs"`, etc.) — Deno våg 17.

use crate::evaluator::{create_global_env, eval_source};
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

pub const NODE_MODULES: &[&str] = &[
    "fs",
    "path",
    "process",
    "os",
    "url",
    "buffer",
    "crypto",
];

static NODE_FS: &str = r#"
pub fn readFileSync(path) {
    return read_file(path)
}
pub fn writeFileSync(path, data) {
    return write_file(path, data)
}
pub fn mkdirSync(path) {
    return mkdir(path)
}
pub fn statSync(path) {
    return stat(path)
}
pub fn readdirSync(path) {
    return read_dir(path)
}
pub fn rmSync(path) {
    return remove(path)
}
pub fn existsSync(path) {
    return exists(path)
}
pub fn readTextFileSync(path) {
    return read_text_file(path)
}
pub fn writeTextFileSync(path, text) {
    return write_text_file(path, text)
}
"#;

static NODE_FS_PROMISES: &str = r#"
pub fn readFile(path) {
    return read_file(path)
}
pub fn writeFile(path, data) {
    return write_file(path, data)
}
pub fn mkdir(path) {
    return Deno_mkdir(path)
}
pub fn stat(path) {
    return Deno_stat(path)
}
pub fn readdir(path) {
    return Deno_readDir(path)
}
pub fn rm(path) {
    return Deno_remove(path)
}
pub fn access(path) {
    return Deno_exists(path)
}
"#;

static NODE_PATH: &str = r#"
pub fn join(a, b) {
    return node_path_join(a, b)
}
pub fn resolve(a, b) {
    return node_path_resolve(a, b)
}
pub fn dirname(p) {
    return node_path_dirname(p)
}
pub fn basename(p) {
    return node_path_basename(p)
}
pub fn extname(p) {
    return node_path_extname(p)
}
pub fn normalize(p) {
    return node_path_normalize(p)
}
pub fn sep() {
    return node_path_sep()
}
pub fn delimiter() {
    return node_path_delimiter()
}
"#;

static NODE_PROCESS: &str = r#"
pub fn cwd() {
    return Deno_cwd()
}
pub fn chdir(path) {
    return Deno_chdir(path)
}
pub fn env() {
    return env_to_object()
}
pub fn argv() {
    return []
}
pub fn platform() {
    return node_os_platform()
}
pub fn arch() {
    return node_os_arch()
}
"#;

static NODE_OS: &str = r#"
pub fn platform() {
    return node_os_platform()
}
pub fn arch() {
    return node_os_arch()
}
pub fn homedir() {
    return node_os_homedir()
}
pub fn tmpdir() {
    return node_os_tmpdir()
}
pub fn endianness() {
    return node_os_endianness()
}
pub fn EOL() {
    return node_os_eol()
}
"#;

static NODE_URL: &str = r#"
pub fn parse(input) {
    return url_new(input)
}
pub fn format(obj) {
    if has_key(obj, "href") {
        return obj["href"]
    }
    return ""
}
pub fn fileURLToPath(url) {
    return node_file_url_to_path(url)
}
pub fn pathToFileURL(path) {
    return node_path_to_file_url(path)
}
"#;

static NODE_BUFFER: &str = r#"
pub fn from(data) {
    return node_buffer_from(data)
}
pub fn alloc(size) {
    return node_buffer_alloc(size)
}
pub fn isBuffer(v) {
    return node_buffer_is_buffer(v)
}
"#;

static NODE_CRYPTO: &str = r#"
pub fn randomBytes(size) {
    return crypto_random(size)
}
"#;

fn str_arg(v: &Value, name: &str) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        other => Err(format!("{name} expects string, got {:?}", other)),
    }
}

fn path_to_forward_slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn node_path_join_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::String(".".into()));
    }
    let first = str_arg(&args[0], "node_path_join")?;
    let mut out = PathBuf::from(first);
    for arg in args.iter().skip(1) {
        out.push(str_arg(arg, "node_path_join")?);
    }
    Ok(Value::String(path_to_forward_slash(&out)))
}

fn node_path_resolve_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut out = cwd;
    for arg in args {
        let s = str_arg(arg, "node_path_resolve")?;
        let part = Path::new(&s);
        if part.is_absolute() {
            out = part.to_path_buf();
        } else {
            out = out.join(part);
        }
    }
    Ok(Value::String(path_to_forward_slash(&out)))
}

fn node_path_dirname_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let p = str_arg(args.first().ok_or("node_path_dirname(path)")?, "node_path_dirname")?;
    let parent = Path::new(&p).parent().unwrap_or_else(|| Path::new(""));
    Ok(Value::String(path_to_forward_slash(parent)))
}

fn node_path_basename_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let p = str_arg(args.first().ok_or("node_path_basename(path)")?, "node_path_basename")?;
    Ok(Value::String(
        Path::new(&p)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    ))
}

fn node_path_extname_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let p = str_arg(args.first().ok_or("node_path_extname(path)")?, "node_path_extname")?;
    Ok(Value::String(
        Path::new(&p)
            .extension()
            .map(|s| format!(".{}", s.to_string_lossy()))
            .unwrap_or_default(),
    ))
}

fn node_path_normalize_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let p = str_arg(args.first().ok_or("node_path_normalize(path)")?, "node_path_normalize")?;
    let mut out = PathBuf::new();
    for component in Path::new(&p).components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    Ok(Value::String(path_to_forward_slash(&out)))
}

fn node_path_sep_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(std::path::MAIN_SEPARATOR.to_string()))
}

fn node_path_delimiter_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    #[cfg(windows)]
    let d = ";";
    #[cfg(not(windows))]
    let d = ":";
    Ok(Value::String(d.into()))
}

fn node_os_platform_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let platform = match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        other => other,
    };
    Ok(Value::String(platform.into()))
}

fn node_os_arch_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(std::env::consts::ARCH.into()))
}

fn node_os_homedir_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    #[cfg(windows)]
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOMEDRIVE").and_then(|d| {
        std::env::var("HOMEPATH").map(|p| format!("{d}{p}"))
    }));
    #[cfg(not(windows))]
    let home = std::env::var("HOME");
    Ok(Value::String(home.unwrap_or_else(|_| "/".into())))
}

fn node_os_tmpdir_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(
        std::env::temp_dir().to_string_lossy().into_owned(),
    ))
}

fn node_os_endianness_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if cfg!(target_endian = "little") {
        Ok(Value::String("LE".into()))
    } else {
        Ok(Value::String("BE".into()))
    }
}

fn node_os_eol_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    #[cfg(windows)]
    let eol = "\r\n";
    #[cfg(not(windows))]
    let eol = "\n";
    Ok(Value::String(eol.into()))
}

fn node_file_url_to_path_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let url = str_arg(args.first().ok_or("node_file_url_to_path(url)")?, "node_file_url_to_path")?;
    let path = url
        .strip_prefix("file://")
        .or_else(|| url.strip_prefix("file:"))
        .unwrap_or(&url);
    let path = path.trim_start_matches('/');
    #[cfg(windows)]
    {
        if let Some(drive) = path.strip_prefix('/') {
            if drive.len() >= 2 && drive.as_bytes()[1] == b':' {
                return Ok(Value::String(drive.replace('/', "\\")));
            }
        }
    }
    Ok(Value::String(path.replace('/', std::path::MAIN_SEPARATOR_STR)))
}

fn node_path_to_file_url_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let path = str_arg(args.first().ok_or("node_path_to_file_url(path)")?, "node_path_to_file_url")?;
    let normalized = path.replace('\\', "/");
    let href = if Path::new(&path).is_absolute() {
        format!("file:///{normalized}")
    } else {
        format!("file://{normalized}")
    };
    Ok(Value::String(href))
}

fn bytes_to_array(data: &[u8]) -> Value {
    Value::Array(
        data.iter()
            .map(|b| Value::Number(*b as i64))
            .collect(),
    )
}

fn node_buffer_from_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = args.first().ok_or("node_buffer_from(data)")?;
    match data {
        Value::String(s) => Ok(bytes_to_array(s.as_bytes())),
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::Number(n) if (0..=255).contains(n) => out.push(*n as u8),
                    _ => return Err("node_buffer_from array items must be 0..255".into()),
                }
            }
            Ok(bytes_to_array(&out))
        }
        _ => Err("node_buffer_from expects string or byte array".into()),
    }
}

fn node_buffer_alloc_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let size = match args.first() {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("node_buffer_alloc(size)".into()),
    };
    Ok(bytes_to_array(&vec![0u8; size]))
}

fn node_buffer_is_buffer_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("node_buffer_is_buffer(value)")?;
    Ok(Value::Bool(matches!(
        v,
        Value::Array(items) if !items.is_empty()
            && items.iter().all(|i| matches!(i, Value::Number(n) if (0..=255).contains(n)))
    )))
}

pub fn parse_node_spec(spec: &str) -> Result<(String, Option<String>), String> {
    let rest = spec
        .strip_prefix("node:")
        .ok_or_else(|| format!("not a node: spec: {spec}"))?;
    if rest.is_empty() {
        return Err("node: spec is empty".into());
    }
    if let Some((module, sub)) = rest.split_once('/') {
        if module.is_empty() {
            return Err(format!("invalid node: spec: {spec}"));
        }
        Ok((module.to_string(), Some(sub.to_string())))
    } else {
        Ok((rest.to_string(), None))
    }
}

pub fn node_module_source(spec: &str) -> Result<&'static str, String> {
    let (module, sub) = parse_node_spec(spec)?;
    match (module.as_str(), sub.as_deref()) {
        ("fs", None) => Ok(NODE_FS),
        ("fs", Some("promises")) => Ok(NODE_FS_PROMISES),
        ("path", None) => Ok(NODE_PATH),
        ("process", None) => Ok(NODE_PROCESS),
        ("os", None) => Ok(NODE_OS),
        ("url", None) => Ok(NODE_URL),
        ("buffer", None) => Ok(NODE_BUFFER),
        ("crypto", None) => Ok(NODE_CRYPTO),
        _ => Err(format!(
            "unsupported node: module \"{spec}\" (available: {})",
            NODE_MODULES
                .iter()
                .map(|m| format!("node:{m}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub fn node_resolve(spec: &str) -> Value {
    match node_module_source(spec) {
        Ok(_) => Value::String(spec.to_string()),
        Err(e) => {
            let mut m = HashMap::new();
            m.insert("error".into(), Value::String(e));
            Value::Object(m)
        }
    }
}

pub fn node_list() -> Value {
    Value::Array(
        NODE_MODULES
            .iter()
            .map(|m| Value::String(format!("node:{m}")))
            .collect(),
    )
}

pub fn node_import_source(spec: &str) -> Result<String, String> {
    Ok(node_module_source(spec)?.to_string())
}

fn export_bindings(module_env: &Environment, importer: &mut Environment) -> Vec<String> {
    let mut imported = Vec::new();
    for name in module_env.exported_names() {
        if let Some(val) = module_env.get(&name) {
            importer.set(name.clone(), val);
            imported.push(name);
        }
    }
    imported
}

pub fn import_node_module(spec: &str, env: &mut Environment) -> Result<Vec<String>, String> {
    let source = node_module_source(spec)?;
    let mut module_env = create_global_env();
    eval_source(source, &mut module_env)?;
    Ok(export_bindings(&module_env, env))
}

pub fn register_node_globals(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("node_path_join", node_path_join_native),
        ("node_path_resolve", node_path_resolve_native),
        ("node_path_dirname", node_path_dirname_native),
        ("node_path_basename", node_path_basename_native),
        ("node_path_extname", node_path_extname_native),
        ("node_path_normalize", node_path_normalize_native),
        ("node_path_sep", node_path_sep_native),
        ("node_path_delimiter", node_path_delimiter_native),
        ("node_os_platform", node_os_platform_native),
        ("node_os_arch", node_os_arch_native),
        ("node_os_homedir", node_os_homedir_native),
        ("node_os_tmpdir", node_os_tmpdir_native),
        ("node_os_endianness", node_os_endianness_native),
        ("node_os_eol", node_os_eol_native),
        ("node_file_url_to_path", node_file_url_to_path_native),
        ("node_path_to_file_url", node_path_to_file_url_native),
        ("node_buffer_from", node_buffer_from_native),
        ("node_buffer_alloc", node_buffer_alloc_native),
        ("node_buffer_is_buffer", node_buffer_is_buffer_native),
        ("node_resolve", node_resolve_native),
        ("node_list", node_list_native),
        ("node_import", node_import_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}

fn node_resolve_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let spec = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("node_resolve(spec)".into()),
    };
    Ok(node_resolve(spec))
}

fn node_list_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(node_list())
}

fn node_import_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let spec = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("node_import(spec)".into()),
    };
    Ok(Value::String(node_import_source(spec)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_node_specs() {
        let (m, s) = parse_node_spec("node:fs").unwrap();
        assert_eq!(m, "fs");
        assert!(s.is_none());
        let (m, s) = parse_node_spec("node:fs/promises").unwrap();
        assert_eq!(m, "fs");
        assert_eq!(s.as_deref(), Some("promises"));
    }

    #[test]
    fn path_join_and_extname() {
        let mut env = create_global_env();
        register_node_globals(&mut env);
        let out = node_path_join_native(
            &[Value::String("/tmp".into()), Value::String("a.txt".into())],
            &mut env,
        )
        .unwrap();
        assert!(matches!(out, Value::String(s) if s.ends_with("a.txt")));
        let out = node_path_extname_native(&[Value::String("x.y.js".into())], &mut env).unwrap();
        assert!(matches!(out, Value::String(s) if s == ".js"));
    }
}
