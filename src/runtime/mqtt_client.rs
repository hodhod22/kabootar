//! Minimal MQTT 3.1.1 client over TCP (QoS 0) — IOT3.

use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static NEXT: AtomicU64 = AtomicU64::new(1);

struct MqttConn {
    stream: TcpStream,
    client_id: String,
}

fn conns() -> &'static Mutex<HashMap<u64, MqttConn>> {
    static C: OnceLock<Mutex<HashMap<u64, MqttConn>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn encode_remaining_length(mut len: usize, out: &mut Vec<u8>) {
    loop {
        let mut dig = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            dig |= 0x80;
        }
        out.push(dig);
        if len == 0 {
            break;
        }
    }
}

fn encode_string(s: &str, out: &mut Vec<u8>) {
    let b = s.as_bytes();
    out.push((b.len() >> 8) as u8);
    out.push((b.len() & 0xff) as u8);
    out.extend_from_slice(b);
}

fn build_connect(client_id: &str) -> Vec<u8> {
    let mut vh = Vec::new();
    encode_string("MQTT", &mut vh);
    vh.push(4);
    vh.push(0x02);
    vh.push(0);
    vh.push(60);
    encode_string(client_id, &mut vh);
    let mut pkt = vec![0x10];
    encode_remaining_length(vh.len(), &mut pkt);
    pkt.extend_from_slice(&vh);
    pkt
}

fn build_publish(topic: &str, payload: &[u8]) -> Vec<u8> {
    let mut vh = Vec::new();
    encode_string(topic, &mut vh);
    vh.extend_from_slice(payload);
    let mut pkt = vec![0x30];
    encode_remaining_length(vh.len(), &mut pkt);
    pkt.extend_from_slice(&vh);
    pkt
}

fn build_subscribe(topic: &str, packet_id: u16) -> Vec<u8> {
    let mut vh = Vec::new();
    vh.push((packet_id >> 8) as u8);
    vh.push((packet_id & 0xff) as u8);
    encode_string(topic, &mut vh);
    vh.push(0);
    let mut pkt = vec![0x82];
    encode_remaining_length(vh.len(), &mut pkt);
    pkt.extend_from_slice(&vh);
    pkt
}

fn build_disconnect() -> Vec<u8> {
    vec![0xe0, 0x00]
}

fn read_connack(stream: &mut TcpStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    let mut hdr = [0u8; 2];
    stream
        .read_exact(&mut hdr)
        .map_err(|e| format!("CONNACK read: {e}"))?;
    if hdr[0] != 0x20 {
        return Err(format!("expected CONNACK, got {:#x}", hdr[0]));
    }
    let rem = hdr[1] as usize;
    let mut rest = vec![0u8; rem.min(16)];
    if rem > 0 {
        let n = rem.min(rest.len());
        stream
            .read_exact(&mut rest[..n])
            .map_err(|e| format!("CONNACK body: {e}"))?;
    }
    if rest.len() >= 2 && rest[1] != 0 {
        return Err(format!("CONNACK refused code {}", rest[1]));
    }
    Ok(())
}

fn value_bytes(v: &Value) -> Result<Vec<u8>, String> {
    match v {
        Value::String(s) => Ok(s.as_bytes().to_vec()), Value::Array(items) => {
            let mut out = Vec::new();
            for it in items.iter() {
                match it {
                    Value::Number(n) => out.push(*n as u8),
                    Value::Float(f) => out.push(*f as u8),
                    _ => return Err("mqtt payload array must be bytes".into()),
                }
            }
            Ok(out)
        }
        other => Ok(crate::value::format_value(other).into_bytes()),
    }
}

fn mqtt_connect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let host = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("mqtt_connect(host, port, clientId?)".into()),
    };
    let port = match args.get(1) {
        Some(Value::Number(n)) => *n as u16,
        Some(Value::Float(f)) => *f as u16,
        _ => 1883,
    };
    let client_id = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => format!("kab-{}", NEXT.load(Ordering::Relaxed)),
    };
    let mut stream = TcpStream::connect((host, port)).map_err(|e| format!("mqtt_connect: {e}"))?;
    let _ = stream.set_nodelay(true);
    let pkt = build_connect(&client_id);
    stream
        .write_all(&pkt)
        .map_err(|e| format!("CONNECT write: {e}"))?;
    read_connack(&mut stream)?;
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    conns()
        .lock()
        .map_err(|e| e.to_string())?
        .insert(
            id,
            MqttConn {
                stream,
                client_id: client_id.clone(),
            },
        );
    let mut m = HashMap::new();
    m.insert("kind".into(), Value::String("mqtt_tcp".into()));
    m.insert("id".into(), Value::Number(id as i64));
    m.insert("transport".into(), Value::String("tcp".into()));
    m.insert("connected".into(), Value::Bool(true));
    m.insert("host".into(), Value::String(host.into()));
    m.insert("port".into(), Value::Number(port as i64));
    m.insert("clientId".into(), Value::String(client_id));
    Ok(Value::from_object(m))
}

fn mqtt_id(args: &[Value]) -> Result<u64, String> {
    match args.first() {
        Some(Value::Object(m)) => match m.get("id") {
            Some(Value::Number(n)) => Ok(*n as u64),
            _ => Err("mqtt client missing id".into()),
        },
        Some(Value::Number(n)) => Ok(*n as u64),
        _ => Err("mqtt_* expects client handle".into()),
    }
}

fn mqtt_publish_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = mqtt_id(args)?;
    let topic = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("mqtt_publish(client, topic, payload)".into()),
    };
    let payload = value_bytes(args.get(2).ok_or("mqtt_publish payload")?)?;
    let mut guard = conns().lock().map_err(|e| e.to_string())?;
    let conn = guard.get_mut(&id).ok_or("mqtt client not connected")?;
    let pkt = build_publish(topic, &payload);
    conn.stream
        .write_all(&pkt)
        .map_err(|e| format!("mqtt_publish: {e}"))?;
    Ok(Value::Bool(true))
}

fn mqtt_subscribe_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = mqtt_id(args)?;
    let topic = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("mqtt_subscribe(client, topic)".into()),
    };
    let mut guard = conns().lock().map_err(|e| e.to_string())?;
    let conn = guard.get_mut(&id).ok_or("mqtt client not connected")?;
    let pkt = build_subscribe(topic, 1);
    conn.stream
        .write_all(&pkt)
        .map_err(|e| format!("mqtt_subscribe: {e}"))?;
    Ok(Value::Bool(true))
}

fn mqtt_disconnect_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = mqtt_id(args)?;
    let mut guard = conns().lock().map_err(|e| e.to_string())?;
    if let Some(mut conn) = guard.remove(&id) {
        let _ = conn.stream.write_all(&build_disconnect());
    }
    Ok(Value::Bool(true))
}

fn mqtt_try_connect_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    match mqtt_connect_native(args, env) {
        Ok(v) => Ok(v),
        Err(e) => {
            let host = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => "127.0.0.1".into(),
            };
            let port = match args.get(1) {
                Some(Value::Number(n)) => *n,
                Some(Value::Float(f)) => *f as i64,
                _ => 1883,
            };
            let mut m = HashMap::new();
            m.insert("kind".into(), Value::String("mqtt_client".into()));
            m.insert("transport".into(), Value::String("stub".into()));
            m.insert("connected".into(), Value::Bool(false));
            m.insert("host".into(), Value::String(host));
            m.insert("port".into(), Value::Number(port));
            m.insert("reason".into(), Value::String(e));
            Ok(Value::from_object(m))
        }
    }
}

pub fn register(
    bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>),
) {
    bind(&["mqtt_connect", "mqtt_tcp_connect"], mqtt_connect_native);
    bind(&["mqtt_try_connect"], mqtt_try_connect_native);
    bind(&["mqtt_publish", "mqtt_tcp_publish"], mqtt_publish_native);
    bind(&["mqtt_subscribe", "mqtt_tcp_subscribe"], mqtt_subscribe_native);
    bind(&["mqtt_disconnect", "mqtt_tcp_disconnect"], mqtt_disconnect_native);
}
