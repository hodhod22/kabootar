//! Deno FFI parity — dynamic library calls (native).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FFI: AtomicU64 = AtomicU64::new(1);

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static FFI_LIBS: RefCell<HashMap<u64, libloading::Library>> = RefCell::new(HashMap::new());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ffi_load(path: &str) -> Result<u64, String> {
    let lib = unsafe {
        libloading::Library::new(path).map_err(|e| format!("ffi_load failed: {e}"))?
    };
    let id = NEXT_FFI.fetch_add(1, Ordering::Relaxed);
    FFI_LIBS.with(|m| m.borrow_mut().insert(id, lib));
    Ok(id)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ffi_call_i64(lib_id: u64, symbol: &str, args: &[i64]) -> Result<i64, String> {
    use libloading::Symbol;
    FFI_LIBS.with(|m| {
        let map = m.borrow();
        let lib = map
            .get(&lib_id)
            .ok_or_else(|| format!("invalid ffi library id {lib_id}"))?;
        unsafe {
            let sym: Symbol<unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> i64> = lib
                .get(symbol.as_bytes())
                .map_err(|e| format!("ffi symbol {symbol}: {e}"))?;
            let f = sym.into_raw();
            let a0 = args.first().copied().unwrap_or(0);
            let a1 = args.get(1).copied().unwrap_or(0);
            let a2 = args.get(2).copied().unwrap_or(0);
            let a3 = args.get(3).copied().unwrap_or(0);
            let a4 = args.get(4).copied().unwrap_or(0);
            let a5 = args.get(5).copied().unwrap_or(0);
            Ok(f(a0, a1, a2, a3, a4, a5))
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ffi_close(lib_id: u64) -> Result<(), String> {
    FFI_LIBS.with(|m| {
        if m.borrow_mut().remove(&lib_id).is_some() {
            Ok(())
        } else {
            Err(format!("invalid ffi library id {lib_id}"))
        }
    })
}

#[cfg(target_arch = "wasm32")]
pub fn ffi_load(_path: &str) -> Result<u64, String> {
    Err("ffi_load() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn ffi_call_i64(_lib_id: u64, _symbol: &str, _args: &[i64]) -> Result<i64, String> {
    Err("ffi_call_i64() is not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
pub fn ffi_close(_lib_id: u64) -> Result<(), String> {
    Err("ffi_close() is not available on wasm32".into())
}

#[cfg(all(test, not(target_arch = "wasm32"), unix))]
mod tests {
    use super::*;

    #[test]
    fn ffi_getpid() {
        let id = ffi_load("libc.so.6")
            .or_else(|_| ffi_load("libc.so"))
            .expect("load libc");
        let pid = ffi_call_i64(id, "getpid", &[]).expect("getpid");
        assert!(pid > 0);
        ffi_close(id).unwrap();
    }
}
