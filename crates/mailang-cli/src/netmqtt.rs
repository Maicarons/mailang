//! Real MQTT 3.1.1 transport for the `mqtt` pack, plus a built-in dev broker.
//!
//! Two halves share one packet codec:
//!
//! 1. **Client** — [`install`] registers the pack's `mqtt_*` hooks over a real
//!    TCP socket (enabled by `mailang run --mqtt`). The pack's stubs stay as
//!    the default; registration overrides them by name, exactly like the
//!    `tui` / `gui` backends.
//! 2. **Dev broker** — `mailang mqtt-broker --port 1883` runs a minimal MQTT
//!    3.1.1 broker (CONNECT / SUBSCRIBE / PUBLISH QoS 0 / PINGREQ /
//!    DISCONNECT) for local and CI smoke tests. It is **not** mosquitto; the
//!    point is that the client speaks standard MQTT 3.1.1 on real sockets, so
//!    the same code path also works against a real broker.
//!
//! QoS 0 only, by design: the pack's contract is a fire-and-forget publish
//! plus a poll-style receive, which QoS 0 covers without ack bookkeeping.

use mailang_core::{bytecode::Value, MailangInterpreter};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const IO_TIMEOUT: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------------------
// packet codec (shared by client and dev broker)
// ---------------------------------------------------------------------------

/// MQTT remaining-length varint (up to 4 bytes, 253 MiB cap).
fn encode_remaining_len(mut n: usize, out: &mut Vec<u8>) {
    assert!(n < 268_435_456, "packet too large for MQTT varint");
    loop {
        let mut byte = (n % 128) as u8;
        n /= 128;
        if n > 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if n == 0 {
            return;
        }
    }
}

fn decode_remaining_len(bytes: &[u8]) -> Option<usize> {
    let mut multiplier = 1usize;
    let mut value = 0usize;
    for &b in bytes {
        value += ((b & 0x7f) as usize) * multiplier;
        if b & 0x80 == 0 {
            return Some(value);
        }
        multiplier *= 128;
        if multiplier > 128 * 128 * 128 {
            return None; // malformed: more than 4 length bytes
        }
    }
    None
}

fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn read_u16(r: &mut impl Read) -> std::io::Result<u16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_be_bytes(b))
}

fn read_str(r: &mut impl Read) -> std::io::Result<String> {
    let len = read_u16(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// One MQTT control packet: fixed-header type/flags plus the variable body.
#[derive(Debug)]
struct Packet {
    /// Fixed-header first byte (type in the high nibble, flags in the low).
    header: u8,
    body: Vec<u8>,
}

impl Packet {
    fn new(header: u8, body: Vec<u8>) -> Packet {
        Packet { header, body }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        out.push(self.header);
        encode_remaining_len(self.body.len(), out);
        out.extend_from_slice(&self.body);
    }
}

/// Read one packet, honouring `deadline` between partial reads.
fn read_packet(stream: &mut TcpStream, deadline: Instant) -> Result<Packet, String> {
    let mut header = [0u8; 1];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("mqtt: timed out waiting for a packet".into());
        }
        stream
            .set_read_timeout(Some(remaining.min(IO_TIMEOUT)))
            .map_err(|e| e.to_string())?;
        match stream.read(&mut header) {
            Ok(1) => break,
            Ok(0) => return Err("mqtt: connection closed by peer".into()),
            Ok(_) => unreachable!(),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                continue; // re-check the deadline, then try again
            }
            Err(e) => return Err(format!("mqtt: read failed: {}", e)),
        }
    }

    // Varint length, one byte at a time (each byte is a separate read).
    let mut len_bytes = Vec::with_capacity(4);
    let len = loop {
        let mut b = [0u8; 1];
        stream
            .read_exact(&mut b)
            .map_err(|e| format!("mqtt: read failed: {}", e))?;
        len_bytes.push(b[0]);
        match decode_remaining_len(&len_bytes) {
            Some(n) => break n,
            None if len_bytes.len() >= 4 => return Err("mqtt: malformed packet length".into()),
            None => continue,
        }
    };

    let mut body = vec![0u8; len];
    stream
        .read_exact(&mut body)
        .map_err(|e| format!("mqtt: read failed: {}", e))?;
    Ok(Packet::new(header[0], body))
}

// ---------------------------------------------------------------------------
// client
// ---------------------------------------------------------------------------

/// A live MQTT 3.1.1 session over TCP. All operations are synchronous; the
/// host hooks call into this from the single-threaded VM.
struct MqttClient {
    stream: TcpStream,
    next_pid: u16,
}

impl MqttClient {
    fn connect(broker: &str, port: u16, client_id: &str) -> Result<MqttClient, String> {
        let addr = (broker, port)
            .to_socket_addrs()
            .map_err(|e| format!("mqtt.connect: cannot resolve '{}': {}", broker, e))?
            .next()
            .ok_or_else(|| format!("mqtt.connect: '{}' resolved to no address", broker))?;
        let mut stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
            .map_err(|e| format!("mqtt.connect: {}:{}: {}", broker, port, e))?;
        stream
            .set_nodelay(true)
            .map_err(|e| format!("mqtt.connect: {}", e))?;

        // CONNECT: protocol name/level, clean session, keepalive 60 s, client id.
        let mut body = Vec::new();
        push_str(&mut body, "MQTT");
        body.push(0x04); // protocol level (3.1.1)
        body.push(0x02); // clean session
        body.extend_from_slice(&60u16.to_be_bytes());
        push_str(&mut body, client_id);
        let mut out = Vec::new();
        Packet::new(0x10, body).encode(&mut out);
        stream
            .write_all(&out)
            .map_err(|e| format!("mqtt.connect: send failed: {}", e))?;

        let deadline = Instant::now() + IO_TIMEOUT;
        let pkt = read_packet(&mut stream, deadline)?;
        if pkt.header >> 4 != 0x2 {
            return Err(format!(
                "mqtt.connect: expected CONNACK, got packet type {:#x}",
                pkt.header >> 4
            ));
        }
        let rc = *pkt.body.get(1).unwrap_or(&0xff);
        if rc != 0 {
            return Err(format!("mqtt.connect: broker refused (CONNACK rc={})", rc));
        }
        Ok(MqttClient {
            stream,
            next_pid: 1,
        })
    }

    /// QoS 0 publish: fire and forget.
    fn publish(&mut self, topic: &str, payload: &str) -> Result<(), String> {
        let mut body = Vec::new();
        push_str(&mut body, topic);
        body.extend_from_slice(payload.as_bytes());
        let mut out = Vec::new();
        Packet::new(0x30, body).encode(&mut out); // QoS 0, no retain
        self.stream
            .write_all(&out)
            .map_err(|e| format!("mqtt.publish: send failed: {}", e))?;
        self.stream
            .flush()
            .map_err(|e| format!("mqtt.publish: flush failed: {}", e))
    }

    fn subscribe(&mut self, topic: &str) -> Result<(), String> {
        let pid = self.next_pid;
        self.next_pid = self.next_pid.wrapping_add(1).max(1);
        let mut body = Vec::new();
        body.extend_from_slice(&pid.to_be_bytes());
        push_str(&mut body, topic);
        body.push(0x00); // requested QoS 0
        let mut out = Vec::new();
        Packet::new(0x82, body).encode(&mut out);
        self.stream
            .write_all(&out)
            .map_err(|e| format!("mqtt.subscribe: send failed: {}", e))?;

        let deadline = Instant::now() + IO_TIMEOUT;
        loop {
            let pkt = read_packet(&mut self.stream, deadline)?;
            match pkt.header >> 4 {
                0x9 => {
                    // SUBACK: pid + one granted-QoS byte per topic.
                    let got_pid = pkt
                        .body
                        .get(0..2)
                        .map(|b| u16::from_be_bytes([b[0], b[1]]))
                        .unwrap_or(0);
                    if got_pid != pid {
                        return Err(format!(
                            "mqtt.subscribe: SUBACK pid mismatch ({} != {})",
                            got_pid, pid
                        ));
                    }
                    let granted = *pkt.body.get(2).unwrap_or(&0x80);
                    if granted == 0x80 {
                        return Err("mqtt.subscribe: broker rejected the filter".into());
                    }
                    return Ok(());
                }
                0xD => continue, // late PINGRESP, keep waiting
                other => {
                    return Err(format!(
                        "mqtt.subscribe: expected SUBACK, got packet type {:#x}",
                        other
                    ))
                }
            }
        }
    }

    /// Wait up to `timeout_ms` for one QoS 0 PUBLISH; returns (topic, payload).
    /// PINGRESP and late CONNACK/SUBACK packets are skipped; PINGREQ is
    /// answered so a broker keepalive sees the client as alive.
    fn poll(&mut self, timeout_ms: u64) -> Result<(String, String), String> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            let pkt = read_packet(&mut self.stream, deadline)?;
            match pkt.header >> 4 {
                0x3 => {
                    let qos = (pkt.header >> 1) & 0x03;
                    if qos != 0 {
                        return Err(format!(
                            "mqtt.poll: only QoS 0 deliveries are supported (got QoS {})",
                            qos
                        ));
                    }
                    let mut cursor = &pkt.body[..];
                    let topic = read_str(&mut cursor)
                        .map_err(|e| format!("mqtt.poll: bad topic: {}", e))?;
                    let text = String::from_utf8(cursor.to_vec())
                        .map_err(|_| "mqtt.poll: payload is not valid UTF-8".to_string())?;
                    return Ok((topic, text));
                }
                0xD => continue,       // PINGRESP
                0x2 | 0x9 => continue, // late CONNACK / SUBACK
                0xC => {
                    // PINGREQ (server-initiated keepalive check): answer it.
                    let mut out = Vec::new();
                    Packet::new(0xD0, Vec::new()).encode(&mut out);
                    let _ = self.stream.write_all(&out);
                    continue;
                }
                other => {
                    return Err(format!(
                        "mqtt.poll: unexpected packet type {:#x} while waiting for a message",
                        other
                    ))
                }
            }
        }
    }

    fn disconnect(&mut self) {
        let mut out = Vec::new();
        Packet::new(0xE0, Vec::new()).encode(&mut out);
        let _ = self.stream.write_all(&out);
        let _ = self.stream.flush();
    }
}

// ---------------------------------------------------------------------------
// host hooks (`--mqtt`)
// ---------------------------------------------------------------------------

fn as_str(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

fn as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Int(i) => Some(*i),
        Value::Float(f) => Some(*f as i64),
        _ => None,
    }
}

fn ok_value(v: Value) -> Result<Value, String> {
    Ok(Value::Ok(Box::new(v)))
}

/// "Nothing arrived" is a MaìLang `Err` value, not a host failure, so the
/// script can `match` on it (same protocol as `tui_read_key`).
fn err_value(msg: impl Into<String>) -> Result<Value, String> {
    Ok(Value::Err(Box::new(Value::Str(msg.into().into()))))
}

fn map_entry(k: &str, v: Value) -> (Value, Value) {
    (Value::Str(k.into()), v)
}

/// Register the real-transport `mqtt_*` hooks. The pack's stubs keep running
/// until this is called; afterwards registration wins by name.
pub fn install(interp: &mut MailangInterpreter) {
    let conn: Rc<RefCell<Option<MqttClient>>> = Rc::new(RefCell::new(None));

    let c = conn.clone();
    interp.register_host_fn("mqtt_connect", move |args| {
        let broker = as_str(args.first().unwrap_or(&Value::Null))
            .ok_or("mqtt.connect: broker must be a string")?;
        let port = as_int(args.get(1).unwrap_or(&Value::Null)).unwrap_or(1883) as u16;
        let client_id =
            as_str(args.get(2).unwrap_or(&Value::Null)).unwrap_or_else(|| "mailang".to_string());
        let client = MqttClient::connect(&broker, port, &client_id)?;
        *c.borrow_mut() = Some(client);
        ok_value(Value::Str("connected".into()))
    });

    let c = conn.clone();
    interp.register_host_fn("mqtt_publish", move |args| {
        let topic = as_str(args.first().unwrap_or(&Value::Null))
            .ok_or("mqtt.publish: topic must be a string")?;
        let payload = as_str(args.get(1).unwrap_or(&Value::Null))
            .ok_or("mqtt.publish: payload must be a string")?;
        let mut guard = c.borrow_mut();
        let client = guard
            .as_mut()
            .ok_or_else(|| "mqtt.publish: not connected".to_string())?;
        client.publish(&topic, &payload)?;
        ok_value(Value::Str("sent".into()))
    });

    let c = conn.clone();
    interp.register_host_fn("mqtt_subscribe", move |args| {
        let topic = as_str(args.first().unwrap_or(&Value::Null))
            .ok_or("mqtt.subscribe: topic must be a string")?;
        let mut guard = c.borrow_mut();
        let client = guard
            .as_mut()
            .ok_or_else(|| "mqtt.subscribe: not connected".to_string())?;
        client.subscribe(&topic)?;
        ok_value(Value::Str("subscribed".into()))
    });

    let c = conn.clone();
    interp.register_host_fn("mqtt_poll", move |args| {
        let timeout_ms = as_int(args.first().unwrap_or(&Value::Null)).unwrap_or(0);
        let mut guard = c.borrow_mut();
        let client = match guard.as_mut() {
            Some(client) => client,
            None => return err_value("mqtt.poll: not connected"),
        };
        match client.poll(timeout_ms as u64) {
            Ok((topic, payload)) => {
                let entries = vec![
                    map_entry("topic", Value::Str(topic.into())),
                    map_entry("payload", Value::Str(payload.into())),
                ];
                ok_value(Value::Map(Rc::new(RefCell::new(entries))))
            }
            Err(e) => err_value(e),
        }
    });

    let c = conn.clone();
    interp.register_host_fn("mqtt_disconnect", move |_args| {
        if let Some(client) = c.borrow_mut().as_mut() {
            client.disconnect();
        }
        *c.borrow_mut() = None;
        ok_value(Value::Str("closed".into()))
    });

    let c = conn.clone();
    interp.register_host_fn("mqtt_is_connected", move |_args| {
        Ok(Value::Bool(c.borrow().is_some()))
    });
}

// ---------------------------------------------------------------------------
// dev broker (`mailang mqtt-broker`)
// ---------------------------------------------------------------------------

/// One subscription: a topic filter plus the inbox feeding that client.
struct Subscription {
    filter: String,
    inbox: Arc<Mutex<VecDeque<(String, String)>>>,
}

type Registry = Arc<Mutex<Vec<Subscription>>>;
/// One client's delivery queue for a single subscription.
type Inbox = Arc<Mutex<VecDeque<(String, String)>>>;
/// Inboxes freshly created by the reader thread, picked up by the deliverer.
type InboxQueue = Arc<Mutex<Vec<Inbox>>>;

/// True when `filter` matches `topic` (MQTT wildcard rules for `+` / `#`).
fn filter_matches(filter: &str, topic: &str) -> bool {
    let f: Vec<&str> = filter.split('/').collect();
    let t: Vec<&str> = topic.split('/').collect();
    let mut i = 0;
    while i < f.len() {
        match f[i] {
            "#" => return true, // matches the rest including deeper levels
            "+" => {
                if i >= t.len() {
                    return false;
                }
            }
            level => {
                if i >= t.len() || t[i] != level {
                    return false;
                }
            }
        }
        i += 1;
    }
    f.len() == t.len()
}

fn send_packet(stream: &mut TcpStream, pkt: &Packet) -> Result<(), String> {
    let mut out = Vec::new();
    pkt.encode(&mut out);
    stream
        .write_all(&out)
        .map_err(|e| format!("broker: send failed: {}", e))
}

/// Serve one client. Two threads per connection: this one reads control
/// packets; a helper drains the client's inboxes and delivers QoS 0
/// PUBLISHes back over the same socket.
fn serve_client(mut stream: TcpStream, registry: Registry) -> Result<(), String> {
    let first = read_packet(&mut stream, Instant::now() + IO_TIMEOUT)?;
    if first.header >> 4 != 0x1 {
        return Err(format!(
            "broker: first packet was not CONNECT ({:#x})",
            first.header >> 4
        ));
    }
    send_packet(
        &mut stream,
        &Packet::new(0x20, vec![0x00, 0x00]), // session present = 0, rc = success
    )?;

    // Delivery thread: owns the write side of the socket while this thread
    // reads. New subscriptions are handed over through `new_inboxes`.
    let mut writer = stream
        .try_clone()
        .map_err(|e| format!("broker: stream clone failed: {}", e))?;
    let new_inboxes: InboxQueue = Arc::new(Mutex::new(Vec::new()));
    let delivery_watcher = new_inboxes.clone();
    thread::spawn(move || {
        let mut inboxes: Vec<Inbox> = Vec::new();
        loop {
            inboxes.append(&mut delivery_watcher.lock().unwrap());
            let mut alive = true;
            for inbox in &inboxes {
                let msgs: Vec<_> = inbox.lock().unwrap().drain(..).collect();
                for (topic, payload) in msgs {
                    let mut body = Vec::new();
                    push_str(&mut body, &topic);
                    body.extend_from_slice(payload.as_bytes());
                    if send_packet(&mut writer, &Packet::new(0x30, body)).is_err() {
                        alive = false;
                        break;
                    }
                }
                if !alive {
                    break;
                }
            }
            if !alive {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    loop {
        let pkt = read_packet(&mut stream, Instant::now() + IO_TIMEOUT)?;
        match pkt.header >> 4 {
            0x8 => {
                // SUBSCRIBE: pid, then (filter, qos) pairs — accept QoS 0 only.
                let pid = u16::from_be_bytes([
                    *pkt.body.first().unwrap_or(&0),
                    *pkt.body.get(1).unwrap_or(&0),
                ]);
                let mut cursor = &pkt.body[2..];
                let mut granted = Vec::new();
                while cursor.len() >= 3 {
                    let filter =
                        read_str(&mut cursor).map_err(|e| format!("broker: bad filter: {}", e))?;
                    let qos = cursor[0];
                    cursor = &cursor[1..];
                    if qos == 0 {
                        let inbox = Arc::new(Mutex::new(VecDeque::new()));
                        registry.lock().unwrap().push(Subscription {
                            filter,
                            inbox: inbox.clone(),
                        });
                        new_inboxes.lock().unwrap().push(inbox);
                        granted.push(0x00);
                    } else {
                        granted.push(0x80);
                    }
                }
                let mut body = pid.to_be_bytes().to_vec();
                body.extend_from_slice(&granted);
                send_packet(&mut stream, &Packet::new(0x90, body))?;
            }
            0x3 => {
                // PUBLISH QoS 0: decode and fan out to matching subscribers.
                let qos = (pkt.header >> 1) & 0x03;
                if qos != 0 {
                    return Err("broker: only QoS 0 publishes are supported".into());
                }
                let mut cursor = &pkt.body[..];
                let topic =
                    read_str(&mut cursor).map_err(|e| format!("broker: bad topic: {}", e))?;
                let payload = String::from_utf8_lossy(cursor).to_string();
                registry.lock().unwrap().retain(|s| {
                    if filter_matches(&s.filter, &topic) {
                        let mut inbox = s.inbox.lock().unwrap();
                        if inbox.len() < 256 {
                            inbox.push_back((topic.clone(), payload.clone()));
                            true // drop instead of blocking when a client stalls
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                });
            }
            0xC => send_packet(&mut stream, &Packet::new(0xD0, Vec::new()))?,
            0xE => return Ok(()), // DISCONNECT
            other => return Err(format!("broker: unexpected packet type {:#x}", other)),
        }
    }
}

/// Run the dev broker on an already-bound listener. Never returns while the
/// listener lives; each client is served on its own threads.
pub fn serve(listener: TcpListener) -> ! {
    let registry: Registry = Arc::new(Mutex::new(Vec::new()));
    loop {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let registry = registry.clone();
                thread::spawn(move || {
                    if let Err(e) = serve_client(stream, registry) {
                        eprintln!("mqtt-broker: client dropped: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("mqtt-broker: accept failed: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_len_roundtrip() {
        for n in [0usize, 1, 127, 128, 16_383, 16_384, 2_097_151, 2_097_152] {
            let mut enc = Vec::new();
            encode_remaining_len(n, &mut enc);
            assert_eq!(decode_remaining_len(&enc), Some(n), "n={}", n);
            assert!(enc.len() <= 4);
        }
    }

    #[test]
    fn remaining_len_rejects_overlong() {
        assert_eq!(decode_remaining_len(&[0x80, 0x80, 0x80, 0x80, 0x01]), None);
    }

    #[test]
    fn filter_matching() {
        assert!(filter_matches("a/b", "a/b"));
        assert!(!filter_matches("a/b", "a/b/c"));
        assert!(!filter_matches("a/b/c", "a/b"));
        assert!(filter_matches("a/#", "a/b/c"));
        assert!(!filter_matches("a/#", "b/c"));
        assert!(filter_matches("a/+/c", "a/b/c"));
        assert!(!filter_matches("a/+/c", "a/b/d"));
        assert!(!filter_matches("+/b", "a/b/c"));
    }

    /// Full transport roundtrip over real sockets against the in-process dev
    /// broker: connect → subscribe → publish (second client) → poll receives.
    #[test]
    fn client_roundtrip_against_dev_broker() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || serve(listener));

        let mut sub = MqttClient::connect("127.0.0.1", addr.port(), "subscriber").unwrap();
        sub.subscribe("demo/#").unwrap();

        let mut pub1 = MqttClient::connect("127.0.0.1", addr.port(), "publisher").unwrap();
        pub1.publish("demo/temp", "21.5").unwrap();

        let (topic, payload) = sub.poll(3000).unwrap();
        assert_eq!(topic, "demo/temp");
        assert_eq!(payload, "21.5");

        // Nothing else is queued: a short poll must report an Err, not hang.
        assert!(sub.poll(200).is_err());

        sub.disconnect();
        pub1.disconnect();
    }

    /// `mqtt_is_connected` resolves through the host table even before any
    /// connection exists.
    #[test]
    fn install_registers_all_hooks() {
        let mut interp = MailangInterpreter::new();
        install(&mut interp);
        let out = interp
            .eval("mqtt_is_connected()")
            .expect("mqtt_is_connected callable");
        assert_eq!(out, "false");
    }
}
