use moteus::transport::args::TransportArgs;
use moteus::transport::singleton::get_singleton_transport;
use moteus::{Transport, TransportOptions};

fn main() -> Result<(), moteus::Error> {
    let args = TransportOptions::new()
        .transport_args(TransportArgs::new().with_can_interface("can0".to_string()));

    let transport = get_singleton_transport(Some(&args))?;
    let mut transport = transport.lock().unwrap();

    let devices = transport.discover(0, 0)?;

    if devices.is_empty() {
        println!("No devices found!");
        return Ok(());
    }

    for device in &devices {
        println!("{}", device);
    }

    Ok(())
}
