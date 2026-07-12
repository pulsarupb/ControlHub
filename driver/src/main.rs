use std::env;
use std::net::SocketAddr;

mod api;
mod config;
mod follower;
mod localizer;
mod motor;
mod simulator;
mod state;
mod urdf;
mod wifi;

use motor::read_motor_continuously;

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

    let simulate = args.iter().any(|a| a == "--simulate");
    let config_path = args
        .iter()
        .position(|a| a == "--config")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("data/config.toml");

    let chassis_config = config::load_chassis_config(config_path)
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });

    let urdf_path = "data/rover.urdf";
    let urdf_xml = std::fs::read_to_string(urdf_path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to read URDF '{urdf_path}': {e}");
            std::process::exit(1);
        });
    let urdf_json = urdf::parse_urdf_to_json(&urdf_xml)
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse URDF: {e}");
            std::process::exit(1);
        });

    if !simulate {
        let (ssid, password) = wifi::ensure_hotspot();
        println!("WiFi network: {ssid}");
        println!("Password: {password}");
    } else {
        println!("DEV MODE — using simulated rover (no WiFi)");
    }

    let state = state::AppState::new(simulate, urdf_json);

    let motor_state = state.clone();
    if simulate {
        std::thread::spawn(move || simulator::run_simulated_motor_loop(motor_state, chassis_config));
    } else {
        std::thread::spawn(move || motor::run_motor_loop(motor_state, chassis_config));
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Rover API listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, api::router(state)).await?;
    Ok(())
}
