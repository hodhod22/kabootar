//! Browser Platform v2 tests (post-Kv8)

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::runtime::browser_platform::{load_wasm, minimal_add_wasm};
use kabootar_lib::value::Value;

fn eval(code: &str) -> Value {
    let mut env = create_global_env();
    eval_source(code, &mut env).unwrap()
}

#[test]
fn bp_info_lists_all_modules() {
    let info = eval("bp_info()");
    let Value::Object(o) = info else {
        panic!("expected object");
    };
    assert!(matches!(o.get("platform"), Some(Value::String(s)) if s.contains("browser")));
    assert!(matches!(o.get("version"), Some(Value::String(s)) if s == "2.58.0"));
    assert!(o.contains_key("wasm"));
    assert!(o.contains_key("webgl"));
    assert!(o.contains_key("pwa"));
    let Value::Object(wasm) = o.get("wasm").unwrap() else {
        panic!("expected wasm object");
    };
    assert!(matches!(wasm.get("phase"), Some(Value::String(s)) if s == "v2.51"));
}

#[test]
fn devtools_console_and_inspect() {
    let logs = eval(
        r#"
        devtools_log("info", "hello");
        devtools_dump();
        "#,
    );
    let Value::Array(entries) = logs else {
        panic!("expected array");
    };
    assert!(!entries.is_empty());
}

#[test]
fn kv8_console_log_hooks_devtools() {
    let logs = eval(
        r#"
        let ctx = kv8_create();
        kv8_eval(ctx, "console.log('from kv8');");
        devtools_dump();
        "#,
    );
    let Value::Array(entries) = logs else {
        panic!("expected array");
    };
    let has_kv8 = entries.iter().any(|e| {
        let Value::Object(o) = e else {
            return false;
        };
        matches!(o.get("source"), Some(Value::String(s)) if s == "kv8")
            && matches!(o.get("message"), Some(Value::String(s)) if s.contains("from kv8"))
    });
    assert!(has_kv8);
}

#[test]
fn ext_install_and_list() {
    let list = eval(
        r#"
        ext_install(`{"name":"TestExt","version":"0.1"}`);
        ext_list();
        "#,
    );
    let Value::Array(exts) = list else {
        panic!("expected array");
    };
    assert!(exts.len() >= 1);
    let has_test = exts.iter().any(|e| {
        let Value::Object(o) = e else {
            return false;
        };
        matches!(o.get("name"), Some(Value::String(s)) if s == "TestExt")
    });
    assert!(has_test);
}

#[test]
fn ext_content_scripts_manifest() {
    let ext = eval(
        r#"
        ext_install(`{"name":"Injector","version":"1.0","content_scripts":["document.title='x';"]}`);
        "#,
    );
    let Value::Object(o) = ext else {
        panic!("expected object");
    };
    assert!(o.contains_key("id"));
    let info = eval("ext_info()");
    let Value::Object(meta) = info else {
        panic!("expected object");
    };
    assert!(matches!(meta.get("content_scripts"), Some(Value::String(s)) if s == "true"));
}

#[test]
fn pwa_install_creates_vfs_app() {
    let url = eval(
        r#"
        pwa_install(`{"name":"Demo","short_name":"demo","start_url":"/"}`);
        "#,
    );
    let Value::String(s) = url else {
        panic!("expected url");
    };
    assert!(s.contains("kabootar://vfs/apps/demo"));
}

#[test]
fn pwa_register_worker_and_fetch_cached() {
    let body = eval(
        r#"
        pwa_install(`{"name":"CacheApp","short_name":"cache","start_url":"/cache/"}`);
        pwa_register_worker("/cache/", "// sw");
        pwa_fetch_cached("/cache/");
        "#,
    );
    let Value::String(s) = body else {
        panic!("expected cached body after pwa_install seeds cache");
    };
    assert!(s.contains("---kml---"));
}

#[test]
fn pwa_dispatch_fetch_event_cache_first() {
    let out = eval(
        r#"
        pwa_register_worker("/app/", "self.addEventListener('fetch', (e) => {});");
        // seed offline + SW cache via install-style put
        pwa_install(`{"name":"FetchApp","short_name":"fetchapp","start_url":"/app/"}`);
        let r = pwa_dispatch_fetch("/app/");
        r["handled"] && r["from_cache"] && r["status"] == 200 && str_includes(r["body"], "---kml---")
        "#,
    );
    assert!(
        matches!(out, Value::Bool(true)),
        "fetch event cache-first failed: {out:?}"
    );
    let info = eval("pwa_info()");
    let Value::Object(o) = info else {
        panic!("expected pwa info");
    };
    assert!(matches!(o.get("fetch_events"), Some(Value::String(s)) if s == "true"));
    assert!(matches!(o.get("phase"), Some(Value::String(s)) if s == "C8"));
}

#[test]
fn pwa_on_fetch_network_stub_and_no_listener() {
    let stub = eval(
        r#"
        pwa_register_worker("/stub/", "// no listener yet");
        pwa_on_fetch("/stub/", "network-stub");
        let r = pwa_dispatch_fetch("/stub/x");
        r["handled"] && !r["from_cache"] && r["body"] == "network-stub:/stub/x"
        "#,
    );
    assert!(matches!(stub, Value::Bool(true)), "network-stub failed: {stub:?}");

    let quiet = eval(
        r#"
        pwa_register_worker("/quiet/", "// silent worker");
        let r = pwa_dispatch_fetch("/quiet/page");
        !r["handled"] && r["status"] == 0
        "#,
    );
    assert!(matches!(quiet, Value::Bool(true)), "no-listener failed: {quiet:?}");
}

#[test]
fn ext_permissions_gate_storage_and_tabs() {
    let granted = eval(
        r#"
        let e = ext_install(`{"name":"PermExt","version":"1.0","permissions":["storage"]}`);
        let id = e["id"];
        ext_has_permission(id, "storage")
          && !ext_has_permission(id, "tabs")
          && ext_storage_set(id, "k", "v")
          && ext_storage_get(id, "k") == "v"
          && ext_request_permission(id, "tabs")
          && ext_tabs_query(id)[0] == "kabootar://active"
        "#,
    );
    assert!(
        matches!(granted, Value::Bool(true)),
        "permission grant path failed: {granted:?}"
    );

    let mut env = create_global_env();
    let denied = eval_source(
        r#"
        let e = ext_install(`{"name":"NoPerm","version":"1.0"}`);
        ext_storage_get(e["id"], "x");
        "#,
        &mut env,
    );
    assert!(
        denied
            .as_ref()
            .err()
            .map(|e| e.contains("missing permission") || e.contains("storage"))
            .unwrap_or(false),
        "expected storage denial, got {denied:?}"
    );

    let info = eval("ext_info()");
    let Value::Object(o) = info else {
        panic!("expected ext info");
    };
    assert!(matches!(o.get("permissions"), Some(Value::String(s)) if s == "true"));
    assert!(matches!(o.get("phase"), Some(Value::String(s)) if s == "C8"));
}

#[test]
fn webgl_create_context() {
    let ctx = eval("webgl_create(640, 480)");
    let Value::Object(o) = ctx else {
        panic!("expected object");
    };
    assert!(o.contains_key("id"));
    assert!(o.contains_key("backend"));
}

#[test]
fn webgl_shader_clear_and_draw() {
    let ok = eval(
        r#"
        let gl = webgl_create(32, 32);
        let sh = webgl_shader("void main() { gl_Position = vec4(0.0); }", "void main() { }");
        webgl_use_program(gl["id"], sh);
        webgl_clear(gl["id"], 10, 20, 30, 255);
        webgl_draw(gl["id"], 3);
        "#,
    );
    assert!(matches!(ok, Value::Bool(true)));
}

#[test]
fn webrtc_peer_offer_flow() {
    let sdp = eval(
        r#"
        let peer = webrtc_create_peer();
        webrtc_create_offer(peer);
        "#,
    );
    let Value::String(s) = sdp else {
        panic!("expected sdp");
    };
    assert!(s.contains("v=0"));
    assert!(s.contains("a=ice-ufrag:"));
    assert!(s.contains("a=fingerprint:sha-256"));
    assert!(s.contains("a=setup:actpass"));
    assert!(s.contains("UDP/TLS/RTP/SAVPF"));
}

#[test]
fn webrtc_gather_ice_add_track_and_stats() {
    let stats = eval(
        r#"
        let peer = webrtc_create_peer();
        webrtc_add_track(peer, "video");
        let ice = webrtc_gather_ice(peer);
        webrtc_stats(peer);
        "#,
    );
    let Value::Object(o) = stats else {
        panic!("expected stats object");
    };
    assert!(matches!(o.get("tracks"), Some(Value::String(s)) if s == "1"));
    assert!(matches!(o.get("candidates"), Some(Value::String(s)) if s.parse::<u32>().unwrap_or(0) >= 2));
}

#[test]
fn webrtc_rtp_send_and_recv() {
    let pkts = eval(
        r#"
        let peer = webrtc_create_peer();
        let track = webrtc_add_track(peer, "audio");
        webrtc_send_rtp(peer, track, "hello-rtp");
        webrtc_recv_rtp(peer);
        "#,
    );
    let Value::Array(items) = pkts else {
        panic!("expected rtp packets");
    };
    assert_eq!(items.len(), 1);
}

#[test]
fn webrtc_dtls_srtp_peer_bridge() {
    let out = eval(
        r#"
        let a = webrtc_create_peer();
        let b = webrtc_create_peer();
        webrtc_add_track(a, "audio");
        webrtc_connect_peers(a, b);
        let track = webrtc_add_track(a, "audio");
        webrtc_send_rtp(a, track, "srtp-hello");
        let pkts = webrtc_recv_rtp(b);
        let sa = webrtc_stats(a);
        let sb = webrtc_stats(b);
        len(pkts) == 1
            && pkts[0]["payload"] == "srtp-hello"
            && sa["dtls_state"] == "Connected"
            && sb["dtls_state"] == "Connected"
            && sa["srtp_protect"] == "1"
            && sb["srtp_unprotect"] == "1"
        "#,
    );
    assert!(
        matches!(out, Value::Bool(true)),
        "DTLS-SRTP bridge failed: {out:?}"
    );
}

#[test]
fn webrtc_create_answer_after_offer() {
    let out = eval(
        r#"
        let offerer = webrtc_create_peer();
        let answerer = webrtc_create_peer();
        let offer = webrtc_create_offer(offerer);
        webrtc_set_remote(answerer, offer);
        let answer = webrtc_create_answer(answerer);
        webrtc_set_remote(offerer, answer);
        webrtc_gather_ice(offerer);
        webrtc_gather_ice(answerer);
        let so = webrtc_stats(offerer);
        let sa = webrtc_stats(answerer);
        string_includes(answer, "a=setup:active")
            && string_includes(answer, "a=fingerprint:sha-256")
            && so["dtls_state"] == "Connected"
            && sa["dtls_state"] == "Connected"
        "#,
    );
    assert!(
        matches!(out, Value::Bool(true)),
        "answer/DTLS flow failed: {out:?}"
    );
}

#[test]
fn webrtc_configure_turn_server() {
    let info = eval(
        r#"
        webrtc_configure_ice(`{"urls":["turn:relay.kabootar:3478"],"username":"u","credential":"p"}`);
        webrtc_info();
        "#,
    );
    let Value::Object(o) = info else {
        panic!("expected object");
    };
    assert!(matches!(o.get("ice"), Some(Value::String(s)) if s.contains("turn")));
    assert!(matches!(o.get("phase"), Some(Value::String(s)) if s == "C7"));
    assert!(matches!(o.get("dtls"), Some(Value::String(s)) if s.contains("fingerprint")));
    assert!(matches!(o.get("srtp"), Some(Value::String(_))));
}

#[test]
fn webgl_vertex_index_buffers_and_draw_elements() {
    let ok = eval(
        r#"
        let gl = webgl_create(64, 64);
        let vbo = webgl_create_buffer("array", [-0.8, -0.8, 0.8, -0.8, 0.0, 0.8]);
        let ibo = webgl_create_index_buffer([0, 1, 2]);
        webgl_bind_buffer(gl["id"], vbo);
        webgl_bind_buffer(gl["id"], ibo);
        webgl_draw_elements(gl["id"], 3, 0);
        "#,
    );
    assert!(matches!(ok, Value::Bool(true)));
    let info = eval("webgl_info()");
    let Value::Object(o) = info else {
        panic!("expected webgl info");
    };
    assert!(matches!(o.get("buffers"), Some(Value::String(s)) if s.contains("array")));
}

#[test]
fn devtools_dom_tree_from_active_document() {
    let tree = eval(
        r#"
        let page = kml("<html><body><h1 id='t'>Hi</h1></body></html>");
        kb_mount(page);
        devtools_dom_tree();
        "#,
    );
    let Value::Object(root) = tree else {
        panic!("expected dom tree object");
    };
    assert!(matches!(root.get("tag"), Some(Value::String(s)) if s == "html"));
}

#[test]
fn wasm_info_reports_guest_engine() {
    let info = eval("wasm_info()");
    let Value::Object(o) = info else {
        panic!("expected object");
    };
    assert!(matches!(o.get("guest"), Some(Value::String(s)) if s == "true"));
}

#[test]
fn wasm_run_executes_add_export() {
    let m = load_wasm("add", minimal_add_wasm()).unwrap();
    let sum = eval(&format!(r#"wasm_run({}, "add", [5, 7])"#, m.id));
    assert!(matches!(sum, Value::Number(12)));
}

#[test]
fn devtools_breakpoint_and_source_map() {
    let ok = eval(
        r#"
        devtools_breakpoint("/apps/demo.kv8", 12);
        devtools_source_map("bundle.js", "/apps/demo.kv8");
        "#,
    );
    assert!(matches!(ok, Value::Null));
}

#[test]
fn c9_devtools_network_panel() {
    let out = eval(
        r#"
        devtools_network_clear();
        let e = devtools_network_record("GET", "https://api.kabootar/v1", 200, 128, 12.5, "test");
        let dump = devtools_network_dump();
        let info = devtools_info();
        e["status"] == 200
          && dump[0]["url"] == "https://api.kabootar/v1"
          && info["network"] == "true"
          && info["phase"] == "C9"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "network panel failed: {out:?}");
}

#[test]
fn c9_devtools_profiler() {
    let out = eval(
        r#"
        devtools_profile_start("c9");
        devtools_profile_mark("start");
        devtools_profile_mark("end");
        let d = devtools_profile_measure("span", "start", "end");
        let stop = devtools_profile_stop();
        let dump = devtools_profile_dump();
        d >= 0
          && stop["label"] == "c9"
          && dump["measures"][0]["name"] == "span"
          && len(dump["marks"]) >= 2
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "profiler failed: {out:?}");
}

#[test]
fn c9_devtools_live_edit() {
    let out = eval(
        r#"
        let page = kml("<html><body><p id='p'>old</p></body></html>");
        kb_mount(page);
        let tree = devtools_dom_tree();
        let p = tree["children"][0]["children"][0];
        let id = p["id"];
        let ok_text = devtools_live_edit(id, "text", "live");
        let ok_attr = devtools_live_edit(id, "attr", "data-x", "1");
        let ev = devtools_live_eval("1 + 2");
        let info = devtools_info();
        ok_text && ok_attr && ev == 3 && info["live_edit"] == "true"
        "#,
    );
    assert!(matches!(out, Value::Bool(true)), "live edit failed: {out:?}");
}
