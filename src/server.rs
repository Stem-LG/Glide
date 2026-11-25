use crate::{
    config::{self, Config, Device, DeviceSettings},
    input::Frame,
};
use anyhow::Result;
use axum::{
    Json, Router,
    extract::{
        ConnectInfo, State,
        ws::{Message, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc,
};

pub struct Shared {
    pub config: Config,
    pub pairing_code: String,
    pub pairing: bool,
    pub enabled: bool,
    pub connected: Option<(String, String)>,
    pub status: String,
    pub input_feedback: Option<serde_json::Value>,
    pub address: String,
    pub attempts: HashMap<IpAddr, (Instant, u8)>,
    pub helper: Option<mpsc::Sender<Frame>>,
    pub audio_listener: bool,
    pub microphone: bool,
    pub preview: bool,
    pub https: bool,
    pub port: u16,
}
pub type AppState = Arc<Mutex<Shared>>;
impl Shared {
    /// Switches the advertised address; the listener follows on the next connection.
    pub fn set_https(&mut self, https: bool) {
        self.https = https;
        let scheme = if https { "https" } else { "http" };
        self.address = format!("{scheme}://{}:{}", lan_ip(), self.port);
    }
    pub fn new(config: Config, port: u16) -> Self {
        let https = config.https;
        let mut shared = Self {
            config,
            pairing_code: format!("{:06}", rand::random_range(0..1000000)),
            pairing: true,
            enabled: true,
            connected: None,
            status: "Starting…".into(),
            input_feedback: None,
            address: String::new(),
            attempts: HashMap::new(),
            helper: None,
            audio_listener: false,
            microphone: false,
            preview: false,
            https: false,
            port,
        };
        shared.set_https(https);
        shared
    }
}
pub fn lan_ip() -> IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0");
    socket
        .and_then(|s| {
            s.connect("192.0.2.1:80")?;
            s.local_addr()
        })
        .map(|a| a.ip())
        .unwrap_or("127.0.0.1".parse().unwrap())
}
fn same_origin(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get("host").and_then(|s| s.to_str().ok()) else {
        return false;
    };
    headers
        .get("origin")
        .and_then(|s| s.to_str().ok())
        .is_some_and(|o| o == format!("http://{host}") || o == format!("https://{host}"))
}
#[derive(Deserialize)]
struct Pair {
    code: String,
    name: String,
    profile_id: Option<String>,
}
async fn pair(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<Pair>,
) -> Response {
    if !same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut s = state.lock().unwrap();
    s.attempts
        .retain(|_, (time, _)| time.elapsed() < Duration::from_secs(60));
    if s.attempts.len() >= 1024 && !s.attempts.contains_key(&peer.ip()) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let item = s.attempts.entry(peer.ip()).or_insert((Instant::now(), 0));
    if item.1 >= 5 {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many attempts. Wait one minute.",
        )
            .into_response();
    }
    item.1 += 1;
    if !s.pairing || req.code != s.pairing_code {
        return (StatusCode::UNAUTHORIZED, "Check the pairing code in Glide.").into_response();
    }
    let name = req
        .name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(48)
        .collect::<String>();
    // Reveal saved profiles only after proof of the current desktop pairing PIN.
    if req.profile_id.is_none() && !s.config.devices.is_empty() {
        return Json(json!({"profiles":s.config.devices.iter().map(|d| json!({"id":d.id,"name":d.name})).collect::<Vec<_>>() })).into_response();
    }
    if name.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let token = config::token();
    let mut next = s.config.clone();
    let id = if let Some(profile) = req.profile_id.filter(|id| !id.is_empty()) {
        let Some(device) = next.devices.iter_mut().find(|d| d.id == profile) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        // Re-pairing rotates the credential, preserving the stable profile/settings.
        device.hash = config::digest(&token);
        device.id.clone()
    } else {
        if next.devices.len() >= 20 {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let id = config::token()[..16].to_string();
        next.devices.push(Device {
            id: id.clone(),
            name,
            hash: config::digest(&token),
            settings: DeviceSettings::default(),
        });
        id
    };
    if config::save(&next).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Cannot save pairing").into_response();
    }
    s.config = next;
    s.pairing = false;
    s.pairing_code = format!("{:06}", rand::random_range(0..1000000));
    Json(json!({"token":token,"id":id})).into_response()
}
#[derive(Deserialize)]
struct Hello {
    token: String,
    rate: Option<u32>,
}
async fn ws(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    upgrade
        .max_message_size(8192)
        .max_frame_size(8192)
        .on_upgrade(move |ws| connection(ws, state))
        .into_response()
}
async fn connection(mut ws: axum::extract::ws::WebSocket, state: AppState) {
    let hello = tokio::time::timeout(Duration::from_secs(5), ws.recv()).await;
    let Ok(Some(Ok(Message::Text(text)))) = hello else {
        return;
    };
    let Ok(hello) = serde_json::from_str::<Hello>(&text) else {
        return;
    };
    let owner = {
        let mut s = state.lock().unwrap();
        let d = s
            .config
            .devices
            .iter()
            .find(|d| d.hash == config::digest(&hello.token))
            .cloned();
        if s.connected.is_some() || !s.enabled || s.helper.is_none() {
            None
        } else if let Some(d) = d {
            s.connected = Some((d.id.clone(), d.name.clone()));
            s.input_feedback = None;
            Some(d)
        } else {
            None
        }
    };
    let Some(owner) = owner else {
        let _ = ws
            .send(Message::Text(
                json!({"error":"Connection unavailable. Check Glide or pair again."})
                    .to_string()
                    .into(),
            ))
            .await;
        return;
    };
    let _ = ws
        .send(Message::Text(
            json!({"ready":true,"name":owner.name,"device_id":owner.id,"settings":owner.settings})
                .to_string()
                .into(),
        ))
        .await;
    let mut last_settings = owner.settings.clone();
    let mut ticker = tokio::time::interval(Duration::from_millis(250));
    let mut last = Instant::now();
    let mut last_media: Option<Instant> = None;
    let mut last_feedback = None;
    loop {
        tokio::select! {
        _=ticker.tick()=>{
          let settings = {
            let s=state.lock().unwrap();
            if !s.enabled||last.elapsed()>Duration::from_secs(2){None}
            else{s.config.devices.iter().find(|d|d.id==owner.id && d.hash==owner.hash).map(|d|d.settings.clone())}
          };
          let Some(settings)=settings else{break};
          let feedback=state.lock().unwrap().input_feedback.clone();
          if feedback!=last_feedback {
            last_feedback=feedback.clone();
            if let Some(reply)=feedback
              && ws.send(Message::Text(reply.to_string().into())).await.is_err(){break;}
          }
          if settings!=last_settings {
            last_settings=settings.clone();
            if ws.send(Message::Text(json!({"settings":settings}).to_string().into())).await.is_err(){break;}
          }
        },
        msg=ws.recv()=>{
         let Some(Ok(Message::Text(text)))=msg else{break};
         let Ok(value)=serde_json::from_str::<serde_json::Value>(&text) else{break};
         if let Some(settings)=value.get("settings") {
            let result=serde_json::from_value::<DeviceSettings>(settings.clone()).map_err(anyhow::Error::from).and_then(|settings| {
                let mut s=state.lock().unwrap();
                anyhow::ensure!(s.config.devices.iter().any(|d|d.id==owner.id && d.hash==owner.hash),"Device no longer paired");
                save_settings(&mut s,&owner.id,settings.clone())?; Ok(settings)
            });
            let reply=match result {
                Ok(settings)=>{last_settings=settings.clone();json!({"settings":settings,"settings_saved":true})},
                Err(_)=>{let settings=state.lock().unwrap().config.devices.iter().find(|d|d.id==owner.id).map(|d|d.settings.clone());json!({"settings_error":"Could not save settings on the desktop.","settings":settings})},
            };
            if ws.send(Message::Text(reply.to_string().into())).await.is_err(){break;}
            continue;
         }
         let Ok(mut frame)=serde_json::from_value::<Frame>(value) else{break};
         if frame.validate().is_err(){break;}last=Instant::now();
         let tx={
            let s=state.lock().unwrap();
            let Some(device)=s.config.devices.iter().find(|d|d.id==owner.id && d.hash==owner.hash) else{break};
            frame.cursor_speed=device.settings.cursor_speed;
            frame.scroll_speed=device.settings.scroll_speed;
            if frame.media.is_some() && (!device.settings.media_controls || last_media.is_some_and(|t|t.elapsed()<Duration::from_millis(75))) {
                frame.media=None;
            }
            if frame.media.is_some() { last_media=Some(Instant::now()); }
            s.helper.clone()
         };
         let seq=frame.seq;let n=frame.points.len();let media=frame.media;let keypad=frame.keypad;
         if tx.is_none_or(|tx|tx.try_send(frame).is_err()){break;}
         if ws.send(Message::Text(json!({"ack":seq,"contacts":n,"media":media,"keypad":keypad}).to_string().into())).await.is_err(){break;}
        }
        }
    }
    let tx = state.lock().unwrap().helper.clone();
    if let Some(tx) = tx {
        let _ = tokio::time::timeout(Duration::from_secs(1), tx.send(Frame::default())).await;
    }
    state.lock().unwrap().connected = None;
    let _ = ws.send(Message::Close(None)).await;
}

async fn audio_ws(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    upgrade
        .max_message_size(1024)
        .max_frame_size(1024)
        .on_upgrade(move |ws| audio_connection(ws, state))
        .into_response()
}
/// Desktop audio rides its own socket so a congested stream can never delay
/// touch input. It is only served to the device that currently owns input.
async fn audio_connection(mut ws: axum::extract::ws::WebSocket, state: AppState) {
    let hello = tokio::time::timeout(Duration::from_secs(5), ws.recv()).await;
    let Ok(Some(Ok(Message::Text(text)))) = hello else {
        return;
    };
    let Ok(hello) = serde_json::from_str::<Hello>(&text) else {
        return;
    };
    let hash = config::digest(&hello.token);
    let allowed = |s: &Shared| {
        s.enabled
            && s.config.devices.iter().any(|d| {
                d.hash == hash
                    && d.settings.audio
                    && s.connected.as_ref().is_some_and(|(id, _)| id == &d.id)
            })
    };
    let preview = {
        let mut s = state.lock().unwrap();
        if s.audio_listener || !allowed(&s) {
            None
        } else {
            s.audio_listener = true;
            Some(s.preview)
        }
    };
    let Some(preview) = preview else {
        let _ = ws
            .send(Message::Text(
                json!({"error":"Desktop audio is unavailable for this device."})
                    .to_string()
                    .into(),
            ))
            .await;
        return;
    };
    // About 80 ms of queue; the recorder drops frames beyond that.
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(8);
    let mut recorder = tokio::spawn(crate::audio::run(tx, preview));
    let mut ticker = tokio::time::interval(Duration::from_millis(250));
    let format = json!({"audio":{"rate":crate::audio::RATE,"channels":crate::audio::CHANNELS}});
    if ws
        .send(Message::Text(format.to_string().into()))
        .await
        .is_ok()
    {
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if !allowed(&state.lock().unwrap()) { break; }
                },
                frame = rx.recv() => {
                    let Some(frame) = frame else {
                        let reason = match (&mut recorder).await {
                            Ok(Err(e)) => e.to_string(),
                            _ => "Desktop audio stopped".into(),
                        };
                        let _ = ws.send(Message::Text(json!({"error":reason}).to_string().into())).await;
                        break;
                    };
                    if ws.send(Message::Binary(frame.into())).await.is_err() { break; }
                },
                msg = ws.recv() => {
                    if !matches!(msg, Some(Ok(Message::Ping(_) | Message::Pong(_)))) { break; }
                }
            }
        }
    }
    recorder.abort();
    state.lock().unwrap().audio_listener = false;
    let _ = ws.send(Message::Close(None)).await;
}

async fn mic_ws(
    State(state): State<AppState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if !same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    upgrade
        .max_message_size(8192)
        .max_frame_size(8192)
        .on_upgrade(move |ws| mic_connection(ws, state))
        .into_response()
}
/// Phone microphone: mono PCM frames from the input owner feed a virtual
/// desktop microphone that exists only while this socket is open.
async fn mic_connection(mut ws: axum::extract::ws::WebSocket, state: AppState) {
    let hello = tokio::time::timeout(Duration::from_secs(5), ws.recv()).await;
    let Ok(Some(Ok(Message::Text(text)))) = hello else {
        return;
    };
    let Ok(hello) = serde_json::from_str::<Hello>(&text) else {
        return;
    };
    let hash = config::digest(&hello.token);
    let rate = hello.rate.unwrap_or(crate::audio::RATE);
    let allowed = |s: &Shared| {
        s.enabled
            && s.config.devices.iter().any(|d| {
                d.hash == hash
                    && d.settings.microphone
                    && s.connected.as_ref().is_some_and(|(id, _)| id == &d.id)
            })
    };
    let preview = {
        let mut s = state.lock().unwrap();
        if s.microphone || !allowed(&s) || !(8000..=96000).contains(&rate) {
            None
        } else {
            s.microphone = true;
            Some(s.preview)
        }
    };
    let Some(preview) = preview else {
        let _ = ws
            .send(Message::Text(
                json!({"error":"The microphone is unavailable for this device."})
                    .to_string()
                    .into(),
            ))
            .await;
        return;
    };
    // Preview mode accepts and discards frames without touching the desktop.
    let child = if preview {
        Ok(None)
    } else {
        crate::audio::microphone(rate).map(Some)
    };
    match child {
        Err(e) => {
            let _ = ws
                .send(Message::Text(
                    json!({"error":e.to_string()}).to_string().into(),
                ))
                .await;
        }
        Ok(mut child) => {
            let mut stdin = child.as_mut().and_then(|c| c.stdin.take());
            let mut ticker = tokio::time::interval(Duration::from_millis(250));
            let started = json!({"mic":{"rate":rate}}).to_string();
            if ws.send(Message::Text(started.into())).await.is_ok() {
                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            if !allowed(&state.lock().unwrap()) { break; }
                        },
                        msg = ws.recv() => match msg {
                            Some(Ok(Message::Binary(frame))) if frame.len() % 2 == 0 => {
                                if let Some(stdin) = &mut stdin
                                    && stdin.write_all(&frame).await.is_err() { break; }
                            }
                            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                            _ => break,
                        }
                    }
                }
            }
        }
    }
    state.lock().unwrap().microphone = false;
    let _ = ws.send(Message::Close(None)).await;
}

pub fn save_settings(s: &mut Shared, id: &str, settings: DeviceSettings) -> Result<()> {
    settings.validate()?;
    let mut next = s.config.clone();
    let device = next
        .devices
        .iter_mut()
        .find(|d| d.id == id)
        .ok_or_else(|| anyhow::anyhow!("Device no longer paired"))?;
    device.settings = settings;
    config::save(&next)?;
    s.config = next;
    Ok(())
}

async fn input_task(state: AppState, preview: bool) -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<Frame>(16);
    if preview {
        state.lock().unwrap().helper = Some(tx);
        while rx.recv().await.is_some() {}
        return Ok(());
    }
    let mut child = Command::new(std::env::current_exe()?)
        .arg("--input-helper")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(5), stdout.read_line(&mut line)).await??;
    anyhow::ensure!(
        line.starts_with("READY"),
        "Input access unavailable. See Setup in Glide."
    );
    eprintln!("{}", line.trim());
    state.lock().unwrap().helper = Some(tx);
    let feedback_state = state.clone();
    tokio::spawn(async move {
        loop {
            let mut line = String::new();
            match stdout.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line)
                        && value.get("num_lock_state").is_some()
                    {
                        feedback_state.lock().unwrap().input_feedback = Some(value);
                    }
                }
            }
        }
    });
    let mut stdin = child.stdin.take().unwrap();
    while let Some(frame) = rx.recv().await {
        stdin
            .write_all(serde_json::to_string(&frame)?.as_bytes())
            .await?;
        stdin.write_all(b"\n").await?;
    }
    Ok(())
}
pub async fn run(state: AppState, port: u16, preview: bool) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    state.lock().unwrap().preview = preview;
    state.lock().unwrap().status = if preview {
        "Preview mode — no desktop input"
    } else {
        "Ready to connect"
    }
    .into();

    let state_input = state.clone();
    tokio::spawn(async move {
        if let Err(e) = input_task(state_input.clone(), preview).await {
            let mut s = state_input.lock().unwrap();
            s.helper = None;
            s.status = format!("Input unavailable: {e}");
        }
    });
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                (
                    [("cache-control", "no-store")],
                    Html(include_str!("../web/index.html")),
                )
            }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/app.js"),
                )
            }),
        )
        .route(
            "/taps.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/taps.js"),
                )
            }),
        )
        .route(
            "/media.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/media.js"),
                )
            }),
        )
        .route(
            "/tools.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/tools.js"),
                )
            }),
        )
        .route(
            "/tools-ui.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/tools-ui.js"),
                )
            }),
        )
        .route(
            "/audio.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/audio.js"),
                )
            }),
        )
        .route(
            "/audio-ui.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/audio-ui.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [("content-type", "text/css"), ("cache-control", "no-store")],
                    include_str!("../web/style.css"),
                )
            }),
        )
        .route("/api/pair", post(pair))
        .route("/ws", get(ws))
        .route("/audio", get(audio_ws))
        .route("/mic", get(mic_ws))
        .route(
            "/mic-worklet.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/mic-worklet.js"),
                )
            }),
        )
        .route(
            "/mic-ui.js",
            get(|| async {
                (
                    [
                        ("content-type", "text/javascript"),
                        ("cache-control", "no-store"),
                    ],
                    include_str!("../web/mic-ui.js"),
                )
            }),
        )
        .route(
            "/health",
            get(|| async { Json(json!({"app":"Glide","version":env!("CARGO_PKG_VERSION")})) }),
        )
        .layer(axum::extract::DefaultBodyLimit::max(8192))
        .with_state(state.clone());
    // Phones that still open the plain address are sent to the secure one.
    let redirect = Router::new().fallback(|headers: HeaderMap, uri: axum::http::Uri| async move {
        match headers.get("host").and_then(|h| h.to_str().ok()) {
            Some(host) => {
                axum::response::Redirect::temporary(&format!("https://{host}{uri}")).into_response()
            }
            None => StatusCode::BAD_REQUEST.into_response(),
        }
    });
    let mut acceptor = None;
    loop {
        let Ok((stream, peer)) = listener.accept().await else {
            tokio::time::sleep(Duration::from_millis(50)).await;
            continue;
        };
        // Touch and audio frames are small and time-critical; never batch them.
        let _ = stream.set_nodelay(true);
        let https = state.lock().unwrap().https;
        if https && acceptor.is_none() {
            match crate::tls::acceptor(lan_ip()) {
                Ok(tls) => acceptor = Some(tls),
                Err(e) => {
                    let mut s = state.lock().unwrap();
                    s.status = format!("Secure connection unavailable: {e}");
                    s.set_https(false);
                }
            }
        }
        let tls = acceptor.clone().filter(|_| https);
        let (app, redirect) = (app.clone(), redirect.clone());
        tokio::spawn(async move {
            // One port serves both: a TLS handshake always starts with 0x16.
            let mut first = [0u8; 1];
            let wait = Duration::from_secs(10);
            let Ok(Ok(1)) = tokio::time::timeout(wait, stream.peek(&mut first)).await else {
                return;
            };
            match (first[0] == 0x16, tls) {
                (true, Some(tls)) => {
                    if let Ok(Ok(stream)) = tokio::time::timeout(wait, tls.accept(stream)).await {
                        serve(stream, peer, app).await;
                    }
                }
                (true, None) => {}
                (false, Some(_)) => serve(stream, peer, redirect).await,
                (false, None) => serve(stream, peer, app).await,
            }
        });
    }
}
async fn serve<S>(stream: S, peer: SocketAddr, app: Router)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let service = hyper::service::service_fn(move |mut request: hyper::Request<_>| {
        request.extensions_mut().insert(ConnectInfo(peer));
        tower::ServiceExt::oneshot(app.clone(), request)
    });
    let _ = hyper::server::conn::http1::Builder::new()
        .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
        .with_upgrades()
        .await;
}
