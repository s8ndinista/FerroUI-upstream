use crate::{
    protocol::{self, Session},
    Client,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{self, BufRead, Read, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
const MAX_MESSAGE: usize = 1024 * 1024;
const MAX_WORKERS: usize = 16;

fn spawn_request(
    client: Client,
    session: Arc<Session>,
    message: Value,
    writer: Arc<Mutex<std::fs::File>>,
    workers: Arc<AtomicUsize>,
) {
    if workers.fetch_add(1, Ordering::AcqRel) >= MAX_WORKERS {
        workers.fetch_sub(1, Ordering::Release);
        if let Some(id) = message.get("id") {
            let _ = writeln!(
                writer.lock().unwrap(),
                "{}",
                protocol::error(id.clone(), -32000, "Too many requests")
            );
        }
        return;
    }
    std::thread::spawn(move || {
        if let Some(response) = session.handle(&client, message) {
            let mut output = writer.lock().unwrap();
            let _ = writeln!(output, "{response}");
            let _ = output.flush();
        }
        workers.fetch_sub(1, Ordering::Release);
    });
}
pub fn stdio(client: Client, protocol_output: std::fs::File) -> io::Result<()> {
    let session = Arc::new(Session::default());
    let output = Arc::new(Mutex::new(protocol_output));
    let workers = Arc::new(AtomicUsize::new(0));
    let logger_client = client.clone();
    let logger_session = session.clone();
    let logger_output = output.clone();
    std::thread::spawn(move || {
        let mut cursor = 0;
        while !logger_client.stop.load(Ordering::Acquire) {
            for message in logger_session.logs(&logger_client, &mut cursor) {
                let mut out = logger_output.lock().unwrap();
                let _ = writeln!(out, "{message}");
                let _ = out.flush();
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    let mut input = io::stdin().lock();
    while !client.stop.load(Ordering::Acquire) {
        let mut bytes = Vec::new();
        let n = input
            .by_ref()
            .take((MAX_MESSAGE + 1) as u64)
            .read_until(b'\n', &mut bytes)?;
        if n == 0 {
            break;
        }
        if n > MAX_MESSAGE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MCP message exceeds 1 MiB",
            ));
        }
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(message) if message.get("id").is_none() => {
                if let Some(response) = session.handle(&client, message) {
                    let mut out = output.lock().unwrap();
                    writeln!(out, "{response}")?;
                    out.flush()?;
                }
            }
            Ok(message) => spawn_request(
                client.clone(),
                session.clone(),
                message,
                output.clone(),
                workers.clone(),
            ),
            Err(_) => {
                let mut out = output.lock().unwrap();
                writeln!(
                    out,
                    "{}",
                    protocol::error(Value::Null, -32700, "Parse error")
                )?;
                out.flush()?;
            }
        }
    }
    client.stop.store(true, Ordering::Release);
    Ok(())
}
fn header<'a>(request: &'a tiny_http::Request, name: &str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}
fn respond(request: tiny_http::Request, status: u16, body: String) {
    let response = tiny_http::Response::from_string(body)
        .with_status_code(status)
        .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap());
    let _ = request.respond(response);
}
type Sessions = Arc<Mutex<HashMap<String, (Arc<Session>, Instant)>>>;
pub fn http(server: tiny_http::Server, client: Client, token: String) {
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));
    let workers = Arc::new(AtomicUsize::new(0));
    let address = server.server_addr().to_ip().unwrap();
    let port = address.port();
    let mut allowed_hosts = vec![address.to_string(), format!("localhost:{port}")];
    if port == 80 {
        allowed_hosts.push(address.ip().to_string());
        allowed_hosts.push("localhost".into());
    }
    let allowed_origins = allowed_hosts
        .iter()
        .map(|host| format!("http://{host}"))
        .collect::<Vec<_>>();
    while !client.stop.load(Ordering::Acquire) {
        let request = match server.recv_timeout(Duration::from_millis(100)) {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(_) => break,
        };
        if request.url() != "/mcp" {
            respond(request, 404, String::new());
            continue;
        }
        if !header(&request, "Host").is_some_and(|h| allowed_hosts.iter().any(|a| a == h))
            || header(&request, "Origin").is_some_and(|o| !allowed_origins.iter().any(|a| a == o))
        {
            respond(request, 403, String::new());
            continue;
        }
        if header(&request, "Authorization") != Some(format!("Bearer {token}").as_str()) {
            respond(request, 401, String::new());
            continue;
        }
        if header(&request, "MCP-Protocol-Version")
            .is_some_and(|v| !protocol::VERSIONS.contains(&v))
        {
            respond(request, 400, String::new());
            continue;
        }
        if request.method() != &tiny_http::Method::Post
            && request.method() != &tiny_http::Method::Get
            && request.method() != &tiny_http::Method::Delete
        {
            respond(request, 405, String::new());
            continue;
        }
        if workers.fetch_add(1, Ordering::AcqRel) >= MAX_WORKERS {
            workers.fetch_sub(1, Ordering::Release);
            respond(request, 503, String::new());
            continue;
        }
        let client = client.clone();
        let sessions = sessions.clone();
        let workers = workers.clone();
        std::thread::spawn(move || {
            handle_http(request, client, sessions);
            workers.fetch_sub(1, Ordering::Release);
        });
    }
}
fn handle_http(mut request: tiny_http::Request, client: Client, sessions: Sessions) {
    let session_id = header(&request, "Mcp-Session-Id").map(str::to_owned);
    let session = {
        let mut sessions = sessions.lock().unwrap();
        sessions.retain(|_, (session, used)| {
            let keep = used.elapsed() < Duration::from_secs(1800);
            if !keep {
                session.close();
            }
            keep
        });
        session_id
            .as_ref()
            .and_then(|id| sessions.get_mut(id))
            .map(|(session, used)| {
                *used = Instant::now();
                session.clone()
            })
    };
    if session_id.is_some() && session.is_none() {
        respond(request, 404, String::new());
        return;
    }
    if request.method() == &tiny_http::Method::Delete {
        if let Some(id) = session_id {
            if let Some((session, _)) = sessions.lock().unwrap().remove(&id) {
                session.close();
            }
            respond(request, 200, String::new());
        } else {
            respond(request, 400, String::new());
        }
        return;
    }
    if request.method() == &tiny_http::Method::Get {
        let Some(session) = session else {
            respond(request, 400, String::new());
            return;
        };
        if !header(&request, "Accept").is_some_and(|a| a.contains("text/event-stream")) {
            respond(request, 406, String::new());
            return;
        }
        if session.stream_open.swap(true, Ordering::AcqRel) {
            respond(request, 409, String::new());
            return;
        }
        let _ = stream_logs(request, client, session.clone());
        session.stream_open.store(false, Ordering::Release);
        return;
    }
    if !header(&request, "Content-Type")
        .is_some_and(|s| s.split(';').next() == Some("application/json"))
    {
        respond(request, 415, String::new());
        return;
    }
    if !header(&request, "Accept")
        .is_some_and(|s| s.contains("application/json") && s.contains("text/event-stream"))
    {
        respond(request, 406, String::new());
        return;
    }
    if request.body_length().is_some_and(|n| n > MAX_MESSAGE) {
        respond(request, 413, String::new());
        return;
    }
    let mut bytes = Vec::new();
    if request
        .as_reader()
        .take((MAX_MESSAGE + 1) as u64)
        .read_to_end(&mut bytes)
        .is_err()
    {
        respond(request, 400, String::new());
        return;
    }
    if bytes.len() > MAX_MESSAGE {
        respond(request, 413, String::new());
        return;
    }
    match serde_json::from_slice(&bytes) {
        Ok(message) => {
            let message: Value = message;
            if message["method"] == "initialize" && session.is_none() {
                let mut bytes = [0u8; 24];
                if getrandom::fill(&mut bytes).is_err() {
                    respond(request, 500, String::new());
                    return;
                }
                let id = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                let session = Arc::new(Session::default());
                let Some(response) = session.handle(&client, message) else {
                    respond(request, 400, String::new());
                    return;
                };
                if response.get("error").is_some() {
                    respond(request, 400, response.to_string());
                    return;
                }
                {
                    let mut sessions = sessions.lock().unwrap();
                    if sessions.len() >= 64 {
                        respond(request, 503, String::new());
                        return;
                    }
                    sessions.insert(id.clone(), (session, Instant::now()));
                }
                let response = tiny_http::Response::from_string(response.to_string())
                    .with_header(
                        tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap(),
                    )
                    .with_header(tiny_http::Header::from_bytes("Mcp-Session-Id", id).unwrap());
                let _ = request.respond(response);
            } else if let Some(session) = session {
                match session.handle(&client, message) {
                    Some(response) => respond(request, 200, response.to_string()),
                    None => respond(request, 202, String::new()),
                }
            } else {
                respond(request, 400, String::new());
            }
        }
        Err(_) => respond(
            request,
            400,
            protocol::error(Value::Null, -32700, "Parse error").to_string(),
        ),
    }
}
// Flush every event: tiny_http's generic streaming response buffers chunks, which otherwise
// delays small notifications for minutes. We keep HTTP chunk framing explicit here.
fn stream_logs(
    request: tiny_http::Request,
    client: Client,
    session: Arc<Session>,
) -> io::Result<()> {
    let mut writer = request.into_writer();
    writer.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nTransfer-Encoding: chunked\r\n\r\n")?;
    writer.flush()?;
    let mut cursor = 0;
    while !client.stop.load(Ordering::Acquire) && !session.is_closed() {
        let messages = session.logs(&client, &mut cursor);
        let text = if messages.is_empty() {
            ": keepalive\n\n".into()
        } else {
            messages
                .into_iter()
                .map(|m| format!("event: message\ndata: {m}\n\n"))
                .collect::<String>()
        };
        write!(writer, "{:x}\r\n{}\r\n", text.len(), text)?;
        writer.flush()?;
        std::thread::sleep(Duration::from_millis(250));
    }
    writer.write_all(b"0\r\n\r\n")?;
    writer.flush()
}
