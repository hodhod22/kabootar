//! OpenXR / WebXR host FFI probe + bind (GP6n).
//!
//! Native: dynamically load `openxr_loader` via `libloading` and probe
//! `xrGetInstanceProcAddr` / `xrEnumerateInstanceExtensionProperties`.
//! Wasm: probe `navigator.xr` (WebXR).
//!
//! Full headset swapchain still uses the Kab runtime loop when no HMD is present;
//! when a loader/runtime is found, `xr_bind_headset` marks the session as FFI-bound.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::value::Value;

#[derive(Debug, Clone, Default)]
pub struct XrFfiStatus {
    pub openxr_loader: bool,
    pub openxr_runtime: bool,
    pub webxr: bool,
    pub bound: bool,
    pub backend: String,
    pub loader_path: String,
    pub detail: String,
}

static XR_FFI: Mutex<XrFfiStatus> = Mutex::new(XrFfiStatus {
    openxr_loader: false,
    openxr_runtime: false,
    webxr: false,
    bound: false,
    backend: String::new(),
    loader_path: String::new(),
    detail: String::new(),
});

pub fn reset_for_tests() {
    if let Ok(mut s) = XR_FFI.lock() {
        *s = XrFfiStatus {
            openxr_loader: false,
            openxr_runtime: false,
            webxr: false,
            bound: false,
            backend: String::new(),
            loader_path: String::new(),
            detail: String::new(),
        };
    }
}

pub fn status() -> XrFfiStatus {
    XR_FFI.lock().map(|s| s.clone()).unwrap_or_default()
}

fn env_loader_override() -> Option<String> {
    std::env::var("KABOOTAR_XR_LOADER").ok().filter(|s| !s.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn openxr_loader_candidates() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(p) = env_loader_override() {
        out.push(p);
    }
    #[cfg(target_os = "windows")]
    {
        out.push("openxr_loader.dll".into());
        out.push("OpenXR-SDK/openxr_loader.dll".into());
    }
    #[cfg(target_os = "linux")]
    {
        out.push("libopenxr_loader.so".into());
        out.push("libopenxr_loader.so.1".into());
    }
    #[cfg(target_os = "macos")]
    {
        out.push("libopenxr_loader.dylib".into());
    }
    out
}

/// OpenXR: XrResult == 0 is success.
#[cfg(not(target_arch = "wasm32"))]
type XrResult = i32;
#[cfg(not(target_arch = "wasm32"))]
type XrGetInstanceProcAddr = unsafe extern "system" fn(
    instance: u64,
    name: *const std::os::raw::c_char,
    function: *mut *const std::os::raw::c_void,
) -> XrResult;

#[cfg(not(target_arch = "wasm32"))]
fn probe_openxr_loader() -> (bool, bool, String, String) {
    use libloading::Library;
    use std::ffi::CString;

    for path in openxr_loader_candidates() {
        let lib = match unsafe { Library::new(&path) } {
            Ok(l) => l,
            Err(_) => continue,
        };
        // Keep library leaked for process lifetime (loader must stay mapped).
        let lib = Box::leak(Box::new(lib));
        let get_proc: libloading::Symbol<XrGetInstanceProcAddr> =
            match unsafe { lib.get(b"xrGetInstanceProcAddr\0") } {
                Ok(s) => s,
                Err(e) => {
                    return (
                        true,
                        false,
                        path,
                        format!("loader loaded but xrGetInstanceProcAddr missing: {e}"),
                    );
                }
            };

        // Probe xrEnumerateInstanceExtensionProperties via GetInstanceProcAddr(NULL, ...).
        let mut fn_ptr: *const std::os::raw::c_void = std::ptr::null();
        let name = CString::new("xrEnumerateInstanceExtensionProperties").unwrap();
        let rc = unsafe { get_proc(0, name.as_ptr(), &mut fn_ptr) };
        if rc == 0 && !fn_ptr.is_null() {
            return (
                true,
                true,
                path,
                "openxr_loader + xrEnumerateInstanceExtensionProperties".into(),
            );
        }
        return (
            true,
            false,
            path,
            format!("loader found; runtime proc probe rc={rc}"),
        );
    }
    (
        false,
        false,
        String::new(),
        "openxr_loader not found (set KABOOTAR_XR_LOADER)".into(),
    )
}

#[cfg(target_arch = "wasm32")]
fn probe_openxr_loader() -> (bool, bool, String, String) {
    (false, false, String::new(), "openxr not available on wasm32".into())
}

#[cfg(target_arch = "wasm32")]
fn probe_webxr() -> (bool, String) {
    let Some(window) = web_sys::window() else {
        return (false, "no window".into());
    };
    let navigator = window.navigator();
    match js_sys::Reflect::get(&navigator, &wasm_bindgen::JsValue::from_str("xr")) {
        Ok(xr) if !xr.is_undefined() && !xr.is_null() => (true, "navigator.xr present".into()),
        _ => (false, "navigator.xr missing".into()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn probe_webxr() -> (bool, String) {
    (false, "webxr only on wasm32".into())
}

/// Refresh FFI probe results into process state (does not require a headset).
pub fn probe() -> XrFfiStatus {
    let (openxr_loader, openxr_runtime, loader_path, ox_detail) = probe_openxr_loader();
    let (webxr, wx_detail) = probe_webxr();
    let backend = if openxr_runtime {
        "openxr-ffi"
    } else if openxr_loader {
        "openxr-loader"
    } else if webxr {
        "webxr-ffi"
    } else {
        "none"
    };
    let detail = if openxr_loader {
        ox_detail
    } else if webxr {
        wx_detail
    } else {
        format!("{ox_detail}; {wx_detail}")
    };
    let mut status = XrFfiStatus {
        openxr_loader,
        openxr_runtime,
        webxr,
        bound: false,
        backend: backend.into(),
        loader_path,
        detail,
    };
    if let Ok(mut g) = XR_FFI.lock() {
        status.bound = g.bound;
        *g = status.clone();
        g.bound = status.bound;
    }
    status
}

/// Bind headset runtime when OpenXR/WebXR is available (or stub when forced).
pub fn bind_headset(force_stub: bool) -> Result<XrFfiStatus, String> {
    let mut st = probe();
    if force_stub {
        st.bound = true;
        st.backend = "xr-stub".into();
        st.detail = "bound via KABOOTAR_XR_STUB".into();
    } else if st.openxr_runtime || st.webxr {
        st.bound = true;
        if st.openxr_runtime {
            st.backend = "openxr-ffi".into();
        } else {
            st.backend = "webxr-ffi".into();
        }
    } else if st.openxr_loader {
        // Loader without runtime: bind in loader-only mode (no HMD yet).
        st.bound = true;
        st.backend = "openxr-loader".into();
        st.detail = format!("{} (no active runtime/HMD)", st.detail);
    } else {
        return Err(format!(
            "xr_bind_headset: no OpenXR/WebXR runtime ({})",
            st.detail
        ));
    }
    if let Ok(mut g) = XR_FFI.lock() {
        *g = st.clone();
    }
    Ok(st)
}

pub fn status_value() -> Value {
    let s = status();
    let mut out = HashMap::new();
    out.insert("openxrLoader".into(), Value::Bool(s.openxr_loader));
    out.insert("openxrRuntime".into(), Value::Bool(s.openxr_runtime));
    out.insert("webxr".into(), Value::Bool(s.webxr));
    out.insert("bound".into(), Value::Bool(s.bound));
    out.insert("backend".into(), Value::String(s.backend));
    out.insert("loaderPath".into(), Value::String(s.loader_path));
    out.insert("detail".into(), Value::String(s.detail));
    Value::Object(out)
}
