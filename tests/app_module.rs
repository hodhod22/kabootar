//! App module MVP — nav, lifecycle, ui, i18n, offline, stubs.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn env_host() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

fn eval(code: &str) -> Value {
    env_host();
    let mut env = create_global_env();
    eval_source(code, &mut env).expect("eval")
}

#[test]
fn app_shell_nav_lifecycle_i18n() {
    let v = eval(
        r#"
        import "app"
        let app = createApp("t", { "title": "T" })
        app = start(app)
        let nav = createStack("home")
        nav = pushRoute(nav, "x", { "a": 1 })
        nav = popRoute(nav)
        app = attachNav(app, nav)
        let screen = createScreen("s", 100, 200)
        let panel = createPanel("p", 0, 0, 100, 100)
        panel = addChild(panel, createButton("b", "Go", 0, 0, 40, 20))
        panel = layoutColumn(panel, 4)
        let cat = createCatalog("en", { "hi": "Hi {name}" })
        cat = addLocale(cat, "sv", { "hi": "Hej {name}" })
        cat = setLocale(cat, "sv")
        let cache = createOfflineCache()
        cache = cachePut(cache, "/u", "1")
        let geo = getCurrentPosition(createGeolocation())
        let sh = shareText("x")
        let n = show("hi", "body")
        let m = createManifest({ "id": "demo", "entry": "main.kab" })
        let planOk = plan(m)["ok"]
        let intent = createIntent("VIEW", "kabootar://x")
        isForeground(app) && currentRoute(nav)["name"] == "home" && t(cat, "hi", { "name": "A" }) == "Hej A" && cacheHas(cache, "/u") && geo["coords"]["latitude"] != null && sh["mode"] == "stub" && screen["h"] == 200 && n["kind"] == "local" && planOk && intent["bridge"] == "intent"
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
