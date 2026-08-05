# IoT (`import "iot"`)

Kab-first **Internet of Things** MVP: MQTT-shaped pub/sub, sensor abstractions, CoAP / BLE / Zigbee stubs — pairs with OS USB/HID and future `sim` digital twins.

## Quick start

```kab
import "iot"

let bus = createBroker()
let linked = connect(bus, "edge-1")
bus = linked["broker"]
bus = subscribe(bus, "edge-1", "iot/sensors/#")

let temp = createTemperature({ "base": 21.0 })
let s = sample(temp, 0.2)
bus = publish(bus, topicFor(s["sensor"]), s["reading"])
let msgs = poll(bus, "edge-1")["messages"]
```

## API (MVP)

| Area | Functions |
|------|-----------|
| MQTT (memory) | `createBroker`, `connect`, `subscribe`, `publish`, `poll`, `disconnect` |
| MQTT (TCP) | `connectTcp` — **stub** (real codec deferred) |
| Sensors | `createTemperature`, `createHumidity`, `createAccelerometer`, `sample`, `attachUsb`, `topicFor` |
| CoAP | `createEndpoint`, `get`, `put`, `observe` (stub / local resource map) |
| Radio | `createBleAdapter`, `bleScan`, `bleConnect`, `createZigbeeNet`, `zigbeePermitJoin`, `zigbeeNodes` |

## Files

- `lib/iot.kab` — `pub import` surface
- `lib/iot/mqtt.kab`, `sensors.kab`, `coap.kab`, `radio.kab`
- `examples/iot_sensors_mqtt.kab`
- `tests/iot_module.rs`

Roadmap: [ROADMAP.md](ROADMAP.md) **Våg IOT**.
