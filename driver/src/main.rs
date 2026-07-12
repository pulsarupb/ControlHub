use std::env;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use localizer::ChassisConfig;

mod api;
mod config;
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
        .map(|s| s.to_string())
        .unwrap_or_else(|| "data/config.json".to_string());

    let raw_config = config::load_raw_config(&config_path).unwrap_or_else(|e| {
        eprintln!("Note: {e} — using default config");
        config::RawConfig::default()
    });
    let chassis_config = config::raw_to_chassis_config(&raw_config);

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

    let shared_config: Arc<Mutex<ChassisConfig>> = Arc::new(Mutex::new(chassis_config));
    let restart_flag = Arc::new(AtomicBool::new(false));

    let state = state::AppState::new(
        simulate,
        urdf_json,
        config_path.clone(),
        shared_config.clone(),
    );

    let motor_state = state.clone();
    let motor_restart = restart_flag.clone();
    let motor_shared_config = shared_config.clone();

    std::thread::spawn(move || {
        motor_supervisor(simulate, motor_state, motor_shared_config, motor_restart);
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Rover API listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, api::router(state, restart_flag)).await?;
    Ok(())
}

fn motor_supervisor(
    simulate: bool,
    state: state::AppState,
    shared_config: Arc<Mutex<ChassisConfig>>,
    restart_flag: Arc<AtomicBool>,
) {
    loop {
        restart_flag.store(false, Ordering::SeqCst);

        let state_clone = state.clone();
        let config_clone = shared_config.clone();
        let flag_clone = restart_flag.clone();

        let handle = std::thread::spawn(move || {
            if simulate {
                simulator::run_simulated_motor_loop(state_clone, config_clone, flag_clone);
            } else {
                motor::run_motor_loop(state_clone, config_clone, flag_clone);
            }
        });

        handle.join().ok();

        if restart_flag.load(Ordering::SeqCst) {
            eprintln!("Motor loop restarting...");
        } else {
            eprintln!("Motor loop exited unexpectedly. Restarting in 1s...");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
