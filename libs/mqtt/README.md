# mqtt — MQTT client pack

**Status: real transport + host-fn contract + pure-Mai simulation.** The
`mailang` CLI ships a real MQTT 3.1.1 client (`mailang run --mqtt`, QoS 0 over
TCP) and a dev broker (`mailang mqtt-broker`). Without `--mqtt` the pack stays
a pure-Mai simulation: no sockets, publishes recorded in an in-memory outbox.

## Host-function contract

Real network delivery requires the host to register these via
`mailang_register_host_fn` (C FFI) or `Vm::register_host_fn` (Rust).
Host registration **overrides** the stubs in `lib.mai` at call time.

| Host function | Signature | Returns |
|---------------|-----------|---------|
| `mqtt_connect` | `(broker:str, port:int, client_id:str)` | `Ok` / `Err(str)` |
| `mqtt_publish` | `(topic:str, payload:str)` | `Ok` / `Err(str)` |
| `mqtt_subscribe` | `(topic:str)` | `Ok` / `Err(str)` |
| `mqtt_poll` | `(timeout_ms:int)` | `Ok({topic,payload})` / `Err(str)` |
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

## Real transport (mailang CLI, `--mqtt`)

`mailang run --mqtt app.mai` registers all six hooks with a standard MQTT
3.1.1 client (QoS 0, clean session, keepalive 60 s) and speaks real TCP —
against `mailang mqtt-broker`, mosquitto, or any 3.1.1 broker:

```bash
# terminal 1: a local broker (dev tool, QoS 0 — not a production broker)
mailang mqtt-broker --port 1883

# terminal 2: run the app with real transport
mailang run --mqtt app.mai

# real-transport host suite (what CI runs):
mailang run --mqtt libs/mqtt/test/host_mqtt.mai
```

`mqtt.poll(timeout_ms)` returns `Ok({topic, payload})` for one delivered QoS 0
message, or `Err("mqtt.poll: …")` when nothing arrives in time (the script
`match`es on it — "nothing available" is a value, not a host failure).


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
| `poll(timeout_ms)` | `Ok({topic,payload})` / `Err` | pops the sim inbox (LIFO); `Err` when empty |
| `mqtt_test_inject(topic, payload)` | `Ok("injected")` | sim-only: feed the inbox `poll` drains |
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
