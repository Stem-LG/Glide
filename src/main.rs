mod config;
mod input;
mod qr;
mod server;
mod tray;
mod ui;
use std::sync::{Arc, Mutex};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--input-helper") {
        return input::helper();
    }
    if args.iter().any(|a| a == "--version") {
        println!("Glide {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let port = args
        .windows(2)
        .find(|w| w[0] == "--port")
        .map(|w| w[1].parse())
        .transpose()?
        .unwrap_or(8766);
    let preview = args.iter().any(|a| a == "--preview");
    let state = Arc::new(Mutex::new(server::Shared::new(config::load()?, port)));
    if args.iter().any(|a| a == "--headless") {
        {
            let s = state.lock().unwrap();
            println!("Glide: {}\nPairing code: {}", s.address, s.pairing_code);
        }
        return tokio::runtime::Runtime::new()?.block_on(server::run(state, port, preview));
    }
    ui::run(
        state,
        port,
        preview,
        args.iter().any(|a| a == "--background"),
    );
    Ok(())
}
