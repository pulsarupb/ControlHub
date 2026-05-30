use std::env;
use std::net::SocketAddr;

mod api;
mod follower;
mod localizer;
mod motor;
mod state;
mod wifi;

use motor::{read_motor_continuously, run_motor_loop};
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if matches!(args.get(1).map(String::as_str), Some("read" | "calibrate")) {
        let Some(motor_id) = args.get(2).and_then(|value| value.parse::<u8>().ok()) else {
            println!("Usage: cargo run -p driver -- read <motor_id>");
            return Ok(());
        };

        read_motor_continuously(motor_id)?;
        return Ok(());
    }

    let (ssid, password) = wifi::ensure_hotspot();
    println!("WiFi network: {ssid}");
    println!("Password: {password}");

    let state = AppState::new();

    let motor_state = state.clone();
    std::thread::spawn(move || run_motor_loop(motor_state));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Rover API listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, api::router(state)).await?;
    Ok(())
}
