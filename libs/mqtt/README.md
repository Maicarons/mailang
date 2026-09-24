# mqtt — MQTT publish pack (stub)

**Status: stub only — no network.**

`publish()` records that a publish was requested and returns `Ok("sim")`.
It **does not** open sockets or speak MQTT. The **host must supply transport**
(e.g. a `mqtt_publish` host fn via `mailang_register_host_fn`, or a platform
MQTT client) before this is useful on a device.

## API (stub)

| Function | Returns | Stub behavior |
|----------|---------|---------------|
| `publish(topic, payload)` | `Ok("sim")` | Pretend publish; no I/O |

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

let r = mqtt.publish("devices/1/temp", "25.0")
println(r)
```

## Honest limits

- No QoS, retain, TLS, reconnect, or broker URL handling.
- Return value is a stub success marker — not an ACK from a broker.
- Real device: register a host `publish` (or wrap a C MQTT client) and call it
  from `lib.mai`.
