//! Guest WebAssembly — run C/Rust-compiled `.wasm` inside Kabootar Browser sandbox.

use super::devtools;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

#[derive(Clone)]
pub struct WasmModule {
    pub id: u64,
    pub name: String,
    pub bytes: Vec<u8>,
    pub exports: Vec<String>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn meta_store() -> &'static Mutex<HashMap<u64, WasmModule>> {
    static S: OnceLock<Mutex<HashMap<u64, WasmModule>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn validate_wasm(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 8 {
        return Err("wasm: file too short".into());
    }
    if bytes[0..4] != WASM_MAGIC {
        return Err("wasm: invalid magic (expected \\0asm)".into());
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 1 {
        return Err(format!("wasm: unsupported version {version}"));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
mod wasmi_rt {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use wasmi::{Caller, Engine, Error, Func, Instance, Linker, Module, Store};

    #[derive(Default)]
    pub struct HostState;

    pub struct ModuleRuntime {
        pub store: Store<HostState>,
        pub instance: Instance,
    }

    static ENGINE: OnceLock<Engine> = OnceLock::new();
    static RUNTIMES: OnceLock<Mutex<HashMap<u64, ModuleRuntime>>> = OnceLock::new();

    fn engine() -> &'static Engine {
        ENGINE.get_or_init(Engine::default)
    }

    fn runtimes() -> &'static Mutex<HashMap<u64, ModuleRuntime>> {
        RUNTIMES.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn parse_exports(bytes: &[u8]) -> Result<Vec<String>, String> {
        let module = Module::new(engine(), bytes).map_err(|e| format!("wasm parse: {e}"))?;
        Ok(module
            .exports()
            .map(|ex| ex.name().to_string())
            .collect())
    }

    pub fn instantiate(id: u64, bytes: &[u8]) -> Result<(), String> {
        let module = Module::new(engine(), bytes).map_err(|e| format!("wasm module: {e}"))?;
        let mut store = Store::new(engine(), HostState);
        let mut linker = Linker::new(engine());

        let log_fn = Func::wrap(
            &mut store,
            |caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<(), Error> {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .ok_or_else(|| Error::new("wasm log: no memory"))?;
                let data = mem.data(&caller);
                let start = ptr as usize;
                let end = start.saturating_add(len.max(0) as usize);
                if end > data.len() {
                    return Err(Error::new("wasm log: bad range"));
                }
                let msg = std::str::from_utf8(&data[start..end]).unwrap_or("<binary>");
                devtools::console_log("log", msg, "wasm");
                Ok(())
            },
        );
        linker
            .define("env", "log", log_fn)
            .map_err(|e| format!("wasm linker: {e}"))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("wasm link: {e}"))?
            .start(&mut store)
            .map_err(|e| format!("wasm instantiate: {e}"))?;

        runtimes()
            .lock()
            .map_err(|_| "wasm runtime lock".to_string())?
            .insert(id, ModuleRuntime { store, instance });
        Ok(())
    }

    pub fn call_export(id: u64, export: &str, args: &[i32]) -> Result<i32, String> {
        let mut guard = runtimes()
            .lock()
            .map_err(|_| "wasm runtime lock".to_string())?;
        let rt = guard.get_mut(&id).ok_or("wasm: module not instantiated")?;
        let result = match args.len() {
            0 => {
                let f = rt
                    .instance
                    .get_typed_func::<(), i32>(&rt.store, export)
                    .map_err(|e| format!("wasm typed func: {e}"))?;
                f.call(&mut rt.store, ())
                    .map_err(|e| format!("wasm call: {e}"))?
            }
            1 => {
                let f = rt
                    .instance
                    .get_typed_func::<i32, i32>(&rt.store, export)
                    .map_err(|e| format!("wasm typed func: {e}"))?;
                f.call(&mut rt.store, args[0])
                    .map_err(|e| format!("wasm call: {e}"))?
            }
            2 => {
                let f = rt
                    .instance
                    .get_typed_func::<(i32, i32), i32>(&rt.store, export)
                    .map_err(|e| format!("wasm typed func: {e}"))?;
                f.call(&mut rt.store, (args[0], args[1]))
                    .map_err(|e| format!("wasm call: {e}"))?
            }
            n => return Err(format!("wasm: unsupported arg count {n}")),
        };
        Ok(result)
    }
}

#[cfg(target_arch = "wasm32")]
mod wasmi_rt {
    use super::*;

    pub fn parse_exports(_bytes: &[u8]) -> Result<Vec<String>, String> {
        Ok(vec!["memory".into()])
    }

    pub fn instantiate(_id: u64, _bytes: &[u8]) -> Result<(), String> {
        Err("wasm guest execution unavailable on wasm32 host".into())
    }

    pub fn call_export(_id: u64, _export: &str, _args: &[i32]) -> Result<i32, String> {
        Err("wasm guest execution unavailable on wasm32 host".into())
    }
}

pub fn load_wasm(name: &str, bytes: Vec<u8>) -> Result<WasmModule, String> {
    validate_wasm(&bytes)?;
    let exports = wasmi_rt::parse_exports(&bytes)?;
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    wasmi_rt::instantiate(id, &bytes)?;
    let module = WasmModule {
        id,
        name: name.into(),
        exports,
        bytes,
    };
    meta_store()
        .lock()
        .map_err(|_| "wasm store lock poisoned".to_string())?
        .insert(id, module.clone());
    Ok(module)
}

pub fn get_module(id: u64) -> Option<WasmModule> {
    meta_store().lock().ok()?.get(&id).cloned()
}

pub fn run_export(id: u64, export: &str, args: &[i32]) -> Result<i32, String> {
    let module = get_module(id).ok_or("wasm: unknown module")?;
    if !module.exports.iter().any(|e| e == export) {
        return Err(format!("wasm: export '{export}' not found"));
    }
    wasmi_rt::call_export(id, export, args)
}

pub fn list_modules() -> Vec<WasmModule> {
    meta_store()
        .lock()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

pub fn info() -> HashMap<String, String> {
    let mut o = HashMap::new();
    o.insert("engine".into(), "kabootar-wasm+wasmi".into());
    o.insert("phase".into(), "v2.51".into());
    o.insert("guest".into(), "true".into());
    o.insert("host".into(), "kabootar-browser".into());
    o.insert("imports".into(), "env.log".into());
    o.insert("modules".into(), list_modules().len().to_string());
    o
}

/// Minimal wasm module: `(export "add" (func $add))` — i32 add.
pub fn minimal_add_wasm() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f,
        0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00,
        0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_runs_add() {
        let wasm = minimal_add_wasm();
        assert!(validate_wasm(&wasm).is_ok());
        let m = load_wasm("add", wasm).unwrap();
        assert!(m.exports.iter().any(|e| e == "add"));
        let sum = run_export(m.id, "add", &[5, 7]).unwrap();
        assert_eq!(sum, 12);
    }
}
