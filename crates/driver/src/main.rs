mod api;
mod arm;
mod config;
mod d435;
mod motor;
mod oak;
mod sensors;
mod state;
mod wifi;

use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut values = args.iter().skip(1);
    while let Some(arg) = values.next() {
        match arg.as_str() {
            "--simulate" | "--hotspot" | "--help" => {}
            "--config" | "--bind" | "--sidecar" | "--oak-sidecar" => {
                if values.next().is_none_or(|value| value.starts_with("--")) {
                    return Err(format!("missing value for {arg}").into());
                }
            }
            _ => return Err(format!("unknown argument: {arg} (see --help)").into()),
        }
    }
    let argument = |name: &str, default: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| default.into())
    };
    if args.iter().any(|a| a == "--help") {
        println!(
            "driver [--simulate] [--config data/config.json] [--bind 0.0.0.0:8080] [--sidecar crates/zed/sidecar/main.py] [--oak-sidecar crates/oakd/sidecar/main.py] [--hotspot]"
        );
        return Ok(());
    }
    let simulate = args.iter().any(|a| a == "--simulate");
    let config_path = argument("--config", "data/config.json");
    let configuration = config::load(&config_path)?;
    let script = std::path::PathBuf::from(argument("--sidecar", "crates/zed/sidecar/main.py"));
    let (state, arm_rx) = state::AppState::new(configuration, config_path, simulate);
    let address = argument("--bind", "0.0.0.0:8080");
    let listener = tokio::net::TcpListener::bind(&address).await?;
    if simulate {
        println!("SIMULATION: no hardware, SDK, NTRIP or robot connections will be opened");
    }
    if !simulate && args.iter().any(|a| a == "--hotspot") {
        wifi::ensure_hotspot();
    }
    for worker in [motor::run, sensors::rtk_worker] {
        let s = state.clone();
        std::thread::spawn(move || worker(s));
    }
    let s = state.clone();
    std::thread::spawn(move || sensors::zed_worker(s, script));
    let s = state.clone();
    let oak_script =
        std::path::PathBuf::from(argument("--oak-sidecar", "crates/oakd/sidecar/main.py"));
    std::thread::spawn(move || oak::run(s, oak_script));
    tokio::spawn(arm::run(state.clone(), arm_rx));
    if !simulate {
        let camera = state.d435.clone();
        let shutdown = state.shutting_down.clone();
        std::thread::spawn(move || camera.run(&shutdown));
    }
    println!(
        "Rover API listening on http://{address}; motion starts stopped, explicit enable required"
    );
    axum::serve(listener, api::router(state.clone()))
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            state
                .shutting_down
                .store(true, std::sync::atomic::Ordering::SeqCst);
            state.stop_all();
            tokio::time::sleep(Duration::from_secs(2)).await;
        })
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
