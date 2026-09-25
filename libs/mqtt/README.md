# mqtt — MQTT client pack

**Status: host-fn contract + pure-Mai simulation.** No sockets, no broker, no
MQTT handshake on desktop. Publishes are recorded in an in-memory outbox.

## Host-function contract

Real network delivery requires the host to register these via
`mailang_register_host_fn` (C FFI) or `Vm::register_host_fn` (Rust).
Host registration **overrides** the stubs in `lib.mai` at call time.

| Host function | Signature | Returns |
|---------------|-----------|---------|
| `mqtt_connect` | `(broker:str, port:int, client_id:str)` | `Ok` / `Err(str)` |
| `mqtt_publish` | `(topic:str, payload:str)` | `Ok` / `Err(str)` |
| `mqtt_subscribe` | `(topic:str)` | `Ok` / `Err(str)` |
| `mqtt_disconnect` | `()` | `Ok` / `Err(str)` |
| `mqtt_is_connected` | `()` | `bool` |

C sketch:

```c
/* wrap esp-mqtt / lwMQTT / mosquitto / … */
mailang_register_host_fn(interp, "mqtt_connect",    host_mqtt_connect,    NULL);
mailang_register_host_fn(interp, "mqtt_publish",    host_mqtt_publish,    NULL);
mailang_register_host_fn(interp, "mqtt_subscribe",  host_mqtt_subscribe,  NULL);
mailang_register_host_fn(interp, "mqtt_disconnect", host_mqtt_disconnect, NULL);
mailang_register_host_fn(interp, "mqtt_is_connected", host_mqtt_is_connected, NULL);
```

The public API (`mqtt.connect` / `mqtt.publish` / …) calls these **by name**,
so a registered host function wins over the sim stub.

## Pure-Mai simulation (default)

| Function | Returns | Sim behavior |
|----------|---------|--------------|
| `connect(broker, port, client_id)` | `Ok("sim")` | records broker/client_id, sets connected |
| `publish(topic, payload)` | `Ok("sim")` | appends `{topic,payload}` to outbox |
| `subscribe(topic)` | `Ok("sim")` | appends to subscriptions |
| `disconnect()` | `Ok("sim")` | clears connected |
| `is_connected()` | bool | in-memory flag |
| `outbox()` / `outbox_len()` | array / int | inspect sim publishes |
| `clear_outbox()` | null | empty the log |
| `subscriptions()` | array | subscribed topics |
| `reset()` | null | full sim state reset |
| `set_mode(m)` / `mode()` | str | currently `"sim"` (host override is automatic) |

## Layout

```
mqtt/
├── mailib.ini
├── lib.mai
├── example/basic.mai
└── test/test_mqtt.mai
```

## Usage

```mai
import "mqtt"

mqtt.connect("broker.local", 1883, "dev-1")
let r = mqtt.publish("devices/1/temp", "25.0")
println(r)          // Ok("sim") on desktop
println(mqtt.outbox())
mqtt.disconnect()
```

## Tests

```bash
# from repo root
cargo run -p mailang-cli --release -- run libs/mqtt/test/test_mqtt.mai
```

Exit code `0` on success, `1` on any failure.

## Honest limits

- **No network I/O in this pack.** `Ok("sim")` is not a broker ACK.
- No QoS, retain, TLS, reconnect, will, or broker URL parsing in `.mai`
  (host transport may implement those underneath the same hook names).
- Real device: register the host fns above (e.g. wrap esp-mqtt) and call the
  same public API. Host fns take priority over the stubs.
- We did **not** run against a live broker in this repo.
