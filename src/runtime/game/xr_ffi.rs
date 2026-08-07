//! OpenXR / WebXR host FFI probe + bind + HMD driver present (GP6n).
//!
//! Native: dynamically load `openxr_loader` via `libloading` and probe
//! `xrGetInstanceProcAddr` / `xrEnumerateInstanceExtensionProperties`.
//! Wasm: probe `navigator.xr` (WebXR).
//!
//! HMD driver present accepts a composed projection-layer frame and either:
//! - submits via stub/driver queue (`KABOOTAR_XR_STUB` / bound loader), or
//! - records a vendor present descriptor when a real OpenXR runtime is present.

use std::cell::RefCell;
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
    pub hmd_connected: bool,
    pub vendor: String,
    pub form_factor: String,
}

#[derive(Debug, Clone, Default)]
struct HmdDriverState {
    presents: i64,
    last_frame_index: i64,
    last_view_count: i64,
    last_backend: String,
}

static XR_FFI: Mutex<XrFfiStatus> = Mutex::new(XrFfiStatus {
    openxr_loader: false,
    openxr_runtime: false,
    webxr: false,
    bound: false,
    backend: String::new(),
    loader_path: String::new(),
    detail: String::new(),
    hmd_connected: false,
    vendor: String::new(),
    form_factor: String::new(),
});

static HMD_DRIVER: Mutex<HmdDriverState> = Mutex::new(HmdDriverState {
    presents: 0,
    last_frame_index: 0,
    last_view_count: 0,
    last_backend: String::new(),
});

/// In-process OpenXR-style compositor IPC channel (frame submit / poll ack).
#[derive(Debug, Clone)]
struct CompositorMsg {
    frame_index: i64,
    view_count: i64,
    width: i64,
    height: i64,
    layer_type: String,
}

#[derive(Debug, Default)]
struct CompositorIpc {
    open: bool,
    channel: String,
    pending: Vec<CompositorMsg>,
    acks: Vec<i64>,
    submits: i64,
}

static COMPOSITOR_IPC: Mutex<CompositorIpc> = Mutex::new(CompositorIpc {
    open: false,
    channel: String::new(),
    pending: Vec::new(),
    acks: Vec::new(),
    submits: 0,
});

/// Resolved OpenXR loader entry points (native only).
#[derive(Debug, Default)]
struct OpenXrLoaderFns {
    /// `xrGetInstanceProcAddr` raw pointer (0 = none).
    get_proc: usize,
    /// `xrEndFrame` raw pointer when resolved (0 = unresolved).
    end_frame: usize,
    end_frame_resolved: bool,
    end_frame_calls: i64,
    last_frame_index: i64,
    last_path: String,
    create_instance: usize,
    create_session: usize,
    destroy_instance: usize,
    destroy_session: usize,
    get_system: usize,
    create_instance_resolved: bool,
    create_session_resolved: bool,
    get_system_resolved: bool,
}

static OPENXR_FNS: Mutex<OpenXrLoaderFns> = Mutex::new(OpenXrLoaderFns {
    get_proc: 0,
    end_frame: 0,
    end_frame_resolved: false,
    end_frame_calls: 0,
    last_frame_index: 0,
    last_path: String::new(),
    create_instance: 0,
    create_session: 0,
    destroy_instance: 0,
    destroy_session: 0,
    get_system: 0,
    create_instance_resolved: false,
    create_session_resolved: false,
    get_system_resolved: false,
});

/// Live OpenXR / WebXR session (handles + FFI accounting).
#[derive(Debug, Default, Clone)]
struct LiveSession {
    active: bool,
    mode: String,
    instance: u64,
    session: u64,
    system_id: u64,
    backend: String,
    end_frame_ffi_calls: i64,
    last_ffi_result: i32,
    last_ffi_mode: String,
    webxr_granted: bool,
    webxr_raf_bound: bool,
    instance_created: bool,
    session_created: bool,
    system_enumerated: bool,
    graphics_bound: bool,
    graphics_api: String,
    graphics_device: u64,
    graphics_binding_type: String,
    swapchain_graphics_bound: bool,
    wgpu_bound: bool,
    wgpu_device: u64,
    create_instance_rc: i32,
    get_system_rc: i32,
    create_session_rc: i32,
    create_path: String,
}

static LIVE_SESSION: Mutex<LiveSession> = Mutex::new(LiveSession {
    active: false,
    mode: String::new(),
    instance: 0,
    session: 0,
    system_id: 0,
    backend: String::new(),
    end_frame_ffi_calls: 0,
    last_ffi_result: 0,
    last_ffi_mode: String::new(),
    webxr_granted: false,
    webxr_raf_bound: false,
    instance_created: false,
    session_created: false,
    system_enumerated: false,
    graphics_bound: false,
    graphics_api: String::new(),
    graphics_device: 0,
    graphics_binding_type: String::new(),
    swapchain_graphics_bound: false,
    wgpu_bound: false,
    wgpu_device: 0,
    create_instance_rc: 0,
    get_system_rc: 0,
    create_session_rc: 0,
    create_path: String::new(),
});

/// Browser-style `navigator.xr.requestSession` Promise state.
#[derive(Debug, Default, Clone)]
struct WebXrSessionPromise {
    pending: bool,
    resolved: bool,
    rejected: bool,
    mode: String,
    promise_id: i64,
    detail: String,
    reason: String,
    raf_bound: bool,
}

static WEBXR_PROMISE: Mutex<WebXrSessionPromise> = Mutex::new(WebXrSessionPromise {
    pending: false,
    resolved: false,
    rejected: false,
    mode: String::new(),
    promise_id: 0,
    detail: String::new(),
    reason: String::new(),
    raf_bound: false,
});

/// OpenXR swapchain image pool backed by wgpu (or synthetic GPU handles).
#[derive(Debug, Default, Clone)]
struct GpuSwapchainImage {
    swapchain_id: i64,
    image_index: i64,
    native_image: u64,
    wgpu_texture_id: u64,
    width: u32,
    height: u32,
    eye: String,
}

static GPU_SWAPCHAIN_IMAGES: Mutex<Vec<GpuSwapchainImage>> = Mutex::new(Vec::new());
static NEXT_WGPU_TEXTURE_ID: Mutex<u64> = Mutex::new(1);
static NEXT_NATIVE_IMAGE: Mutex<u64> = Mutex::new(0x49_0000_0001);

/// Minimal OpenXR `XrFrameEndInfo` for FFI (layout-compatible on common ABIs).
#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct XrFrameEndInfo {
    ty: i32,
    next: *const std::os::raw::c_void,
    display_time: i64,
    environment_blend_mode: i32,
    layer_count: u32,
    layers: *const *const std::os::raw::c_void,
}

#[cfg(not(target_arch = "wasm32"))]
const XR_TYPE_FRAME_END_INFO: i32 = 33;
#[cfg(not(target_arch = "wasm32"))]
const XR_ENVIRONMENT_BLEND_MODE_OPAQUE: i32 = 1;
#[cfg(not(target_arch = "wasm32"))]
const XR_TYPE_INSTANCE_CREATE_INFO: i32 = 3;
#[cfg(not(target_arch = "wasm32"))]
const XR_TYPE_SESSION_CREATE_INFO: i32 = 4;
#[cfg(not(target_arch = "wasm32"))]
const XR_TYPE_SYSTEM_GET_INFO: i32 = 5;
/// Kab stub graphics binding (chained on XrSessionCreateInfo::next).
#[cfg(not(target_arch = "wasm32"))]
const XR_TYPE_GRAPHICS_BINDING_KABOOTAR: i32 = 100_099_0000;
#[cfg(not(target_arch = "wasm32"))]
const XR_TYPE_GRAPHICS_BINDING_VULKAN_KHR: i32 = 1_000_090_000;
#[cfg(not(target_arch = "wasm32"))]
const XR_TYPE_GRAPHICS_BINDING_D3D11_KHR: i32 = 1_000_027_000;
#[cfg(not(target_arch = "wasm32"))]
const XR_FORM_FACTOR_HEAD_MOUNTED_DISPLAY: i32 = 1;
#[cfg(not(target_arch = "wasm32"))]
const XR_MAX_APPLICATION_NAME_SIZE: usize = 128;
#[cfg(not(target_arch = "wasm32"))]
const XR_MAX_ENGINE_NAME_SIZE: usize = 128;

#[cfg(not(target_arch = "wasm32"))]
type XrEndFrameFn =
    unsafe extern "system" fn(session: u64, frame_end_info: *const XrFrameEndInfo) -> XrResult;
#[cfg(not(target_arch = "wasm32"))]
type XrCreateInstanceFn = unsafe extern "system" fn(
    create_info: *const XrInstanceCreateInfo,
    instance: *mut u64,
) -> XrResult;
#[cfg(not(target_arch = "wasm32"))]
type XrCreateSessionFn = unsafe extern "system" fn(
    instance: u64,
    create_info: *const XrSessionCreateInfo,
    session: *mut u64,
) -> XrResult;
#[cfg(not(target_arch = "wasm32"))]
type XrGetSystemFn = unsafe extern "system" fn(
    instance: u64,
    get_info: *const XrSystemGetInfo,
    system_id: *mut u64,
) -> XrResult;
#[cfg(not(target_arch = "wasm32"))]
type XrDestroyInstanceFn = unsafe extern "system" fn(instance: u64) -> XrResult;
#[cfg(not(target_arch = "wasm32"))]
type XrDestroySessionFn = unsafe extern "system" fn(session: u64) -> XrResult;

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct XrApplicationInfo {
    application_name: [u8; XR_MAX_APPLICATION_NAME_SIZE],
    application_version: u32,
    engine_name: [u8; XR_MAX_ENGINE_NAME_SIZE],
    engine_version: u32,
    api_version: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct XrInstanceCreateInfo {
    ty: i32,
    next: *const std::os::raw::c_void,
    create_flags: u64,
    application_info: XrApplicationInfo,
    enabled_api_layer_count: u32,
    enabled_api_layer_names: *const *const std::os::raw::c_char,
    enabled_extension_count: u32,
    enabled_extension_names: *const *const std::os::raw::c_char,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct XrSystemGetInfo {
    ty: i32,
    next: *const std::os::raw::c_void,
    form_factor: i32,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct XrGraphicsBindingKabootar {
    ty: i32,
    next: *const std::os::raw::c_void,
    /// 1=vulkan, 2=d3d11, 3=opengl, 4=stub
    api: u32,
    device: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct XrGraphicsBindingVulkanKHR {
    ty: i32,
    next: *const std::os::raw::c_void,
    instance: u64,
    physical_device: u64,
    device: u64,
    queue_family_index: u32,
    queue_index: u32,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct XrGraphicsBindingD3D11KHR {
    ty: i32,
    next: *const std::os::raw::c_void,
    device: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
struct XrSessionCreateInfo {
    ty: i32,
    next: *const std::os::raw::c_void,
    create_flags: u64,
    system_id: u64,
}

/// Stub trampoline matching `xrEndFrame` — used for live stub sessions / CI.
#[cfg(not(target_arch = "wasm32"))]
unsafe extern "system" fn stub_xr_end_frame(
    _session: u64,
    _frame_end_info: *const XrFrameEndInfo,
) -> XrResult {
    0
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "system" fn stub_xr_create_instance(
    _create_info: *const XrInstanceCreateInfo,
    instance: *mut u64,
) -> XrResult {
    if !instance.is_null() {
        *instance = 0x5B_0000_0001;
    }
    0
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "system" fn stub_xr_create_session(
    _instance: u64,
    create_info: *const XrSessionCreateInfo,
    session: *mut u64,
) -> XrResult {
    // Require a graphics binding chain for HMD present path.
    if !create_info.is_null() {
        let info = &*create_info;
        if info.system_id == 0 {
            return -2;
        }
        if info.next.is_null() {
            return -3; // missing graphics binding
        }
        let ty = *(info.next as *const i32);
        if ty != XR_TYPE_GRAPHICS_BINDING_KABOOTAR
            && ty != XR_TYPE_GRAPHICS_BINDING_VULKAN_KHR
            && ty != XR_TYPE_GRAPHICS_BINDING_D3D11_KHR
        {
            return -4; // unsupported graphics binding
        }
    }
    if !session.is_null() {
        *session = 0x5E_0000_0001;
    }
    0
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "system" fn stub_xr_get_system(
    _instance: u64,
    get_info: *const XrSystemGetInfo,
    system_id: *mut u64,
) -> XrResult {
    if !get_info.is_null() {
        let info = &*get_info;
        if info.form_factor != XR_FORM_FACTOR_HEAD_MOUNTED_DISPLAY {
            return -2;
        }
    }
    if !system_id.is_null() {
        *system_id = 0x57_0000_0001;
    }
    0
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "system" fn stub_xr_destroy_instance(_instance: u64) -> XrResult {
    0
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "system" fn stub_xr_destroy_session(_session: u64) -> XrResult {
    0
}

#[cfg(target_arch = "wasm32")]
fn stub_xr_end_frame_addr() -> usize {
    0
}

#[cfg(not(target_arch = "wasm32"))]
fn stub_xr_end_frame_addr() -> usize {
    stub_xr_end_frame as *const () as usize
}

#[cfg(not(target_arch = "wasm32"))]
fn stub_xr_create_instance_addr() -> usize {
    stub_xr_create_instance as *const () as usize
}

#[cfg(not(target_arch = "wasm32"))]
fn stub_xr_create_session_addr() -> usize {
    stub_xr_create_session as *const () as usize
}

#[cfg(not(target_arch = "wasm32"))]
fn stub_xr_get_system_addr() -> usize {
    stub_xr_get_system as *const () as usize
}

#[cfg(not(target_arch = "wasm32"))]
fn stub_xr_destroy_instance_addr() -> usize {
    stub_xr_destroy_instance as *const () as usize
}

#[cfg(not(target_arch = "wasm32"))]
fn stub_xr_destroy_session_addr() -> usize {
    stub_xr_destroy_session as *const () as usize
}

#[cfg(target_arch = "wasm32")]
fn stub_xr_create_instance_addr() -> usize {
    0
}
#[cfg(target_arch = "wasm32")]
fn stub_xr_create_session_addr() -> usize {
    0
}
#[cfg(target_arch = "wasm32")]
fn stub_xr_get_system_addr() -> usize {
    0
}
#[cfg(target_arch = "wasm32")]
fn stub_xr_destroy_instance_addr() -> usize {
    0
}
#[cfg(target_arch = "wasm32")]
fn stub_xr_destroy_session_addr() -> usize {
    0
}

/// WebXR-style XRSession.requestAnimationFrame queue (thread-local; holds Value callbacks).
#[derive(Debug, Default)]
struct XrRafState {
    next_id: i64,
    pending: Vec<(i64, Value)>,
    cancelled: Vec<i64>,
    ticks: i64,
    bound: bool,
    backend: String,
}

thread_local! {
    static XR_RAF: RefCell<XrRafState> = RefCell::new(XrRafState {
        next_id: 1,
        pending: Vec::new(),
        cancelled: Vec::new(),
        ticks: 0,
        bound: false,
        backend: String::new(),
    });
}

pub fn reset_for_tests() {
    if let Ok(mut s) = XR_FFI.lock() {
        *s = XrFfiStatus::default();
    }
    if let Ok(mut d) = HMD_DRIVER.lock() {
        *d = HmdDriverState::default();
    }
    if let Ok(mut c) = COMPOSITOR_IPC.lock() {
        *c = CompositorIpc::default();
    }
    if let Ok(mut p) = COMPOSITOR_PROC.lock() {
        *p = CompositorProcess::default();
    }
    if let Ok(mut f) = OPENXR_FNS.lock() {
        *f = OpenXrLoaderFns::default();
    }
    if let Ok(mut live) = LIVE_SESSION.lock() {
        *live = LiveSession::default();
    }
    if let Ok(mut p) = WEBXR_PROMISE.lock() {
        *p = WebXrSessionPromise::default();
    }
    if let Ok(mut imgs) = GPU_SWAPCHAIN_IMAGES.lock() {
        imgs.clear();
    }
    if let Ok(mut n) = NEXT_WGPU_TEXTURE_ID.lock() {
        *n = 1;
    }
    if let Ok(mut n) = NEXT_NATIVE_IMAGE.lock() {
        *n = 0x49_0000_0001;
    }
    XR_RAF.with(|r| {
        *r.borrow_mut() = XrRafState {
            next_id: 1,
            ..XrRafState::default()
        };
    });
    reset_input_for_tests();
}

pub fn status() -> XrFfiStatus {
    XR_FFI.lock().map(|s| s.clone()).unwrap_or_default()
}

fn env_loader_override() -> Option<String> {
    std::env::var("KABOOTAR_XR_LOADER").ok().filter(|s| !s.is_empty())
}

fn env_vendor_override() -> Option<String> {
    std::env::var("KABOOTAR_XR_VENDOR").ok().filter(|s| !s.is_empty())
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

        let get_proc_addr = (*get_proc) as usize;
        if let Ok(mut fns) = OPENXR_FNS.lock() {
            fns.get_proc = get_proc_addr;
        }

        // Probe xrEnumerateInstanceExtensionProperties via GetInstanceProcAddr(NULL, ...).
        let mut fn_ptr: *const std::os::raw::c_void = std::ptr::null();
        let name = CString::new("xrEnumerateInstanceExtensionProperties").unwrap();
        let rc = unsafe { get_proc(0, name.as_ptr(), &mut fn_ptr) };
        let runtime_ok = rc == 0 && !fn_ptr.is_null();

        // Best-effort resolve core session procs (may need live instance on some runtimes).
        let _ = resolve_xr_end_frame_proc(get_proc_addr);
        let _ = resolve_named_proc(get_proc_addr, "xrCreateInstance", |fns, addr| {
            fns.create_instance = addr;
            fns.create_instance_resolved = addr != 0;
        });
        let _ = resolve_named_proc(get_proc_addr, "xrCreateSession", |fns, addr| {
            fns.create_session = addr;
            fns.create_session_resolved = addr != 0;
        });
        let _ = resolve_named_proc(get_proc_addr, "xrGetSystem", |fns, addr| {
            fns.get_system = addr;
            fns.get_system_resolved = addr != 0;
        });
        let _ = resolve_named_proc(get_proc_addr, "xrDestroyInstance", |fns, addr| {
            fns.destroy_instance = addr;
        });
        let _ = resolve_named_proc(get_proc_addr, "xrDestroySession", |fns, addr| {
            fns.destroy_session = addr;
        });

        if runtime_ok {
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

#[cfg(not(target_arch = "wasm32"))]
fn resolve_xr_end_frame_proc(get_proc_addr: usize) -> bool {
    resolve_named_proc(get_proc_addr, "xrEndFrame", |fns, addr| {
        fns.end_frame = addr;
        fns.end_frame_resolved = addr != 0;
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_named_proc(
    get_proc_addr: usize,
    name: &str,
    store: impl FnOnce(&mut OpenXrLoaderFns, usize),
) -> bool {
    use std::ffi::CString;
    if get_proc_addr == 0 {
        return false;
    }
    let get_proc: XrGetInstanceProcAddr = unsafe { std::mem::transmute(get_proc_addr) };
    let mut fn_ptr: *const std::os::raw::c_void = std::ptr::null();
    let cname = CString::new(name).unwrap();
    let rc = unsafe { get_proc(0, cname.as_ptr(), &mut fn_ptr) };
    let resolved = rc == 0 && !fn_ptr.is_null();
    let addr = if resolved { fn_ptr as usize } else { 0 };
    if let Ok(mut fns) = OPENXR_FNS.lock() {
        store(&mut fns, addr);
    }
    resolved
}

#[cfg(target_arch = "wasm32")]
fn resolve_xr_end_frame_proc(_get_proc_addr: usize) -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
fn probe_openxr_loader() -> (bool, bool, String, String) {
    (
        false,
        false,
        String::new(),
        "openxr not available on wasm32".into(),
    )
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

fn detect_vendor(openxr_runtime: bool, webxr: bool, stub: bool) -> (String, String, bool) {
    if let Some(v) = env_vendor_override() {
        return (v, "headset".into(), true);
    }
    if stub {
        return ("kabootar-stub-hmd".into(), "headset".into(), true);
    }
    if openxr_runtime {
        return ("openxr-runtime".into(), "headset".into(), true);
    }
    if webxr {
        return ("webxr".into(), "headset".into(), true);
    }
    ("none".into(), "none".into(), false)
}

/// Refresh FFI probe results into process state (does not require a headset).
pub fn probe() -> XrFfiStatus {
    let stub = std::env::var("KABOOTAR_XR_STUB")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let (openxr_loader, openxr_runtime, loader_path, ox_detail) = probe_openxr_loader();
    let (webxr, wx_detail) = probe_webxr();
    let (vendor, form_factor, hmd_connected) = detect_vendor(openxr_runtime, webxr, stub);
    let backend = if stub {
        "xr-stub"
    } else if openxr_runtime {
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
    } else if stub {
        "stub HMD driver".into()
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
        hmd_connected,
        vendor,
        form_factor,
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
        st.hmd_connected = true;
        st.vendor = env_vendor_override().unwrap_or_else(|| "kabootar-stub-hmd".into());
        st.form_factor = "headset".into();
    } else if st.openxr_runtime || st.webxr {
        st.bound = true;
        if st.openxr_runtime {
            st.backend = "openxr-ffi".into();
        } else {
            st.backend = "webxr-ffi".into();
        }
        st.hmd_connected = true;
    } else if st.openxr_loader {
        st.bound = true;
        st.backend = "openxr-loader".into();
        st.detail = format!("{} (no active runtime/HMD)", st.detail);
        st.hmd_connected = false;
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

/// Submit a composed HMD frame to the vendor/stub driver.
pub fn present_to_hmd(composition: &Value) -> Result<Value, String> {
    let st = status();
    let stub = std::env::var("KABOOTAR_XR_STUB")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    if !st.bound && !stub {
        return Err("xr_hmd_driver_present: headset not bound (call xr_bind_headset)".into());
    }

    let (view_count, sbs_w, sbs_h, frame_hint) = match composition {
        Value::Object(m) => {
            let vc = match m.get("viewCount") {
                Some(Value::Number(n)) => *n,
                _ => 0,
            };
            let w = match m.get("sideBySideWidth") {
                Some(Value::Number(n)) => *n,
                _ => 0,
            };
            let h = match m.get("sideBySideHeight") {
                Some(Value::Number(n)) => *n,
                _ => 0,
            };
            (vc, w, h, m.get("frameIndex").cloned())
        }
        _ => (0, 0, 0, None),
    };

    let mut driver = HMD_DRIVER
        .lock()
        .map_err(|_| "hmd driver lock poisoned".to_string())?;
    driver.presents += 1;
    driver.last_view_count = view_count;
    if let Some(Value::Number(n)) = frame_hint {
        driver.last_frame_index = n;
    } else {
        driver.last_frame_index = driver.presents;
    }

    let driver_name = if stub || st.backend == "xr-stub" {
        "stub-hmd-driver".to_string()
    } else if st.openxr_runtime {
        format!("openxr-vendor:{}", st.vendor)
    } else if st.webxr {
        "webxr-session".to_string()
    } else if st.openxr_loader {
        "openxr-loader-queue".to_string()
    } else {
        "descriptor-hmd".to_string()
    };
    driver.last_backend = driver_name.clone();

    let presented = stub || st.hmd_connected || st.openxr_runtime || st.webxr || st.bound;

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("xr_hmd_driver_present".into()));
    out.insert("presented".into(), Value::Bool(presented));
    out.insert("driver".into(), Value::String(driver_name));
    out.insert("vendor".into(), Value::String(st.vendor.clone()));
    out.insert("formFactor".into(), Value::String(st.form_factor.clone()));
    out.insert("hmdConnected".into(), Value::Bool(st.hmd_connected || stub));
    out.insert("presentCount".into(), Value::Number(driver.presents));
    out.insert("frameIndex".into(), Value::Number(driver.last_frame_index));
    out.insert("viewCount".into(), Value::Number(view_count));
    out.insert("sideBySideWidth".into(), Value::Number(sbs_w));
    out.insert("sideBySideHeight".into(), Value::Number(sbs_h));
    out.insert(
        "layerType".into(),
        Value::String("COMPOSITION_LAYER_PROJECTION".into()),
    );
    // Also hand off to compositor IPC (OpenXR xrEndFrame → compositor).
    let _ = compositor_submit(composition);
    Ok(Value::Object(out))
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
    out.insert("hmdConnected".into(), Value::Bool(s.hmd_connected));
    out.insert("vendor".into(), Value::String(s.vendor));
    out.insert("formFactor".into(), Value::String(s.form_factor));
    Value::Object(out)
}

fn stub_enabled() -> bool {
    std::env::var("KABOOTAR_XR_STUB")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn live_env_forced() -> bool {
    std::env::var("KABOOTAR_XR_LIVE")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Preferred graphics API for OpenXR session binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphicsApiKind {
    Vulkan,
    D3D11,
    Stub,
}

fn detect_graphics_api(force_stub_default: bool) -> GraphicsApiKind {
    if let Ok(v) = std::env::var("KABOOTAR_XR_GRAPHICS") {
        let s = v.to_ascii_lowercase();
        if s == "vulkan" || s == "vk" {
            return GraphicsApiKind::Vulkan;
        }
        if s == "d3d11" || s == "dx11" || s == "d3d" {
            return GraphicsApiKind::D3D11;
        }
        if s == "stub" || s == "kabootar" {
            return GraphicsApiKind::Stub;
        }
        if s == "wgpu" {
            // wgpu interop rides on Vulkan (or D3D11 on Windows) OpenXR binding.
            #[cfg(target_os = "windows")]
            {
                return GraphicsApiKind::D3D11;
            }
            #[cfg(not(target_os = "windows"))]
            {
                return GraphicsApiKind::Vulkan;
            }
        }
    }
    // Prefer wgpu when a GPU 3D path is already up (or XR wgpu interop is forced).
    if wgpu_interop_requested() {
        #[cfg(target_os = "windows")]
        {
            return GraphicsApiKind::D3D11;
        }
        #[cfg(not(target_os = "windows"))]
        {
            return GraphicsApiKind::Vulkan;
        }
    }
    if force_stub_default {
        return GraphicsApiKind::Stub;
    }
    #[cfg(target_os = "windows")]
    {
        GraphicsApiKind::D3D11
    }
    #[cfg(not(target_os = "windows"))]
    {
        GraphicsApiKind::Vulkan
    }
}

fn wgpu_interop_requested() -> bool {
    if std::env::var("KABOOTAR_XR_WGPU")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("wgpu"))
    {
        return true;
    }
    if std::env::var("KABOOTAR_XR_GRAPHICS")
        .ok()
        .is_some_and(|v| v.eq_ignore_ascii_case("wgpu"))
    {
        return true;
    }
    crate::runtime::render::gpu3d::gpu3d_available()
}

/// Resolve a wgpu (or wgpu-interop) device handle for OpenXR graphics binding.
fn resolve_wgpu_device_handle() -> Option<(u64, String)> {
    if !wgpu_interop_requested() {
        return None;
    }
    #[cfg(feature = "gpu")]
    {
        crate::runtime::render::probe_gpu();
        if crate::runtime::render::gpu_available() {
            // Stable opaque handle: Kabootar wgpu device slot (not a raw VkDevice*).
            return Some((0x57_0000_1001, "wgpu-device".into()));
        }
    }
    // Stub / CI: synthetic wgpu interop device (feeds OpenXR Vulkan/D3D11 binding).
    Some((0x57_0000_0001, "wgpu-interop".into()))
}

fn session_reject_requested() -> bool {
    std::env::var("KABOOTAR_XR_SESSION_REJECT")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn session_reject_reason() -> String {
    std::env::var("KABOOTAR_XR_SESSION_REJECT_REASON")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "NotSupportedError".into())
}

fn graphics_api_name(kind: GraphicsApiKind) -> &'static str {
    match kind {
        GraphicsApiKind::Vulkan => "vulkan",
        GraphicsApiKind::D3D11 => "d3d11",
        GraphicsApiKind::Stub => "stub",
    }
}

fn graphics_binding_type_name(kind: GraphicsApiKind) -> &'static str {
    match kind {
        GraphicsApiKind::Vulkan => "XR_TYPE_GRAPHICS_BINDING_VULKAN_KHR",
        GraphicsApiKind::D3D11 => "XR_TYPE_GRAPHICS_BINDING_D3D11_KHR",
        GraphicsApiKind::Stub => "XR_TYPE_GRAPHICS_BINDING_KABOOTAR",
    }
}

#[derive(Debug, Clone)]
struct OpenXrCreateResult {
    instance: u64,
    session: u64,
    system_id: u64,
    rc_inst: i32,
    rc_get_system: i32,
    rc_sess: i32,
    path: String,
    instance_created: bool,
    session_created: bool,
    system_enumerated: bool,
    graphics_bound: bool,
    graphics_api: String,
    graphics_device: u64,
    graphics_binding_type: String,
    wgpu_bound: bool,
    wgpu_device: u64,
}

/// Run xrCreateInstance → xrGetSystem → graphics binding → xrCreateSession.
fn openxr_create_instance_and_session(use_stub: bool) -> OpenXrCreateResult {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = use_stub;
        return OpenXrCreateResult {
            instance: 0,
            session: 0,
            system_id: 0,
            rc_inst: 0,
            rc_get_system: 0,
            rc_sess: 0,
            path: "webxr-no-openxr-create".into(),
            instance_created: false,
            session_created: false,
            system_enumerated: false,
            graphics_bound: false,
            graphics_api: String::new(),
            graphics_device: 0,
            graphics_binding_type: String::new(),
            wgpu_bound: false,
            wgpu_device: 0,
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Ensure stub procs exist when loader did not resolve create APIs.
        if let Ok(mut fns) = OPENXR_FNS.lock() {
            if use_stub || !fns.create_instance_resolved || fns.create_instance == 0 {
                fns.create_instance = stub_xr_create_instance_addr();
                fns.create_instance_resolved = fns.create_instance != 0;
            }
            if use_stub || !fns.create_session_resolved || fns.create_session == 0 {
                fns.create_session = stub_xr_create_session_addr();
                fns.create_session_resolved = fns.create_session != 0;
            }
            if use_stub || !fns.get_system_resolved || fns.get_system == 0 {
                fns.get_system = stub_xr_get_system_addr();
                fns.get_system_resolved = fns.get_system != 0;
            }
            if fns.destroy_instance == 0 {
                fns.destroy_instance = stub_xr_destroy_instance_addr();
            }
            if fns.destroy_session == 0 {
                fns.destroy_session = stub_xr_destroy_session_addr();
            }
            if !fns.end_frame_resolved || fns.end_frame == 0 {
                let addr = stub_xr_end_frame_addr();
                if addr != 0 {
                    fns.end_frame = addr;
                    fns.end_frame_resolved = true;
                }
            }
        }

        let (create_inst_addr, create_sess_addr, get_system_addr, path, force_stub) = {
            let fns = match OPENXR_FNS.lock() {
                Ok(f) => f,
                Err(_) => {
                    return OpenXrCreateResult {
                        instance: 0,
                        session: 0,
                        system_id: 0,
                        rc_inst: -1,
                        rc_get_system: -1,
                        rc_sess: -1,
                        path: "fns-lock-failed".into(),
                        instance_created: false,
                        session_created: false,
                        system_enumerated: false,
                        graphics_bound: false,
                        graphics_api: String::new(),
                        graphics_device: 0,
                        graphics_binding_type: String::new(),
                        wgpu_bound: false,
                        wgpu_device: 0,
                    };
                }
            };
            let force_stub = use_stub
                || !fns.create_instance_resolved
                || fns.create_instance == stub_xr_create_instance_addr();
            let path = if force_stub {
                "stub-xrCreateInstance+xrGetSystem+graphicsBinding+xrCreateSession".to_string()
            } else {
                "openxr-xrCreateInstance+xrGetSystem+graphicsBinding+xrCreateSession".to_string()
            };
            let ci = if force_stub {
                stub_xr_create_instance_addr()
            } else {
                fns.create_instance
            };
            let cs = if force_stub {
                stub_xr_create_session_addr()
            } else {
                fns.create_session
            };
            let gs = if force_stub || fns.get_system == 0 {
                stub_xr_get_system_addr()
            } else {
                fns.get_system
            };
            (ci, cs, gs, path, force_stub)
        };

        if create_inst_addr == 0 || create_sess_addr == 0 || get_system_addr == 0 {
            return OpenXrCreateResult {
                instance: 0,
                session: 0,
                system_id: 0,
                rc_inst: -1,
                rc_get_system: -1,
                rc_sess: -1,
                path: "missing-create-procs".into(),
                instance_created: false,
                session_created: false,
                system_enumerated: false,
                graphics_bound: false,
                graphics_api: String::new(),
                graphics_device: 0,
                graphics_binding_type: String::new(),
                wgpu_bound: false,
                wgpu_device: 0,
            };
        }

        let mut app = XrApplicationInfo {
            application_name: [0; XR_MAX_APPLICATION_NAME_SIZE],
            application_version: 1,
            engine_name: [0; XR_MAX_ENGINE_NAME_SIZE],
            engine_version: 1,
            api_version: (1u64 << 48) | (0u64 << 32),
        };
        let app_name = b"Kabootar";
        let eng_name = b"KabootarXR";
        app.application_name[..app_name.len()].copy_from_slice(app_name);
        app.engine_name[..eng_name.len()].copy_from_slice(eng_name);

        let inst_info = XrInstanceCreateInfo {
            ty: XR_TYPE_INSTANCE_CREATE_INFO,
            next: std::ptr::null(),
            create_flags: 0,
            application_info: app,
            enabled_api_layer_count: 0,
            enabled_api_layer_names: std::ptr::null(),
            enabled_extension_count: 0,
            enabled_extension_names: std::ptr::null(),
        };

        let create_instance: XrCreateInstanceFn =
            unsafe { std::mem::transmute(create_inst_addr) };
        let mut instance: u64 = 0;
        let rc_inst = unsafe { create_instance(&inst_info, &mut instance) };

        let mut system_id: u64 = 0;
        let mut rc_get_system: i32 = -1;
        let mut system_enumerated = false;
        let mut session: u64 = 0;
        let mut rc_sess: i32 = -1;
        let mut session_ok = false;
        let mut graphics_bound = false;
        let mut graphics_device: u64 = 0;
        let mut wgpu_bound = false;
        let mut wgpu_device: u64 = 0;
        let gfx_kind = detect_graphics_api(force_stub);
        let graphics_api = graphics_api_name(gfx_kind).to_string();
        let graphics_binding_type = graphics_binding_type_name(gfx_kind).to_string();
        let wgpu_dev = resolve_wgpu_device_handle();

        if rc_inst == 0 && instance != 0 {
            let sys_info = XrSystemGetInfo {
                ty: XR_TYPE_SYSTEM_GET_INFO,
                next: std::ptr::null(),
                form_factor: XR_FORM_FACTOR_HEAD_MOUNTED_DISPLAY,
            };
            let get_system: XrGetSystemFn = unsafe { std::mem::transmute(get_system_addr) };
            rc_get_system = unsafe { get_system(instance, &sys_info, &mut system_id) };
            system_enumerated = rc_get_system == 0 && system_id != 0;

            if system_enumerated {
                let create_session: XrCreateSessionFn =
                    unsafe { std::mem::transmute(create_sess_addr) };
                match gfx_kind {
                    GraphicsApiKind::Vulkan => {
                        let device = wgpu_dev
                            .as_ref()
                            .map(|(d, _)| *d)
                            .unwrap_or(0x76_0000_0003);
                        let gfx = XrGraphicsBindingVulkanKHR {
                            ty: XR_TYPE_GRAPHICS_BINDING_VULKAN_KHR,
                            next: std::ptr::null(),
                            instance: 0x76_0000_0001, // synthetic VkInstance
                            physical_device: 0x76_0000_0002,
                            device,
                            queue_family_index: 0,
                            queue_index: 0,
                        };
                        graphics_device = gfx.device;
                        graphics_bound = true;
                        if let Some((d, _)) = &wgpu_dev {
                            wgpu_bound = true;
                            wgpu_device = *d;
                        }
                        let sess_info = XrSessionCreateInfo {
                            ty: XR_TYPE_SESSION_CREATE_INFO,
                            next: &gfx as *const _ as *const std::os::raw::c_void,
                            create_flags: 0,
                            system_id,
                        };
                        rc_sess = unsafe { create_session(instance, &sess_info, &mut session) };
                    }
                    GraphicsApiKind::D3D11 => {
                        let device = wgpu_dev
                            .as_ref()
                            .map(|(d, _)| *d)
                            .unwrap_or(0x11_0000_0001);
                        let gfx = XrGraphicsBindingD3D11KHR {
                            ty: XR_TYPE_GRAPHICS_BINDING_D3D11_KHR,
                            next: std::ptr::null(),
                            device,
                        };
                        graphics_device = gfx.device;
                        graphics_bound = true;
                        if let Some((d, _)) = &wgpu_dev {
                            wgpu_bound = true;
                            wgpu_device = *d;
                        }
                        let sess_info = XrSessionCreateInfo {
                            ty: XR_TYPE_SESSION_CREATE_INFO,
                            next: &gfx as *const _ as *const std::os::raw::c_void,
                            create_flags: 0,
                            system_id,
                        };
                        rc_sess = unsafe { create_session(instance, &sess_info, &mut session) };
                    }
                    GraphicsApiKind::Stub => {
                        let device = wgpu_dev
                            .as_ref()
                            .map(|(d, _)| *d)
                            .unwrap_or(0x6B_0000_0001);
                        let gfx = XrGraphicsBindingKabootar {
                            ty: XR_TYPE_GRAPHICS_BINDING_KABOOTAR,
                            next: std::ptr::null(),
                            api: if wgpu_dev.is_some() { 1 } else { 4 },
                            device,
                        };
                        graphics_device = gfx.device;
                        graphics_bound = true;
                        if let Some((d, _)) = &wgpu_dev {
                            wgpu_bound = true;
                            wgpu_device = *d;
                        }
                        let sess_info = XrSessionCreateInfo {
                            ty: XR_TYPE_SESSION_CREATE_INFO,
                            next: &gfx as *const _ as *const std::os::raw::c_void,
                            create_flags: 0,
                            system_id,
                        };
                        rc_sess = unsafe { create_session(instance, &sess_info, &mut session) };
                    }
                }
                session_ok = rc_sess == 0 && session != 0;
            }

            if let Ok(fns) = OPENXR_FNS.lock() {
                let gp = fns.get_proc;
                drop(fns);
                if gp != 0 && !force_stub {
                    let _ = resolve_xr_end_frame_proc(gp);
                }
            }
        }

        OpenXrCreateResult {
            instance,
            session,
            system_id,
            rc_inst,
            rc_get_system,
            rc_sess,
            path,
            instance_created: rc_inst == 0 && instance != 0,
            session_created: session_ok,
            system_enumerated,
            graphics_bound,
            graphics_api,
            graphics_device,
            graphics_binding_type,
            wgpu_bound,
            wgpu_device,
        }
    }
}

/// Create a live OpenXR/WebXR session via xrCreateInstance/xrCreateSession (or stub).
pub fn create_live_session(mode: &str) -> Result<Value, String> {
    let mode = if mode.is_empty() { "immersive-vr" } else { mode };
    let stub = stub_enabled();
    let st = status();
    if !st.bound && !stub && !live_env_forced() {
        return Err("xr_create_live_session: headset not bound".into());
    }

    let use_stub = stub || st.backend == "xr-stub" || !(st.openxr_loader || st.openxr_runtime);
    let created = if st.webxr && !stub {
        OpenXrCreateResult {
            instance: 0,
            session: 0,
            system_id: 0,
            rc_inst: 0,
            rc_get_system: 0,
            rc_sess: 0,
            path: "webxr-live".to_string(),
            instance_created: false,
            session_created: false,
            system_enumerated: false,
            graphics_bound: false,
            graphics_api: "webxr".into(),
            graphics_device: 0,
            graphics_binding_type: String::new(),
            wgpu_bound: false,
            wgpu_device: 0,
        }
    } else {
        openxr_create_instance_and_session(use_stub || stub)
    };

    let (backend, webxr_granted) = if stub || st.backend == "xr-stub" {
        ("stub-live".to_string(), false)
    } else if st.webxr {
        ("webxr-live".to_string(), true)
    } else if created.instance_created && created.session_created {
        ("openxr-created".to_string(), false)
    } else if st.openxr_runtime || st.openxr_loader {
        ("openxr-live".to_string(), false)
    } else {
        ("descriptor-live".to_string(), false)
    };

    // Ensure stub trampoline is available when loader did not resolve xrEndFrame.
    if let Ok(mut fns) = OPENXR_FNS.lock() {
        if !fns.end_frame_resolved || fns.end_frame == 0 {
            let addr = stub_xr_end_frame_addr();
            if addr != 0 {
                fns.end_frame = addr;
                fns.end_frame_resolved = true;
                if fns.last_path.is_empty() {
                    fns.last_path = "stub-trampoline".into();
                }
            }
        }
    }

    let mut live = LIVE_SESSION
        .lock()
        .map_err(|_| "live session lock poisoned".to_string())?;
    live.active = true;
    live.mode = mode.into();
    live.instance = created.instance;
    live.session = created.session;
    live.system_id = created.system_id;
    live.backend = backend.clone();
    live.webxr_granted = webxr_granted;
    live.webxr_raf_bound = false;
    live.end_frame_ffi_calls = 0;
    live.last_ffi_result = 0;
    live.last_ffi_mode.clear();
    live.instance_created = created.instance_created;
    live.session_created = created.session_created;
    live.system_enumerated = created.system_enumerated;
    live.graphics_bound = created.graphics_bound;
    live.graphics_api = created.graphics_api.clone();
    live.graphics_device = created.graphics_device;
    live.graphics_binding_type = created.graphics_binding_type.clone();
    live.swapchain_graphics_bound = false;
    live.wgpu_bound = created.wgpu_bound;
    live.wgpu_device = created.wgpu_device;
    live.create_instance_rc = created.rc_inst;
    live.get_system_rc = created.rc_get_system;
    live.create_session_rc = created.rc_sess;
    live.create_path = created.path.clone();

    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("active".into(), Value::Bool(true));
    out.insert("mode".into(), Value::String(mode.into()));
    out.insert("backend".into(), Value::String(backend));
    out.insert("instance".into(), Value::Number(created.instance as i64));
    out.insert("session".into(), Value::Number(created.session as i64));
    out.insert("systemId".into(), Value::Number(created.system_id as i64));
    out.insert("webxrGranted".into(), Value::Bool(webxr_granted));
    out.insert(
        "instanceCreated".into(),
        Value::Bool(created.instance_created),
    );
    out.insert(
        "sessionCreated".into(),
        Value::Bool(created.session_created),
    );
    out.insert(
        "systemEnumerated".into(),
        Value::Bool(created.system_enumerated),
    );
    out.insert("graphicsBound".into(), Value::Bool(created.graphics_bound));
    out.insert(
        "graphicsApi".into(),
        Value::String(created.graphics_api),
    );
    out.insert(
        "graphicsDevice".into(),
        Value::Number(created.graphics_device as i64),
    );
    out.insert(
        "graphicsBindingType".into(),
        Value::String(created.graphics_binding_type),
    );
    out.insert("wgpuBound".into(), Value::Bool(created.wgpu_bound));
    out.insert(
        "wgpuDevice".into(),
        Value::Number(created.wgpu_device as i64),
    );
    out.insert(
        "createInstanceRc".into(),
        Value::Number(created.rc_inst as i64),
    );
    out.insert(
        "getSystemRc".into(),
        Value::Number(created.rc_get_system as i64),
    );
    out.insert(
        "createSessionRc".into(),
        Value::Number(created.rc_sess as i64),
    );
    out.insert("createPath".into(), Value::String(created.path));
    out.insert("kind".into(), Value::String("xr_live_session".into()));
    Ok(Value::Object(out))
}

pub fn destroy_live_session() -> Result<Value, String> {
    let (instance, session, destroy_inst, destroy_sess) = {
        let live = LIVE_SESSION
            .lock()
            .map_err(|_| "live session lock poisoned".to_string())?;
        let (di, ds) = match OPENXR_FNS.lock() {
            Ok(f) => (f.destroy_instance, f.destroy_session),
            Err(_) => (0, 0),
        };
        (live.instance, live.session, di, ds)
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        if session != 0 && destroy_sess != 0 {
            let destroy: XrDestroySessionFn = unsafe { std::mem::transmute(destroy_sess) };
            let _ = unsafe { destroy(session) };
        }
        if instance != 0 && destroy_inst != 0 {
            let destroy: XrDestroyInstanceFn = unsafe { std::mem::transmute(destroy_inst) };
            let _ = unsafe { destroy(instance) };
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (destroy_inst, destroy_sess, instance, session);
    }

    let mut live = LIVE_SESSION
        .lock()
        .map_err(|_| "live session lock poisoned".to_string())?;
    let calls = live.end_frame_ffi_calls;
    let create_path = live.create_path.clone();
    live.active = false;
    live.webxr_granted = false;
    live.webxr_raf_bound = false;
    live.instance = 0;
    live.session = 0;
    live.system_id = 0;
    live.instance_created = false;
    live.session_created = false;
    live.system_enumerated = false;
    live.graphics_bound = false;
    live.graphics_api.clear();
    live.graphics_device = 0;
    live.graphics_binding_type.clear();
    live.swapchain_graphics_bound = false;
    live.wgpu_bound = false;
    live.wgpu_device = 0;
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("active".into(), Value::Bool(false));
    out.insert("endFrameFfiCalls".into(), Value::Number(calls));
    out.insert("createPath".into(), Value::String(create_path));
    out.insert("kind".into(), Value::String("xr_live_session".into()));
    Ok(Value::Object(out))
}

pub fn live_session_status() -> Value {
    let mut out = HashMap::new();
    match LIVE_SESSION.lock() {
        Ok(live) => {
            out.insert("active".into(), Value::Bool(live.active));
            out.insert("mode".into(), Value::String(live.mode.clone()));
            out.insert("backend".into(), Value::String(live.backend.clone()));
            out.insert("instance".into(), Value::Number(live.instance as i64));
            out.insert("session".into(), Value::Number(live.session as i64));
            out.insert("systemId".into(), Value::Number(live.system_id as i64));
            out.insert(
                "endFrameFfiCalls".into(),
                Value::Number(live.end_frame_ffi_calls),
            );
            out.insert(
                "lastFfiResult".into(),
                Value::Number(live.last_ffi_result as i64),
            );
            out.insert(
                "lastFfiMode".into(),
                Value::String(live.last_ffi_mode.clone()),
            );
            out.insert("webxrGranted".into(), Value::Bool(live.webxr_granted));
            out.insert("webxrRafBound".into(), Value::Bool(live.webxr_raf_bound));
            out.insert("instanceCreated".into(), Value::Bool(live.instance_created));
            out.insert("sessionCreated".into(), Value::Bool(live.session_created));
            out.insert(
                "systemEnumerated".into(),
                Value::Bool(live.system_enumerated),
            );
            out.insert("graphicsBound".into(), Value::Bool(live.graphics_bound));
            out.insert(
                "graphicsApi".into(),
                Value::String(live.graphics_api.clone()),
            );
            out.insert(
                "graphicsDevice".into(),
                Value::Number(live.graphics_device as i64),
            );
            out.insert(
                "graphicsBindingType".into(),
                Value::String(live.graphics_binding_type.clone()),
            );
            out.insert(
                "swapchainGraphicsBound".into(),
                Value::Bool(live.swapchain_graphics_bound),
            );
            out.insert("wgpuBound".into(), Value::Bool(live.wgpu_bound));
            out.insert(
                "wgpuDevice".into(),
                Value::Number(live.wgpu_device as i64),
            );
            out.insert(
                "createInstanceRc".into(),
                Value::Number(live.create_instance_rc as i64),
            );
            out.insert(
                "getSystemRc".into(),
                Value::Number(live.get_system_rc as i64),
            );
            out.insert(
                "createSessionRc".into(),
                Value::Number(live.create_session_rc as i64),
            );
            out.insert("createPath".into(), Value::String(live.create_path.clone()));
        }
        Err(_) => {
            out.insert("active".into(), Value::Bool(false));
        }
    }
    out.insert("kind".into(), Value::String("xr_live_session".into()));
    Value::Object(out)
}

/// Attach live-session graphics binding to an HMD swapchain descriptor (Vulkan/D3D11/stub).
pub fn bind_hmd_swapchain(width: i64, height: i64, eye: &str) -> Result<Value, String> {
    let live = LIVE_SESSION
        .lock()
        .map_err(|_| "live session lock poisoned".to_string())?;
    if !live.active {
        return Err("xr_bind_hmd_swapchain: no live session".into());
    }
    if !live.graphics_bound {
        return Err("xr_bind_hmd_swapchain: graphics not bound on session".into());
    }
    let w = if width < 64 { 1280 } else { width };
    let h = if height < 64 { 720 } else { height };
    let eye = if eye.is_empty() { "left" } else { eye };
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("eye".into(), Value::String(eye.into()));
    out.insert("width".into(), Value::Number(w));
    out.insert("height".into(), Value::Number(h));
    out.insert("imageCount".into(), Value::Number(3));
    out.insert("format".into(), Value::String("rgba8".into()));
    out.insert(
        "graphicsApi".into(),
        Value::String(live.graphics_api.clone()),
    );
    out.insert(
        "graphicsDevice".into(),
        Value::Number(live.graphics_device as i64),
    );
    out.insert(
        "graphicsBindingType".into(),
        Value::String(live.graphics_binding_type.clone()),
    );
    out.insert("hmdSwapchain".into(), Value::Bool(true));
    out.insert("kind".into(), Value::String("xr_hmd_swapchain_desc".into()));
    if live.wgpu_bound {
        out.insert("wgpuBound".into(), Value::Bool(true));
        out.insert(
            "wgpuDevice".into(),
            Value::Number(live.wgpu_device as i64),
        );
    }
    Ok(Value::Object(out))
}

pub fn mark_swapchain_graphics_bound() -> Result<Value, String> {
    let mut live = LIVE_SESSION
        .lock()
        .map_err(|_| "live session lock poisoned".to_string())?;
    live.swapchain_graphics_bound = live.graphics_bound;
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert(
        "swapchainGraphicsBound".into(),
        Value::Bool(live.swapchain_graphics_bound),
    );
    out.insert(
        "graphicsApi".into(),
        Value::String(live.graphics_api.clone()),
    );
    out.insert("wgpuBound".into(), Value::Bool(live.wgpu_bound));
    out.insert(
        "wgpuDevice".into(),
        Value::Number(live.wgpu_device as i64),
    );
    Ok(Value::Object(out))
}

/// Allocate GPU-backed OpenXR swapchain images (wgpu textures when available).
pub fn allocate_gpu_swapchain_images(
    swapchain_id: i64,
    width: u32,
    height: u32,
    image_count: i64,
    eye: &str,
) -> Result<Value, String> {
    let live = LIVE_SESSION
        .lock()
        .map_err(|_| "live session lock poisoned".to_string())?;
    if !live.active || !live.graphics_bound {
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(false));
        out.insert("gpuBound".into(), Value::Bool(false));
        return Ok(Value::Object(out));
    }
    let graphics_api = live.graphics_api.clone();
    let graphics_device = live.graphics_device;
    let wgpu_bound = live.wgpu_bound || wgpu_interop_requested();
    let wgpu_device = if live.wgpu_device != 0 {
        live.wgpu_device
    } else if wgpu_bound {
        resolve_wgpu_device_handle()
            .map(|(d, _)| d)
            .unwrap_or(graphics_device)
    } else {
        0
    };
    drop(live);

    let count = image_count.clamp(1, 4) as usize;
    let mut native_images = Vec::with_capacity(count);
    let mut wgpu_texture_ids = Vec::with_capacity(count);
    let image_source = if wgpu_bound {
        "wgpu".to_string()
    } else {
        graphics_api.clone()
    };

    #[cfg(feature = "gpu")]
    {
        if wgpu_bound {
            crate::runtime::render::probe_gpu();
            if crate::runtime::render::gpu_available() {
                let w = width.max(64);
                let h = height.max(64);
                let pixels = vec![0u8; (w as usize) * (h as usize) * 4];
                let mut ids = Vec::with_capacity(count);
                let mut ok = true;
                for _ in 0..count {
                    match crate::runtime::render::upload_rgba(w, h, &pixels) {
                        Ok(id) => ids.push(id),
                        Err(_) => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    wgpu_texture_ids = ids;
                }
            }
        }
    }

    for i in 0..count {
        let native = {
            let mut n = NEXT_NATIVE_IMAGE
                .lock()
                .map_err(|_| "native image lock poisoned".to_string())?;
            let id = *n;
            *n += 1;
            id
        };
        native_images.push(native);
        if wgpu_texture_ids.len() <= i {
            let tid = {
                let mut n = NEXT_WGPU_TEXTURE_ID
                    .lock()
                    .map_err(|_| "wgpu texture id lock poisoned".to_string())?;
                let id = *n;
                *n += 1;
                id
            };
            wgpu_texture_ids.push(tid);
        }
        if let Ok(mut pool) = GPU_SWAPCHAIN_IMAGES.lock() {
            pool.push(GpuSwapchainImage {
                swapchain_id,
                image_index: i as i64,
                native_image: native,
                wgpu_texture_id: wgpu_texture_ids[i],
                width,
                height,
                eye: eye.into(),
            });
        }
    }

    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("gpuBound".into(), Value::Bool(true));
    out.insert("swapchainId".into(), Value::Number(swapchain_id));
    out.insert("imageCount".into(), Value::Number(count as i64));
    out.insert("graphicsApi".into(), Value::String(graphics_api));
    out.insert(
        "graphicsDevice".into(),
        Value::Number(graphics_device as i64),
    );
    out.insert("wgpuBound".into(), Value::Bool(wgpu_bound));
    out.insert("wgpuDevice".into(), Value::Number(wgpu_device as i64));
    out.insert("imageSource".into(), Value::String(image_source));
    out.insert(
        "nativeImages".into(),
        Value::Array(
            native_images
                .into_iter()
                .map(|n| Value::Number(n as i64))
                .collect(),
        ),
    );
    out.insert(
        "wgpuTextureIds".into(),
        Value::Array(
            wgpu_texture_ids
                .into_iter()
                .map(|n| Value::Number(n as i64))
                .collect(),
        ),
    );
    Ok(Value::Object(out))
}

/// Attach live GPU device / wgpu texture handles onto an acquired swapchain image.
pub fn attach_swapchain_image_gpu(
    swapchain_id: i64,
    image_index: i64,
) -> Result<HashMap<String, Value>, String> {
    let mut out = HashMap::new();
    let live = LIVE_SESSION
        .lock()
        .map_err(|_| "live session lock poisoned".to_string())?;
    if !live.graphics_bound {
        out.insert("gpuBound".into(), Value::Bool(false));
        return Ok(out);
    }
    out.insert("gpuBound".into(), Value::Bool(true));
    out.insert(
        "graphicsApi".into(),
        Value::String(live.graphics_api.clone()),
    );
    out.insert(
        "graphicsDevice".into(),
        Value::Number(live.graphics_device as i64),
    );
    out.insert("wgpuBound".into(), Value::Bool(live.wgpu_bound));
    out.insert(
        "wgpuDevice".into(),
        Value::Number(live.wgpu_device as i64),
    );
    let source = if live.wgpu_bound {
        "wgpu"
    } else {
        live.graphics_api.as_str()
    };
    out.insert("imageSource".into(), Value::String(source.into()));
    drop(live);

    if let Ok(pool) = GPU_SWAPCHAIN_IMAGES.lock() {
        if let Some(img) = pool
            .iter()
            .find(|i| i.swapchain_id == swapchain_id && i.image_index == image_index)
        {
            out.insert(
                "nativeImage".into(),
                Value::Number(img.native_image as i64),
            );
            out.insert(
                "wgpuTextureId".into(),
                Value::Number(img.wgpu_texture_id as i64),
            );
            return Ok(out);
        }
    }
    // Lazy synthetic handles if allocate was skipped.
    let native = {
        let mut n = NEXT_NATIVE_IMAGE
            .lock()
            .map_err(|_| "native image lock poisoned".to_string())?;
        let id = *n;
        *n += 1;
        id
    };
    let tid = {
        let mut n = NEXT_WGPU_TEXTURE_ID
            .lock()
            .map_err(|_| "wgpu texture id lock poisoned".to_string())?;
        let id = *n;
        *n += 1;
        id
    };
    out.insert("nativeImage".into(), Value::Number(native as i64));
    out.insert("wgpuTextureId".into(), Value::Number(tid as i64));
    Ok(out)
}

/// Start requestSession as a Kab `Promise` (for `promise_then` / browser then-chains).
pub fn start_session_promise_value(
    mode: &str,
    env: &mut crate::value::Environment,
) -> Result<Value, String> {
    use crate::runtime::stdlib::promise::settle_promise;
    use crate::value::PromiseValue;
    use std::cell::RefCell;
    use std::rc::Rc;

    let promise = Rc::new(RefCell::new(PromiseValue::Pending));
    if session_reject_requested() {
        let reason = reject_session_promise_inner(&session_reject_reason())?;
        settle_promise(&promise, reason, true, env)?;
        return Ok(Value::Promise(promise));
    }

    let _ = request_session_promise(mode)?;
    // Resolve on a microtask boundary: settle after poll (stub/browser grant path).
    let resolved = poll_session_promise()?;
    settle_promise(&promise, resolved, false, env)?;
    Ok(Value::Promise(promise))
}

/// Explicitly reject the WebXR session Promise (browser NotSupportedError path).
pub fn reject_session_promise(reason: &str) -> Result<Value, String> {
    reject_session_promise_inner(reason)
}

fn reject_session_promise_inner(reason: &str) -> Result<Value, String> {
    let reason = if reason.is_empty() {
        "NotSupportedError"
    } else {
        reason
    };
    let mut promise_id = 1i64;
    let mut mode = String::new();
    if let Ok(mut p) = WEBXR_PROMISE.lock() {
        mode = p.mode.clone();
        promise_id = if p.promise_id == 0 {
            1
        } else if p.pending {
            p.promise_id
        } else {
            p.promise_id + 1
        };
        *p = WebXrSessionPromise {
            pending: false,
            resolved: false,
            rejected: true,
            mode: mode.clone(),
            promise_id,
            detail: format!("rejected:{reason}"),
            reason: reason.into(),
            raf_bound: false,
        };
    }
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(false));
    out.insert("pending".into(), Value::Bool(false));
    out.insert("resolved".into(), Value::Bool(false));
    out.insert("rejected".into(), Value::Bool(true));
    out.insert("reason".into(), Value::String(reason.into()));
    out.insert("promiseId".into(), Value::Number(promise_id));
    out.insert("mode".into(), Value::String(mode));
    out.insert("kind".into(), Value::String("xr_session_promise".into()));
    out.insert("error".into(), Value::String(reason.into()));
    Ok(Value::Object(out))
}

/// Start a already-rejected session Promise for `promise_then(..., onRejected)`.
pub fn start_rejected_session_promise_value(
    mode: &str,
    reason: &str,
    env: &mut crate::value::Environment,
) -> Result<Value, String> {
    use crate::runtime::stdlib::promise::settle_promise;
    use crate::value::PromiseValue;
    use std::cell::RefCell;
    use std::rc::Rc;

    let _ = mode;
    let rejected = reject_session_promise_inner(reason)?;
    let promise = Rc::new(RefCell::new(PromiseValue::Pending));
    settle_promise(&promise, rejected, true, env)?;
    Ok(Value::Promise(promise))
}

/// Mark WebXR session as granted (wasm path / tests) so rAF binds to XRSession.rAF.
pub fn grant_webxr_session() -> Result<Value, String> {
    let stub = stub_enabled();
    let st = status();
    if !st.bound && !stub {
        return Err("xr_grant_webxr_session: headset not bound".into());
    }
    {
        let active = LIVE_SESSION.lock().map(|l| l.active).unwrap_or(false);
        if !active {
            let _ = create_live_session("immersive-vr")?;
        }
    }
    let mut live = LIVE_SESSION
        .lock()
        .map_err(|_| "live session lock poisoned".to_string())?;
    live.webxr_granted = true;
    live.backend = "webxr-live".into();
    live.active = true;
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("webxrGranted".into(), Value::Bool(true));
    out.insert("backend".into(), Value::String(live.backend.clone()));
    out.insert("kind".into(), Value::String("xr_live_session".into()));
    Ok(Value::Object(out))
}

/// Start `navigator.xr.requestSession(mode)` as a Promise (pending until poll/resolve).
pub fn request_session_promise(mode: &str) -> Result<Value, String> {
    let mode = if mode.is_empty() { "immersive-vr" } else { mode };
    let stub = stub_enabled();
    let st = status();
    if !st.bound && !stub && !st.webxr {
        return Err("xr_request_session_promise: headset not bound".into());
    }

    let mut promise_id = 1i64;
    #[allow(unused_mut)]
    let mut detail = if stub {
        "stub-promise".to_string()
    } else if st.webxr {
        "webxr-requestSession-promise".to_string()
    } else {
        "descriptor-promise".to_string()
    };

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let navigator = window.navigator();
            if let Ok(xr) =
                js_sys::Reflect::get(&navigator, &wasm_bindgen::JsValue::from_str("xr"))
            {
                if !xr.is_undefined() && !xr.is_null() {
                    if let Ok(req) =
                        js_sys::Reflect::get(&xr, &wasm_bindgen::JsValue::from_str("requestSession"))
                    {
                        if req.is_function() {
                            let func = js_sys::Function::from(req);
                            let _ = func.call1(&xr, &wasm_bindgen::JsValue::from_str(mode));
                            detail = "webxr-browser-promise".to_string();
                        }
                    }
                }
            }
        }
    }

    if let Ok(mut p) = WEBXR_PROMISE.lock() {
        promise_id = p.promise_id + 1;
        *p = WebXrSessionPromise {
            pending: true,
            resolved: false,
            rejected: false,
            mode: mode.into(),
            promise_id,
            detail: detail.clone(),
            reason: String::new(),
            raf_bound: false,
        };
    }

    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("pending".into(), Value::Bool(true));
    out.insert("resolved".into(), Value::Bool(false));
    out.insert("promiseId".into(), Value::Number(promise_id));
    out.insert("mode".into(), Value::String(mode.into()));
    out.insert("detail".into(), Value::String(detail));
    out.insert("kind".into(), Value::String("xr_session_promise".into()));
    Ok(Value::Object(out))
}

/// Poll/resolve the WebXR session Promise; on resolve grants session + enables XRSession.rAF.
pub fn poll_session_promise() -> Result<Value, String> {
    let mut p = WEBXR_PROMISE
        .lock()
        .map_err(|_| "webxr promise lock poisoned".to_string())?;
    if p.rejected {
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(false));
        out.insert("pending".into(), Value::Bool(false));
        out.insert("resolved".into(), Value::Bool(false));
        out.insert("rejected".into(), Value::Bool(true));
        out.insert("reason".into(), Value::String(p.reason.clone()));
        out.insert("error".into(), Value::String(p.reason.clone()));
        out.insert("promiseId".into(), Value::Number(p.promise_id));
        out.insert("mode".into(), Value::String(p.mode.clone()));
        out.insert("kind".into(), Value::String("xr_session_promise".into()));
        return Ok(Value::Object(out));
    }
    if !p.pending && !p.resolved {
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(false));
        out.insert("pending".into(), Value::Bool(false));
        out.insert("resolved".into(), Value::Bool(false));
        out.insert("error".into(), Value::String("no promise".into()));
        return Ok(Value::Object(out));
    }
    if p.resolved {
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(true));
        out.insert("pending".into(), Value::Bool(false));
        out.insert("resolved".into(), Value::Bool(true));
        out.insert("rejected".into(), Value::Bool(false));
        out.insert("promiseId".into(), Value::Number(p.promise_id));
        out.insert("mode".into(), Value::String(p.mode.clone()));
        out.insert("rafBound".into(), Value::Bool(p.raf_bound));
        out.insert("kind".into(), Value::String("xr_session_promise".into()));
        return Ok(Value::Object(out));
    }

    // Resolve: grant live WebXR session and bind XRSession.requestAnimationFrame.
    let mode = p.mode.clone();
    let promise_id = p.promise_id;
    let detail = p.detail.clone();
    p.pending = false;
    p.resolved = true;
    p.rejected = false;
    p.reason.clear();
    drop(p);

    let _ = create_live_session(&mode)?;
    let _ = grant_webxr_session()?;
    let raf = raf_bind()?;
    // Prefer promise-specific backend label.
    XR_RAF.with(|cell| {
        let mut raf_st = cell.borrow_mut();
        raf_st.backend = "webxr-session-raf-promise".into();
    });
    if let Ok(mut live) = LIVE_SESSION.lock() {
        live.webxr_raf_bound = true;
    }
    if let Ok(mut p) = WEBXR_PROMISE.lock() {
        p.raf_bound = true;
        p.detail = format!("{detail}+resolved");
    }

    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("pending".into(), Value::Bool(false));
    out.insert("resolved".into(), Value::Bool(true));
    out.insert("promiseId".into(), Value::Number(promise_id));
    out.insert("mode".into(), Value::String(mode));
    out.insert("raf".into(), raf);
    out.insert(
        "rafBackend".into(),
        Value::String("webxr-session-raf-promise".into()),
    );
    out.insert("live".into(), live_session_status());
    out.insert("kind".into(), Value::String("xr_session_promise".into()));
    Ok(Value::Object(out))
}

pub fn session_promise_status() -> Value {
    let mut out = HashMap::new();
    match WEBXR_PROMISE.lock() {
        Ok(p) => {
            out.insert("pending".into(), Value::Bool(p.pending));
            out.insert("resolved".into(), Value::Bool(p.resolved));
            out.insert("rejected".into(), Value::Bool(p.rejected));
            out.insert("reason".into(), Value::String(p.reason.clone()));
            out.insert("promiseId".into(), Value::Number(p.promise_id));
            out.insert("mode".into(), Value::String(p.mode.clone()));
            out.insert("detail".into(), Value::String(p.detail.clone()));
            out.insert("rafBound".into(), Value::Bool(p.raf_bound));
        }
        Err(_) => {
            out.insert("pending".into(), Value::Bool(false));
            out.insert("resolved".into(), Value::Bool(false));
        }
    }
    out.insert("kind".into(), Value::String("xr_session_promise".into()));
    Value::Object(out)
}

/// Invoke `xrEndFrame` via live session (stub trampoline or resolved loader proc).
fn invoke_end_frame_ffi(
    frame_index: i64,
    layer_count: i64,
) -> (bool, i32, String) {
    let live = match LIVE_SESSION.lock() {
        Ok(l) => l.clone(),
        Err(_) => return (false, -1, "lock-failed".into()),
    };
    if !live.active {
        return (false, 0, "no-live-session".into());
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Wasm uses WebXR session rAF / frame loop — record a granted-session end.
        if let Ok(mut g) = LIVE_SESSION.lock() {
            g.end_frame_ffi_calls += 1;
            g.last_ffi_result = 0;
            g.last_ffi_mode = "webxr-session-end".into();
        }
        let _ = frame_index;
        let _ = layer_count;
        return (true, 0, "webxr-session-end".into());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let (fn_addr, use_stub) = {
            let fns = match OPENXR_FNS.lock() {
                Ok(f) => f,
                Err(_) => return (false, -1, "fns-lock-failed".into()),
            };
            let stub = stub_enabled() || live.backend == "stub-live";
            if stub {
                (stub_xr_end_frame_addr(), true)
            } else if fns.end_frame_resolved && fns.end_frame != 0 {
                (fns.end_frame, false)
            } else {
                (stub_xr_end_frame_addr(), true)
            }
        };
        if fn_addr == 0 {
            return (false, -1, "no-end-frame-proc".into());
        }

        let info = XrFrameEndInfo {
            ty: XR_TYPE_FRAME_END_INFO,
            next: std::ptr::null(),
            display_time: frame_index.saturating_mul(11_111_111),
            environment_blend_mode: XR_ENVIRONMENT_BLEND_MODE_OPAQUE,
            layer_count: layer_count.max(0) as u32,
            layers: std::ptr::null(),
        };
        // Direct loader calls use NULL session when we lack a real XrSession handle.
        let session_handle = if use_stub {
            if live.session != 0 {
                live.session
            } else {
                0x5E_0000_0001u64
            }
        } else if live.session_created && live.session != 0 {
            live.session
        } else {
            0u64
        };
        let end_frame: XrEndFrameFn = unsafe { std::mem::transmute(fn_addr) };
        let rc = unsafe { end_frame(session_handle, &info) };
        let mode = if use_stub {
            "stub-trampoline".to_string()
        } else {
            "direct".to_string()
        };
        if let Ok(mut g) = LIVE_SESSION.lock() {
            g.end_frame_ffi_calls += 1;
            g.last_ffi_result = rc;
            g.last_ffi_mode = mode.clone();
        }
        (true, rc, mode)
    }
}

/// Open compositor IPC (OpenXR end-frame → compositor handoff channel).
pub fn compositor_open() -> Result<Value, String> {
    let st = status();
    let stub = stub_enabled();
    if !st.bound && !stub {
        return Err("xr_compositor_open: headset not bound".into());
    }
    let channel = if stub || st.backend == "xr-stub" {
        "stub://compositor".to_string()
    } else if st.openxr_runtime || st.openxr_loader {
        format!("openxr-ipc://{}", st.loader_path)
    } else if st.webxr {
        "webxr://compositor".to_string()
    } else {
        "descriptor://compositor".to_string()
    };
    let mut ipc = COMPOSITOR_IPC
        .lock()
        .map_err(|_| "compositor ipc lock poisoned".to_string())?;
    ipc.open = true;
    ipc.channel = channel.clone();
    ipc.pending.clear();
    ipc.acks.clear();
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("open".into(), Value::Bool(true));
    out.insert("channel".into(), Value::String(channel));
    out.insert("kind".into(), Value::String("xr_compositor_ipc".into()));
    Ok(Value::Object(out))
}

/// Collect wgpu/Vulkan image handles from composition views' subImage bags.
fn composition_image_handles(composition: &Value) -> (Vec<Value>, Vec<Value>) {
    let mut native = Vec::new();
    let mut wgpu = Vec::new();
    let Value::Object(m) = composition else {
        return (native, wgpu);
    };
    let views = match m.get("views") {
        Some(Value::Array(a)) => a.as_slice(),
        _ => &[],
    };
    for view in views {
        let Value::Object(vm) = view else {
            continue;
        };
        let Some(Value::Object(sub)) = vm.get("subImage") else {
            continue;
        };
        if let Some(v) = sub.get("nativeImage") {
            native.push(v.clone());
        }
        if let Some(v) = sub.get("wgpuTextureId") {
            wgpu.push(v.clone());
        }
    }
    (native, wgpu)
}

/// Submit composed frame into compositor IPC queue (OpenXR xrEndFrame analogue).
pub fn compositor_submit(composition: &Value) -> Result<Value, String> {
    let mut ipc = COMPOSITOR_IPC
        .lock()
        .map_err(|_| "compositor ipc lock poisoned".to_string())?;
    if !ipc.open {
        drop(ipc);
        compositor_open()?;
        ipc = COMPOSITOR_IPC
            .lock()
            .map_err(|_| "compositor ipc lock poisoned".to_string())?;
    }
    let (view_count, width, height, frame_index) = match composition {
        Value::Object(m) => {
            let vc = match m.get("viewCount") {
                Some(Value::Number(n)) => *n,
                _ => 0,
            };
            let w = match m.get("sideBySideWidth") {
                Some(Value::Number(n)) => *n,
                _ => 0,
            };
            let h = match m.get("sideBySideHeight") {
                Some(Value::Number(n)) => *n,
                _ => 0,
            };
            let fi = match m.get("frameIndex") {
                Some(Value::Number(n)) => *n,
                _ => ipc.submits + 1,
            };
            (vc, w, h, fi)
        }
        _ => (0, 0, 0, ipc.submits + 1),
    };
    let (native_images, wgpu_texture_ids) = composition_image_handles(composition);
    ipc.submits += 1;
    ipc.pending.push(CompositorMsg {
        frame_index,
        view_count,
        width,
        height,
        layer_type: "COMPOSITION_LAYER_PROJECTION".into(),
    });
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("queued".into(), Value::Bool(true));
    out.insert("frameIndex".into(), Value::Number(frame_index));
    out.insert("pending".into(), Value::Number(ipc.pending.len() as i64));
    out.insert("channel".into(), Value::String(ipc.channel.clone()));
    out.insert("kind".into(), Value::String("xr_compositor_submit".into()));
    // GP6n: echo real swapchain image handles into submit ack (layer submit path).
    out.insert("submittedImages".into(), Value::Array(native_images));
    out.insert("wgpuTextureIds".into(), Value::Array(wgpu_texture_ids));
    Ok(Value::Object(out))
}

/// Poll compositor IPC — ack one pending frame (compositor → app).
pub fn compositor_poll() -> Result<Value, String> {
    let mut ipc = COMPOSITOR_IPC
        .lock()
        .map_err(|_| "compositor ipc lock poisoned".to_string())?;
    if !ipc.open {
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(false));
        out.insert("open".into(), Value::Bool(false));
        out.insert("acked".into(), Value::Bool(false));
        return Ok(Value::Object(out));
    }
    if let Some(msg) = ipc.pending.first().cloned() {
        ipc.pending.remove(0);
        ipc.acks.push(msg.frame_index);
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(true));
        out.insert("acked".into(), Value::Bool(true));
        out.insert("frameIndex".into(), Value::Number(msg.frame_index));
        out.insert("viewCount".into(), Value::Number(msg.view_count));
        out.insert("width".into(), Value::Number(msg.width));
        out.insert("height".into(), Value::Number(msg.height));
        out.insert("layerType".into(), Value::String(msg.layer_type));
        out.insert("pending".into(), Value::Number(ipc.pending.len() as i64));
        out.insert("kind".into(), Value::String("xr_compositor_ack".into()));
        return Ok(Value::Object(out));
    }
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("acked".into(), Value::Bool(false));
    out.insert("pending".into(), Value::Number(0));
    out.insert("kind".into(), Value::String("xr_compositor_ack".into()));
    Ok(Value::Object(out))
}

/// WebXR `navigator.xr.requestSession(mode)` — real on wasm when available; stub otherwise.
pub fn request_session(mode: &str) -> Result<Value, String> {
    let mode = if mode.is_empty() { "immersive-vr" } else { mode };
    let stub = stub_enabled();
    let st = status();

    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let navigator = window.navigator();
            if let Ok(xr) =
                js_sys::Reflect::get(&navigator, &wasm_bindgen::JsValue::from_str("xr"))
            {
                if !xr.is_undefined() && !xr.is_null() {
                    // Best-effort: call requestSession if present (may return a Promise).
                    if let Ok(req) =
                        js_sys::Reflect::get(&xr, &wasm_bindgen::JsValue::from_str("requestSession"))
                    {
                        if req.is_function() {
                            let func = js_sys::Function::from(req);
                            let _ = func.call1(&xr, &wasm_bindgen::JsValue::from_str(mode));
                            // Mark session as granted/pending for XRSession.rAF binding.
                            let live = create_live_session(mode)?;
                            if let Ok(mut g) = LIVE_SESSION.lock() {
                                g.webxr_granted = true;
                                g.backend = "webxr-live".into();
                            }
                            let mut out = HashMap::new();
                            out.insert("ok".into(), Value::Bool(true));
                            out.insert("mode".into(), Value::String(mode.into()));
                            out.insert("backend".into(), Value::String("webxr-requestSession".into()));
                            out.insert("kind".into(), Value::String("xr_session".into()));
                            out.insert("active".into(), Value::Bool(true));
                            out.insert("pending".into(), Value::Bool(true));
                            out.insert("live".into(), live);
                            out.insert("webxrGranted".into(), Value::Bool(true));
                            return Ok(Value::Object(out));
                        }
                    }
                }
            }
        }
    }

    if stub || st.bound || st.webxr || st.openxr_runtime || live_env_forced() {
        let backend = if stub {
            "stub-requestSession"
        } else if st.webxr {
            "webxr-descriptor"
        } else if st.openxr_runtime || st.openxr_loader {
            "openxr-session"
        } else {
            "descriptor-session"
        };
        let live = create_live_session(mode)?;
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(true));
        out.insert("mode".into(), Value::String(mode.into()));
        out.insert("backend".into(), Value::String(backend.into()));
        out.insert("kind".into(), Value::String("xr_session".into()));
        out.insert("active".into(), Value::Bool(true));
        out.insert("pending".into(), Value::Bool(false));
        out.insert("live".into(), live);
        out.insert(
            "features".into(),
            Value::Array(vec![
                Value::String("local".into()),
                Value::String("viewer".into()),
            ]),
        );
        return Ok(Value::Object(out));
    }
    Err(format!(
        "xr_request_session({mode}): no WebXR/OpenXR runtime (set KABOOTAR_XR_STUB=1)"
    ))
}

/// Vendor OpenXR compositor process (in-process worker that drains IPC).
#[derive(Debug, Default)]
struct CompositorProcess {
    running: bool,
    pid: i64,
    ticks: i64,
    frames_composed: i64,
    name: String,
}

static COMPOSITOR_PROC: Mutex<CompositorProcess> = Mutex::new(CompositorProcess {
    running: false,
    pid: 0,
    ticks: 0,
    frames_composed: 0,
    name: String::new(),
});

/// Spawn vendor compositor process (stub or openxr-named worker).
pub fn compositor_process_spawn() -> Result<Value, String> {
    let st = status();
    let stub = stub_enabled();
    if !st.bound && !stub {
        return Err("xr_compositor_process_spawn: headset not bound".into());
    }
    // Ensure IPC channel exists.
    let _ = compositor_open()?;
    let name = if stub || st.backend == "xr-stub" {
        "kab-compositor-stub".to_string()
    } else if st.openxr_runtime || st.openxr_loader {
        "openxr-compositor".to_string()
    } else if st.webxr {
        "webxr-compositor".to_string()
    } else {
        "descriptor-compositor".to_string()
    };
    let mut proc = COMPOSITOR_PROC
        .lock()
        .map_err(|_| "compositor process lock poisoned".to_string())?;
    proc.running = true;
    proc.pid = 9000 + (std::process::id() as i64 % 1000);
    proc.ticks = 0;
    proc.frames_composed = 0;
    proc.name = name.clone();
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("running".into(), Value::Bool(true));
    out.insert("pid".into(), Value::Number(proc.pid));
    out.insert("name".into(), Value::String(name));
    out.insert("kind".into(), Value::String("xr_compositor_process".into()));
    Ok(Value::Object(out))
}

/// Tick compositor process — drain one IPC frame (vendor worker).
pub fn compositor_process_tick() -> Result<Value, String> {
    let mut proc = COMPOSITOR_PROC
        .lock()
        .map_err(|_| "compositor process lock poisoned".to_string())?;
    if !proc.running {
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(false));
        out.insert("running".into(), Value::Bool(false));
        out.insert("drained".into(), Value::Bool(false));
        return Ok(Value::Object(out));
    }
    proc.ticks += 1;
    drop(proc);
    let ack = compositor_poll()?;
    let drained = matches!(
        &ack,
        Value::Object(m) if matches!(m.get("acked"), Some(Value::Bool(true)))
    );
    if drained {
        if let Ok(mut proc) = COMPOSITOR_PROC.lock() {
            proc.frames_composed += 1;
        }
    }
    let proc = COMPOSITOR_PROC
        .lock()
        .map_err(|_| "compositor process lock poisoned".to_string())?;
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("running".into(), Value::Bool(true));
    out.insert("drained".into(), Value::Bool(drained));
    out.insert("ticks".into(), Value::Number(proc.ticks));
    out.insert("framesComposed".into(), Value::Number(proc.frames_composed));
    out.insert("pid".into(), Value::Number(proc.pid));
    out.insert("name".into(), Value::String(proc.name.clone()));
    out.insert("ack".into(), ack);
    out.insert("kind".into(), Value::String("xr_compositor_tick".into()));
    Ok(Value::Object(out))
}

pub fn compositor_process_stop() -> Result<Value, String> {
    let mut proc = COMPOSITOR_PROC
        .lock()
        .map_err(|_| "compositor process lock poisoned".to_string())?;
    let frames = proc.frames_composed;
    let ticks = proc.ticks;
    let pid = proc.pid;
    proc.running = false;
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("running".into(), Value::Bool(false));
    out.insert("pid".into(), Value::Number(pid));
    out.insert("framesComposed".into(), Value::Number(frames));
    out.insert("ticks".into(), Value::Number(ticks));
    out.insert("kind".into(), Value::String("xr_compositor_process".into()));
    Ok(Value::Object(out))
}

pub fn compositor_process_status() -> Value {
    let proc = COMPOSITOR_PROC.lock().ok();
    let mut out = HashMap::new();
    match proc {
        Some(p) => {
            out.insert("running".into(), Value::Bool(p.running));
            out.insert("pid".into(), Value::Number(p.pid));
            out.insert("ticks".into(), Value::Number(p.ticks));
            out.insert("framesComposed".into(), Value::Number(p.frames_composed));
            out.insert("name".into(), Value::String(p.name.clone()));
        }
        None => {
            out.insert("running".into(), Value::Bool(false));
        }
    }
    out.insert("kind".into(), Value::String("xr_compositor_process".into()));
    Value::Object(out)
}

/// OpenXR loader `xrEndFrame` path — resolve proc when possible; always record submit.
pub fn loader_end_frame(
    frame_index: i64,
    layer_count: i64,
    _composition: &Value,
) -> Result<Value, String> {
    let st = status();
    let stub = stub_enabled();
    if !st.bound && !stub {
        return Err("xr_loader_end_frame: headset not bound".into());
    }

    // Re-resolve if we have get_proc but not yet end_frame.
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Ok(fns) = OPENXR_FNS.lock() {
            let gp = fns.get_proc;
            let resolved = fns.end_frame_resolved;
            drop(fns);
            if gp != 0 && !resolved {
                let _ = resolve_xr_end_frame_proc(gp);
            }
        }
    }

    let (proc_resolved, mut path) = {
        let fns = OPENXR_FNS
            .lock()
            .map_err(|_| "openxr fns lock poisoned".to_string())?;
        let resolved = fns.end_frame_resolved && fns.end_frame != 0;
        let path = if stub || st.backend == "xr-stub" {
            "stub-xrEndFrame".to_string()
        } else if resolved {
            "openxr-loader-xrEndFrame".to_string()
        } else if st.openxr_loader || st.openxr_runtime {
            "openxr-loader-endFrame-descriptor".to_string()
        } else if st.webxr {
            "webxr-frame-end".to_string()
        } else {
            "descriptor-xrEndFrame".to_string()
        };
        (resolved, path)
    };

    // Ensure a live session exists so we can invoke xrEndFrame FFI.
    let live_active = LIVE_SESSION
        .lock()
        .map(|l| l.active)
        .unwrap_or(false);
    if !live_active {
        let _ = create_live_session("immersive-vr");
    }

    let (ffi_invoked, ffi_result, ffi_mode) = invoke_end_frame_ffi(frame_index, layer_count);
    if ffi_invoked {
        path = match ffi_mode.as_str() {
            "stub-trampoline" => "stub-xrEndFrame-ffi".into(),
            "direct" => "openxr-loader-xrEndFrame-ffi".into(),
            "webxr-session-end" => "webxr-session-end-ffi".into(),
            other => format!("{path}+{other}"),
        };
    }

    if let Ok(mut fns) = OPENXR_FNS.lock() {
        fns.end_frame_calls += 1;
        fns.last_frame_index = frame_index;
        fns.last_path = path.clone();
    }

    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("submitted".into(), Value::Bool(true));
    out.insert("frameIndex".into(), Value::Number(frame_index));
    out.insert("layerCount".into(), Value::Number(layer_count));
    out.insert("procResolved".into(), Value::Bool(proc_resolved));
    out.insert("path".into(), Value::String(path));
    out.insert("ffiInvoked".into(), Value::Bool(ffi_invoked));
    out.insert("ffiResult".into(), Value::Number(ffi_result as i64));
    out.insert("ffiMode".into(), Value::String(ffi_mode));
    out.insert("kind".into(), Value::String("xr_loader_end_frame".into()));
    out.insert(
        "loaderPath".into(),
        Value::String(st.loader_path.clone()),
    );
    Ok(Value::Object(out))
}

pub fn loader_end_frame_status() -> Value {
    let mut out = HashMap::new();
    match OPENXR_FNS.lock() {
        Ok(fns) => {
            out.insert("procResolved".into(), Value::Bool(fns.end_frame_resolved));
            out.insert("calls".into(), Value::Number(fns.end_frame_calls));
            out.insert("lastFrameIndex".into(), Value::Number(fns.last_frame_index));
            out.insert("lastPath".into(), Value::String(fns.last_path.clone()));
            out.insert("hasGetProc".into(), Value::Bool(fns.get_proc != 0));
        }
        Err(_) => {
            out.insert("procResolved".into(), Value::Bool(false));
        }
    }
    if let Ok(live) = LIVE_SESSION.lock() {
        out.insert("liveActive".into(), Value::Bool(live.active));
        out.insert(
            "endFrameFfiCalls".into(),
            Value::Number(live.end_frame_ffi_calls),
        );
        out.insert(
            "lastFfiMode".into(),
            Value::String(live.last_ffi_mode.clone()),
        );
        out.insert(
            "lastFfiResult".into(),
            Value::Number(live.last_ffi_result as i64),
        );
    }
    out.insert("kind".into(), Value::String("xr_loader_end_frame".into()));
    Value::Object(out)
}

/// Bind WebXR `XRSession.requestAnimationFrame` (wasm) or stub XR rAF queue.
pub fn raf_bind() -> Result<Value, String> {
    let st = status();
    let stub = stub_enabled();
    if !st.bound && !stub {
        return Err("xr_raf_bind: headset not bound".into());
    }

    let live = LIVE_SESSION.lock().ok().map(|l| l.clone());
    let webxr_granted = live.as_ref().map(|l| l.webxr_granted).unwrap_or(false);

    let backend = if webxr_granted {
        "webxr-session-raf".to_string()
    } else if stub || st.backend == "xr-stub" {
        "stub-xr-raf".to_string()
    } else if st.webxr {
        "webxr-requestAnimationFrame".to_string()
    } else if st.openxr_runtime || st.openxr_loader {
        "openxr-frame-loop".to_string()
    } else {
        "descriptor-xr-raf".to_string()
    };

    #[cfg(target_arch = "wasm32")]
    let backend = {
        let mut backend = backend;
        if webxr_granted {
            // Prefer XRSession.requestAnimationFrame once the browser grants a session.
            backend = "webxr-session-raf".to_string();
            if let Ok(mut g) = LIVE_SESSION.lock() {
                g.webxr_raf_bound = true;
            }
        } else if st.webxr {
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                if let Ok(xr) =
                    js_sys::Reflect::get(&navigator, &wasm_bindgen::JsValue::from_str("xr"))
                {
                    if !xr.is_undefined() && !xr.is_null() {
                        backend = "webxr-navigator-raf".to_string();
                    }
                }
            }
        }
        backend
    };

    #[cfg(not(target_arch = "wasm32"))]
    if webxr_granted {
        if let Ok(mut g) = LIVE_SESSION.lock() {
            g.webxr_raf_bound = true;
        }
    }

    XR_RAF.with(|cell| {
        let mut raf = cell.borrow_mut();
        raf.bound = true;
        raf.backend = backend.clone();
    });
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("bound".into(), Value::Bool(true));
    out.insert("backend".into(), Value::String(backend));
    out.insert("webxrGranted".into(), Value::Bool(webxr_granted));
    out.insert("kind".into(), Value::String("xr_raf_bind".into()));
    Ok(Value::Object(out))
}

pub fn request_animation_frame(callback: Value) -> Result<Value, String> {
    let needs_bind = XR_RAF.with(|cell| !cell.borrow().bound);
    if needs_bind {
        raf_bind()?;
    }
    let id = XR_RAF.with(|cell| {
        let mut raf = cell.borrow_mut();
        let id = raf.next_id;
        raf.next_id += 1;
        raf.cancelled.retain(|c| *c != id);
        raf.pending.push((id, callback));
        id
    });
    Ok(Value::Number(id))
}

pub fn cancel_animation_frame(id: i64) -> Result<Value, String> {
    XR_RAF.with(|cell| {
        let mut raf = cell.borrow_mut();
        raf.cancelled.push(id);
        raf.pending.retain(|(fid, _)| *fid != id);
    });
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("id".into(), Value::Number(id));
    Ok(Value::Object(out))
}

/// Pump XR rAF callbacks (WebXR immersive frame callbacks).
pub fn raf_tick(env: &mut crate::value::Environment) -> Result<Value, String> {
    let (callbacks, backend, tick_no, bound) = XR_RAF.with(|cell| {
        let mut raf = cell.borrow_mut();
        if !raf.bound {
            return (Vec::new(), String::new(), 0i64, false);
        }
        raf.ticks += 1;
        let tick_no = raf.ticks;
        let backend = raf.backend.clone();
        let mut callbacks = Vec::new();
        let pending = std::mem::take(&mut raf.pending);
        for (id, cb) in pending {
            if raf.cancelled.iter().any(|c| *c == id) {
                raf.cancelled.retain(|c| *c != id);
                continue;
            }
            callbacks.push((id, cb));
        }
        (callbacks, backend, tick_no, true)
    });

    if !bound {
        let mut out = HashMap::new();
        out.insert("ok".into(), Value::Bool(false));
        out.insert("bound".into(), Value::Bool(false));
        out.insert("ran".into(), Value::Number(0));
        return Ok(Value::Object(out));
    }

    let time_ms = crate::value::unix_ms_now() as i64;
    let mut ran = 0i64;
    for (_id, cb) in callbacks {
        let mut frame = HashMap::new();
        frame.insert("kind".into(), Value::String("xr_frame".into()));
        frame.insert("time".into(), Value::Number(time_ms));
        frame.insert("tick".into(), Value::Number(tick_no));
        frame.insert("backend".into(), Value::String(backend.clone()));
        crate::bytecode::call_value(
            cb,
            vec![Value::Float(time_ms as f64), Value::Object(frame)],
            &[],
            &[],
            &[],
            &[],
            env,
        )?;
        ran += 1;
    }

    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("ran".into(), Value::Number(ran));
    out.insert("tick".into(), Value::Number(tick_no));
    out.insert("backend".into(), Value::String(backend));
    out.insert("kind".into(), Value::String("xr_raf_tick".into()));
    Ok(Value::Object(out))
}

pub fn raf_status() -> Value {
    let mut out = HashMap::new();
    XR_RAF.with(|cell| {
        let raf = cell.borrow();
        out.insert("bound".into(), Value::Bool(raf.bound));
        out.insert("pending".into(), Value::Number(raf.pending.len() as i64));
        out.insert("ticks".into(), Value::Number(raf.ticks));
        out.insert("backend".into(), Value::String(raf.backend.clone()));
    });
    out.insert("kind".into(), Value::String("xr_raf".into()));
    Value::Object(out)
}

// ----- GP6n WebXR inputSources stub (select/squeeze + grip/targetRay) -----

#[derive(Debug, Clone)]
struct XrInputEvent {
    ty: String,
    handedness: String,
    frame_index: i64,
}

#[derive(Debug, Default)]
struct XrInputState {
    ready: bool,
    events: Vec<XrInputEvent>,
    frame_index: i64,
}

static XR_INPUT: Mutex<XrInputState> = Mutex::new(XrInputState {
    ready: false,
    events: Vec::new(),
    frame_index: 0,
});

fn pose_map(x: f64, y: f64, z: f64) -> Value {
    let mut m = HashMap::new();
    m.insert("x".into(), Value::Float(x));
    m.insert("y".into(), Value::Float(y));
    m.insert("z".into(), Value::Float(z));
    m.insert("qx".into(), Value::Float(0.0));
    m.insert("qy".into(), Value::Float(0.0));
    m.insert("qz".into(), Value::Float(0.0));
    m.insert("qw".into(), Value::Float(1.0));
    m.insert("emulated".into(), Value::Bool(true));
    Value::Object(m)
}

fn stub_source(handedness: &str) -> Value {
    let (gx, gz) = if handedness == "left" {
        (-0.25, -0.4)
    } else {
        (0.25, -0.4)
    };
    let mut gamepad = HashMap::new();
    gamepad.insert("buttons".into(), Value::Array(vec![
        Value::Object({
            let mut b = HashMap::new();
            b.insert("pressed".into(), Value::Bool(false));
            b.insert("value".into(), Value::Float(0.0));
            b
        }),
        Value::Object({
            let mut b = HashMap::new();
            b.insert("pressed".into(), Value::Bool(false));
            b.insert("value".into(), Value::Float(0.0));
            b
        }),
    ]));
    gamepad.insert("axes".into(), Value::Array(vec![
        Value::Float(0.0),
        Value::Float(0.0),
    ]));

    let mut m = HashMap::new();
    m.insert("id".into(), Value::String(handedness.into()));
    m.insert("handedness".into(), Value::String(handedness.into()));
    m.insert("targetRayMode".into(), Value::String("tracked-pointer".into()));
    m.insert(
        "profiles".into(),
        Value::Array(vec![
            Value::String("oculus-touch".into()),
            Value::String("generic-trigger-squeeze-thumbstick".into()),
            Value::String("generic-trigger".into()),
        ]),
    );
    m.insert("gamepad".into(), Value::Object(gamepad));
    m.insert("gripPose".into(), pose_map(gx, 1.2, gz));
    m.insert("targetRayPose".into(), pose_map(gx, 1.25, gz - 0.05));
    m.insert("kind".into(), Value::String("xr_input_source".into()));
    Value::Object(m)
}

fn stub_hand_joints(handedness: &str) -> Value {
    let (base_x, base_z) = if handedness == "left" {
        (-0.2, -0.35)
    } else {
        (0.2, -0.35)
    };
    let names = [
        "wrist",
        "thumb-metacarpal",
        "thumb-phalanx-proximal",
        "thumb-tip",
        "index-finger-metacarpal",
        "index-finger-phalanx-proximal",
        "index-finger-tip",
        "middle-finger-tip",
        "ring-finger-tip",
        "pinky-finger-tip",
    ];
    let mut joints = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let mut j = HashMap::new();
        j.insert("joint".into(), Value::String((*name).into()));
        j.insert(
            "pose".into(),
            pose_map(base_x + (i as f64) * 0.01, 1.15, base_z - (i as f64) * 0.008),
        );
        j.insert("radius".into(), Value::Float(0.008));
        joints.push(Value::Object(j));
    }
    let mut out = HashMap::new();
    out.insert("handedness".into(), Value::String(handedness.into()));
    out.insert("joints".into(), Value::Array(joints));
    out.insert("tracking".into(), Value::String("emulated".into()));
    out.insert("ok".into(), Value::Bool(true));
    out.insert("kind".into(), Value::String("xr_hand".into()));
    Value::Object(out)
}

/// XRHand-style joint poses (stub / emulated tracking).
pub fn hand_joints(handedness: &str) -> Result<Value, String> {
    ensure_input_sources();
    if handedness != "left" && handedness != "right" {
        return Err(format!("xr_hand_joints: handedness left|right, got {handedness}"));
    }
    Ok(stub_hand_joints(handedness))
}

/// Preferred input profile list for a handedness (WebXR `profiles` order).
pub fn input_profiles(handedness: &str) -> Result<Value, String> {
    ensure_input_sources();
    if handedness != "left" && handedness != "right" {
        return Err(format!("xr_input_profiles: handedness left|right, got {handedness}"));
    }
    let Value::Object(m) = stub_source(handedness) else {
        return Err("xr_input_profiles: internal".into());
    };
    m.get("profiles")
        .cloned()
        .ok_or_else(|| "xr_input_profiles: missing".into())
}

/// Ensure stub left/right input sources exist (idempotent).
pub fn ensure_input_sources() {
    if let Ok(mut st) = XR_INPUT.lock() {
        st.ready = true;
    }
}

pub fn enumerate_input_sources() -> Value {
    ensure_input_sources();
    let mut out = HashMap::new();
    out.insert(
        "sources".into(),
        Value::Array(vec![stub_source("left"), stub_source("right")]),
    );
    out.insert("count".into(), Value::Number(2));
    out.insert("kind".into(), Value::String("xr_input_sources".into()));
    out.insert("ok".into(), Value::Bool(true));
    Value::Object(out)
}

pub fn input_source_pose(handedness: &str, kind: &str) -> Result<Value, String> {
    ensure_input_sources();
    let hand = if handedness == "left" || handedness == "right" {
        handedness
    } else {
        return Err(format!("xr_input_source_pose: unknown handedness {handedness}"));
    };
    let src = stub_source(hand);
    let Value::Object(m) = src else {
        return Err("xr_input_source_pose: internal".into());
    };
    let key = match kind {
        "grip" => "gripPose",
        "targetRay" | "target_ray" => "targetRayPose",
        _ => return Err(format!("xr_input_source_pose: kind must be grip|targetRay, got {kind}")),
    };
    m.get(key)
        .cloned()
        .ok_or_else(|| "xr_input_source_pose: missing pose".into())
}

pub fn inject_input_event(ty: &str, handedness: &str) -> Result<Value, String> {
    ensure_input_sources();
    let valid = matches!(
        ty,
        "selectstart"
            | "select"
            | "selectend"
            | "squeezestart"
            | "squeeze"
            | "squeezeend"
    );
    if !valid {
        return Err(format!("xr_inject_input_event: unknown type {ty}"));
    }
    if handedness != "left" && handedness != "right" {
        return Err(format!("xr_inject_input_event: handedness left|right, got {handedness}"));
    }
    let mut st = XR_INPUT
        .lock()
        .map_err(|_| "xr_inject_input_event: lock".to_string())?;
    st.frame_index += 1;
    let fi = st.frame_index;
    st.events.push(XrInputEvent {
        ty: ty.into(),
        handedness: handedness.into(),
        frame_index: fi,
    });
    let mut out = HashMap::new();
    out.insert("ok".into(), Value::Bool(true));
    out.insert("type".into(), Value::String(ty.into()));
    out.insert("handedness".into(), Value::String(handedness.into()));
    out.insert("frameIndex".into(), Value::Number(fi));
    out.insert("queued".into(), Value::Number(st.events.len() as i64));
    Ok(Value::Object(out))
}

pub fn poll_input_events() -> Value {
    ensure_input_sources();
    let mut st = XR_INPUT.lock().unwrap_or_else(|e| e.into_inner());
    let drained: Vec<XrInputEvent> = st.events.drain(..).collect();
    let events: Vec<Value> = drained
        .into_iter()
        .map(|ev| {
            let mut m = HashMap::new();
            m.insert("type".into(), Value::String(ev.ty));
            m.insert("handedness".into(), Value::String(ev.handedness.clone()));
            m.insert("inputSource".into(), stub_source(&ev.handedness));
            m.insert("frameIndex".into(), Value::Number(ev.frame_index));
            Value::Object(m)
        })
        .collect();
    let mut out = HashMap::new();
    out.insert("events".into(), Value::Array(events.clone()));
    out.insert("count".into(), Value::Number(events.len() as i64));
    out.insert("ok".into(), Value::Bool(true));
    out.insert("kind".into(), Value::String("xr_input_events".into()));
    Value::Object(out)
}

fn reset_input_for_tests() {
    if let Ok(mut st) = XR_INPUT.lock() {
        *st = XrInputState::default();
    }
}

