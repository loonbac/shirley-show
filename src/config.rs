use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

pub const DEFAULT_GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";
pub const DEFAULT_GROQ_MODEL: &str = "openai/gpt-oss-120b";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroqConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
}

impl Default for GroqConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_GROQ_BASE_URL.to_string(),
            model: DEFAULT_GROQ_MODEL.to_string(),
        }
    }
}

impl GroqConfig {
    pub fn normalize(&mut self) {
        self.api_key = self.api_key.take().and_then(|k| {
            let trimmed = k.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        let mut trimmed_url = self.base_url.trim();
        while trimmed_url.ends_with('/') {
            trimmed_url = &trimmed_url[..trimmed_url.len() - 1];
        }
        if trimmed_url.is_empty() {
            self.base_url = DEFAULT_GROQ_BASE_URL.to_string();
        } else {
            self.base_url = trimmed_url.to_string();
        }

        let trimmed_model = self.model.trim();
        if trimmed_model.is_empty() {
            self.model = DEFAULT_GROQ_MODEL.to_string();
        } else {
            self.model = trimmed_model.to_string();
        }
    }

    pub fn from_env() -> Self {
        let api_key = env::var("GROQ_API_KEY").ok();
        let base_url =
            env::var("GROQ_BASE_URL").unwrap_or_else(|_| DEFAULT_GROQ_BASE_URL.to_string());
        let model = env::var("GROQ_MODEL").unwrap_or_else(|_| DEFAULT_GROQ_MODEL.to_string());

        let mut config = Self {
            api_key,
            base_url,
            model,
        };
        config.normalize();
        config
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub data_dir: PathBuf,
    pub frontend_dir: PathBuf,
    pub groq: GroqConfig,
    pub yt_dlp_bin: Option<PathBuf>,
    pub agent_timeout_secs: u64,
    pub separation_timeout_secs: u64,
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

        let groq = GroqConfig::from_env();

        let yt_dlp_bin = env::var("YT_DLP_BIN").ok().and_then(|val| {
            let trimmed = val.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        });

        let agent_timeout_secs = match env::var("SHIRLEY_AGENT_TIMEOUT_SECS") {
            Ok(val) => {
                let trimmed = val.trim();
                if trimmed.is_empty() {
                    120
                } else {
                    let parsed: u64 = trimmed.parse().map_err(|e| {
                        format!("Invalid SHIRLEY_AGENT_TIMEOUT_SECS '{trimmed}': {e}")
                    })?;
                    if parsed == 0 {
                        return Err(
                            "SHIRLEY_AGENT_TIMEOUT_SECS must be strictly positive".to_string()
                        );
                    }
                    parsed
                }
            }
            Err(_) => 120,
        };

        let separation_timeout_secs = match env::var("SHIRLEY_SEPARATION_TIMEOUT_SECS") {
            Ok(val) => {
                let trimmed = val.trim();
                if trimmed.is_empty() {
                    600
                } else {
                    let parsed: u64 = trimmed.parse().map_err(|e| {
                        format!("Invalid SHIRLEY_SEPARATION_TIMEOUT_SECS '{trimmed}': {e}")
                    })?;
                    if parsed == 0 {
                        return Err(
                            "SHIRLEY_SEPARATION_TIMEOUT_SECS must be strictly positive".to_string()
                        );
                    }
                    parsed
                }
            }
            Err(_) => 600,
        };

        Ok(Self {
            bind_addr,
            data_dir,
            frontend_dir,
            groq,
            yt_dlp_bin,
            agent_timeout_secs,
            separation_timeout_secs,
        })
    }
}
