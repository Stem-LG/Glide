use anyhow::{Result, bail};
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command, sync::mpsc};

pub const RATE: u32 = 48000;
pub const CHANNELS: u32 = 2;
/// 10 ms of interleaved signed 16-bit little-endian PCM.
pub const FRAME: usize = (RATE as usize / 100) * CHANNELS as usize * 2;

/// Host tools that can record what the default output is playing. PipeWire is
/// tried first; `parec` covers PulseAudio desktops and older PipeWire releases.
fn recorders() -> Vec<Command> {
    let mut pipewire = Command::new("pw-record");
    pipewire.args([
        "-P",
        "{ stream.capture.sink = true node.name = glide-desktop-audio media.name = \"Glide desktop audio\" }",
        "--rate",
        "48000",
        "--channels",
        "2",
        "--format",
        "s16",
        "--latency",
        "10ms",
        "--raw",
        "-",
    ]);
    let mut pulse = Command::new("parec");
    pulse.args([
        "--device=@DEFAULT_MONITOR@",
        "--format=s16le",
        "--rate=48000",
        "--channels=2",
        "--raw",
        "--latency-msec=10",
        "--client-name=Glide",
    ]);
    let mut all = vec![pipewire, pulse];
    for command in &mut all {
        host_libraries(command);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
    }
    all
}

/// The AppImage prepends its bundled libraries; host tools must not load them.
fn host_libraries(command: &mut Command) {
    let (Some(appdir), Some(paths)) = (
        std::env::var_os("APPDIR"),
        std::env::var_os("LD_LIBRARY_PATH"),
    ) else {
        return;
    };
    let host: Vec<_> = std::env::split_paths(&paths)
        .filter(|p| !p.starts_with(&appdir))
        .collect();
    match std::env::join_paths(host) {
        Ok(joined) if !joined.is_empty() => command.env("LD_LIBRARY_PATH", joined),
        _ => command.env_remove("LD_LIBRARY_PATH"),
    };
}

/// Returns false once the listener is gone. A full queue drops the frame so a
/// slow network costs a glitch instead of ever-growing delay.
fn offer(tx: &mpsc::Sender<Vec<u8>>, frame: Vec<u8>) -> bool {
    !matches!(
        tx.try_send(frame),
        Err(mpsc::error::TrySendError::Closed(_))
    )
}

/// Preview mode has no desktop to record, so it plays a quiet test tone.
async fn tone(tx: mpsc::Sender<Vec<u8>>) {
    let mut ticker = tokio::time::interval(Duration::from_millis(10));
    let mut n = 0u64;
    loop {
        ticker.tick().await;
        let mut frame = Vec::with_capacity(FRAME);
        for _ in 0..FRAME / 4 {
            let t = n as f64 / f64::from(RATE);
            let sample = ((t * 440.0 * std::f64::consts::TAU).sin() * 3000.0) as i16;
            frame.extend_from_slice(&sample.to_le_bytes());
            frame.extend_from_slice(&sample.to_le_bytes());
            n += 1;
        }
        if !offer(&tx, frame) {
            return;
        }
    }
}

/// Streams desktop audio into `tx` until the receiver is dropped. Returns an
/// error when no recorder on this system produces audio.
pub async fn run(tx: mpsc::Sender<Vec<u8>>, preview: bool) -> Result<()> {
    if preview {
        tone(tx).await;
        return Ok(());
    }
    for mut command in recorders() {
        let Ok(mut child) = command.spawn() else {
            continue;
        };
        let mut stdout = child.stdout.take().unwrap();
        let mut produced = false;
        loop {
            let mut frame = vec![0u8; FRAME];
            if stdout.read_exact(&mut frame).await.is_err() {
                break;
            }
            produced = true;
            if !offer(&tx, frame) {
                return Ok(());
            }
        }
        if produced {
            bail!("Desktop audio stopped");
        }
    }
    bail!("Desktop audio needs PipeWire (pw-record) or PulseAudio (parec) on the desktop")
}

/// Exposes a virtual PipeWire microphone that plays the mono signed 16-bit PCM
/// written to its stdin. It disappears when the child is dropped.
pub fn microphone(rate: u32) -> Result<tokio::process::Child> {
    let mut command = Command::new("pw-cat");
    command.args([
        "--playback",
        "--raw",
        "--rate",
        &rate.to_string(),
        "--channels",
        "1",
        "--format",
        "s16",
        "--latency",
        "20ms",
        "-P",
        "{ media.class = Audio/Source node.name = glide-microphone node.description = \"Glide phone microphone\" }",
        "-",
    ]);
    host_libraries(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| anyhow::anyhow!("The phone microphone needs PipeWire (pw-cat) on the desktop"))
}
