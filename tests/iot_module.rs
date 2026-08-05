//! IoT module MVP — MQTT memory bus, sensors, protocol stubs.

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
fn iot_mqtt_sensors_stubs() {
    let v = eval(
        r#"
        import "iot"
        let bus = createBroker()
        let linked = connect(bus, "c1")
        bus = linked["broker"]
        bus = subscribe(bus, "c1", "iot/sensors/#")
        let temp = createTemperature({ "base": 20.0 })
        let s = sample(temp, 0.5)
        bus = publish(bus, topicFor(s["sensor"]), s["reading"])
        let polled = poll(bus, "c1")
        let msgs = polled["messages"]
        let ep = createEndpoint()
        let putRes = put(ep, "/x", 1)
        let ble = bleScan(createBleAdapter(), 100)
        let tcp = connectTcp("127.0.0.1", 1883)
        len(msgs) == 1 && msgs[0]["payload"]["type"] == "temperature" && putRes["ok"] && ble["mode"] == "stub" && tcp["transport"] == "stub"
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
