use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub data_dir: PathBuf,
    pub frontend_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let bind_str = env::var("SHIRLEY_BIND").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
        let bind_addr: SocketAddr = bind_str
            .parse()
            .map_err(|e| format!("Invalid SHIRLEY_BIND address '{bind_str}': {e}"))?;

        let data_dir = env::var("SHIRLEY_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data"));

        let frontend_dir = env::var("SHIRLEY_FRONTEND_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("frontend"));

        Ok(Self {
            bind_addr,
            data_dir,
            frontend_dir,
        })
    }
}
