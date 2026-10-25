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
    pub address: String,
    pub attempts: HashMap<IpAddr, (Instant, u8)>,
    pub helper: Option<mpsc::Sender<Frame>>,
}
pub type AppState = Arc<Mutex<Shared>>;
impl Shared {
    pub fn new(config: Config, port: u16) -> Self {
        Self {
            config,
            pairing_code: format!("{:06}", rand::random_range(0..1000000)),
            pairing: true,
            enabled: true,
            connected: None,
            status: "Starting…".into(),
            address: format!("http://{}:{port}", lan_ip()),
            attempts: HashMap::new(),
            helper: None,
        }
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
        .is_some_and(|o| o == format!("http://{host}"))
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
    loop {
        tokio::select! {
        _=ticker.tick()=>{
          let settings = {
            let s=state.lock().unwrap();
            if !s.enabled||last.elapsed()>Duration::from_secs(2){None}
            else{s.config.devices.iter().find(|d|d.id==owner.id && d.hash==owner.hash).map(|d|d.settings.clone())}
          };
          let Some(settings)=settings else{break};
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
            s.helper.clone()
         };
         let seq=frame.seq;let n=frame.points.len();
         if tx.is_none_or(|tx|tx.try_send(frame).is_err()){break;}
         if ws.send(Message::Text(json!({"ack":seq,"contacts":n}).to_string().into())).await.is_err(){break;}
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
        .route(
            "/health",
            get(|| async { Json(json!({"app":"Glide","version":env!("CARGO_PKG_VERSION")})) }),
        )
        .layer(axum::extract::DefaultBodyLimit::max(8192))
        .with_state(state.clone());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
