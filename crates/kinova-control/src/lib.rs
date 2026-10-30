#[allow(dead_code, clippy::all)]
pub mod api {
    include!(concat!(env!("OUT_DIR"), "/kinova.api.rs"));
    pub mod base {
        include!(concat!(env!("OUT_DIR"), "/kinova.api.base.rs"));
    }
    pub mod common {
        include!(concat!(env!("OUT_DIR"), "/kinova.api.common.rs"));
    }
    pub mod session {
        include!(concat!(env!("OUT_DIR"), "/kinova.api.session.rs"));
    }
    pub mod product_configuration {
        include!(concat!(
            env!("OUT_DIR"),
            "/kinova.api.product_configuration.rs"
        ));
    }
}
pub mod config;
pub mod robot;
pub use config::Config;
pub use robot::RobotClient;
