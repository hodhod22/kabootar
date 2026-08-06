//! MIT pillars behind nabz / peyvand / rasme* kits.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn test_runtime_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

#[test]
fn doc_web_cad_pillars_smoke() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "doc/model"
        import "doc/block"
        import "web/app"
        import "web/router"
        import "cad/aero"
        import "cad/arch"
        import "cad/circuit"
        import "cad/power"

        let doc = createDocument("T")
        doc = docAddPage(doc, "p")
        doc = docAddTextBlock(doc, 0, "a")
        doc = docEditTextBlock(doc, 0, 0, "b")

        let app = createWebApp("s")
        app = webAddRoute(app, "/home", "Home")
        app = webSetRouteBody(app, "/home", "Hi")
        let r = webRenderRoute(app, "/home")
        let miss = webRenderRoute(app, "/x")

        let aero = createAeroModel("w")
        aero = aeroAddAirfoil(aero, "m", 1.0, 2.0)
        aero = aeroSetFlow(aero, 10.0, 2.0, 1.225)

        let b = createBuilding("b")
        b = archAddRoom(b, "r", 2.0, 3.0)

        let board = createBoard("c")
        board = circuitAddPart(board, "R1", "R", 1000.0)

        let inst = createInstall("i")
        inst = powerAddCircuit(inst, "G1", 16.0)
        inst = powerAddDevice(inst, "d", "G1", 100.0)

        docReadPageText(doc, 0) == "b"
            && r["ok"] == true
            && miss["ok"] == false
            && aeroEstimateLift(aero, 0) > 0.0
            && archFloorArea(b) == 6.0
            && circuitDividerVout(5.0, 1000.0, 1000.0) == 2.5
            && powerCircuitOk(inst, "G1") == true
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
