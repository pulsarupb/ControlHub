#[derive(Debug, Clone)]
pub struct Config {
    pub robot_ip: String,
    pub robot_port: u16,
    pub username: String,
    pub password: String,
    pub session_inactivity_timeout_ms: u32,
    pub connection_inactivity_timeout_ms: u32,
    /// Base delay for reconnect backoff after a failed connection attempt.
    pub reconnect_base_ms: u64,
    /// Upper bound for reconnect backoff.
    pub reconnect_max_ms: u64,
    /// Timeout applied to a single TCP connect attempt.
    pub connect_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            robot_ip: "192.168.1.10".to_string(),
            robot_port: 10000,
            username: "admin".to_string(),
            password: "admin".to_string(),
            session_inactivity_timeout_ms: 60_000,
            connection_inactivity_timeout_ms: 2_000,
            reconnect_base_ms: 1_000,
            reconnect_max_ms: 30_000,
            connect_timeout_ms: 5_000,
        }
    }
}
