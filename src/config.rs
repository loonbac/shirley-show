use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";
pub const DEFAULT_GROQ_MODEL: &str = "openai/gpt-oss-120b";
pub const DEFAULT_COMMAND_CODE_BASE_URL: &str = "https://api.commandcode.ai/provider/v1";
pub const DEFAULT_COMMAND_CODE_MODEL: &str = "stealth/space-bunny-alpha";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LlmProviderKind {
    #[default]
    CommandCode,
    Groq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoshuaPcConfig {
    pub host: String,
    pub user: String,
    pub key_path: PathBuf,
    pub port: u16,
    pub wake_timeout: Duration,
}

#[derive(Clone, PartialEq, Eq)]
pub struct GroqConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
    pub provider: LlmProviderKind,
}

impl std::fmt::Debug for GroqConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroqConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("provider", &self.provider)
            .finish()
    }
}

impl Default for GroqConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_COMMAND_CODE_BASE_URL.to_string(),
            model: DEFAULT_COMMAND_CODE_MODEL.to_string(),
            provider: LlmProviderKind::CommandCode,
        }
    }
}

impl GroqConfig {
    pub fn provider_name(&self) -> &'static str {
        match self.provider {
            LlmProviderKind::CommandCode => "Command Code",
            LlmProviderKind::Groq => "Groq",
        }
    }

    pub fn is_command_code(&self) -> bool {
        self.provider == LlmProviderKind::CommandCode
    }

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
            self.base_url = match self.provider {
                LlmProviderKind::CommandCode => DEFAULT_COMMAND_CODE_BASE_URL.to_string(),
                LlmProviderKind::Groq => DEFAULT_GROQ_BASE_URL.to_string(),
            };
        } else {
            self.base_url = trimmed_url.to_string();
        }

        let trimmed_model = self.model.trim();
        if trimmed_model.is_empty() {
            self.model = match self.provider {
                LlmProviderKind::CommandCode => DEFAULT_COMMAND_CODE_MODEL.to_string(),
                LlmProviderKind::Groq => DEFAULT_GROQ_MODEL.to_string(),
            };
        } else {
            self.model = trimmed_model.to_string();
        }
    }

    pub fn from_env() -> Self {
        let cmd_key = env::var("CMD_API_KEY")
            .or_else(|_| env::var("COMMAND_CODE_API_KEY"))
            .ok()
            .and_then(|k| {
                let trimmed = k.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });

        let groq_key = env::var("GROQ_API_KEY").ok().and_then(|k| {
            let trimmed = k.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        let mut config = if let Some(key) = cmd_key {
            let base_url = env::var("CMD_BASE_URL")
                .or_else(|_| env::var("COMMAND_CODE_BASE_URL"))
                .unwrap_or_else(|_| DEFAULT_COMMAND_CODE_BASE_URL.to_string());
            let model = env::var("CMD_MODEL")
                .or_else(|_| env::var("COMMAND_CODE_MODEL"))
                .unwrap_or_else(|_| DEFAULT_COMMAND_CODE_MODEL.to_string());
            Self {
                api_key: Some(key),
                base_url,
                model,
                provider: LlmProviderKind::CommandCode,
            }
        } else if let Some(key) = groq_key {
            let base_url =
                env::var("GROQ_BASE_URL").unwrap_or_else(|_| DEFAULT_GROQ_BASE_URL.to_string());
            let model = env::var("GROQ_MODEL").unwrap_or_else(|_| DEFAULT_GROQ_MODEL.to_string());
            Self {
                api_key: Some(key),
                base_url,
                model,
                provider: LlmProviderKind::Groq,
            }
        } else {
            let base_url = env::var("CMD_BASE_URL")
                .or_else(|_| env::var("COMMAND_CODE_BASE_URL"))
                .unwrap_or_else(|_| DEFAULT_COMMAND_CODE_BASE_URL.to_string());
            let model = env::var("CMD_MODEL")
                .or_else(|_| env::var("COMMAND_CODE_MODEL"))
                .unwrap_or_else(|_| DEFAULT_COMMAND_CODE_MODEL.to_string());
            Self {
                api_key: None,
                base_url,
                model,
                provider: LlmProviderKind::CommandCode,
            }
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
    pub remote_separation_timeout_secs: u64,
    pub joshua_pc: Option<JoshuaPcConfig>,
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

        let joshua_pc = match env::var("JOSHUA_PC_HOST") {
            Ok(raw_host) => {
                let host = raw_host.trim();
                if host.is_empty() {
                    None
                } else {
                    if host.starts_with('-') {
                        return Err("JOSHUA_PC_HOST must not start with '-'".to_string());
                    }
                    if host
                        .chars()
                        .any(|c| c.is_whitespace() || ";|&$`><\"'\\".contains(c))
                    {
                        return Err(
                            "JOSHUA_PC_HOST contains invalid characters or shell metacharacters"
                                .to_string(),
                        );
                    }

                    let user = match env::var("JOSHUA_PC_USER") {
                        Ok(raw_user) => {
                            let u = raw_user.trim();
                            if u.is_empty() {
                                "loonbac".to_string()
                            } else {
                                if u.starts_with('-') {
                                    return Err(
                                        "JOSHUA_PC_USER must not start with '-'".to_string()
                                    );
                                }
                                if u.chars()
                                    .any(|c| c.is_whitespace() || ";|&$`><\"'\\".contains(c))
                                {
                                    return Err(
                                        "JOSHUA_PC_USER contains invalid characters".to_string()
                                    );
                                }
                                u.to_string()
                            }
                        }
                        Err(_) => "loonbac".to_string(),
                    };

                    let key_path = env::var("JOSHUA_PC_SSH_KEY")
                        .map(|v| {
                            let trimmed = v.trim();
                            if trimmed.is_empty() {
                                PathBuf::from("/home/shirley/.ssh/id_ed25519")
                            } else {
                                PathBuf::from(trimmed)
                            }
                        })
                        .unwrap_or_else(|_| PathBuf::from("/home/shirley/.ssh/id_ed25519"));

                    let port = match env::var("JOSHUA_PC_PORT") {
                        Ok(raw_port) => {
                            let trimmed = raw_port.trim();
                            if trimmed.is_empty() {
                                22
                            } else {
                                let p: u16 = trimmed.parse().map_err(|e| {
                                    format!("Invalid JOSHUA_PC_PORT '{trimmed}': {e}")
                                })?;
                                if p == 0 {
                                    return Err(
                                        "JOSHUA_PC_PORT must be strictly positive (1..=65535)"
                                            .to_string(),
                                    );
                                }
                                p
                            }
                        }
                        Err(_) => 22,
                    };

                    let wake_timeout_mins = match env::var("JOSHUA_PC_WAKE_TIMEOUT_MINS") {
                        Ok(raw_mins) => {
                            let trimmed = raw_mins.trim();
                            if trimmed.is_empty() {
                                10
                            } else {
                                let mins: u64 = trimmed.parse().map_err(|e| {
                                    format!("Invalid JOSHUA_PC_WAKE_TIMEOUT_MINS '{trimmed}': {e}")
                                })?;
                                if mins == 0 {
                                    return Err(
                                        "JOSHUA_PC_WAKE_TIMEOUT_MINS must be strictly positive"
                                            .to_string(),
                                    );
                                }
                                mins
                            }
                        }
                        Err(_) => 10,
                    };
                    let wake_timeout = Duration::from_secs(wake_timeout_mins.saturating_mul(60));

                    Some(JoshuaPcConfig {
                        host: host.to_string(),
                        user,
                        key_path,
                        port,
                        wake_timeout,
                    })
                }
            }
            Err(_) => None,
        };

        let default_remote_timeout = joshua_pc
            .as_ref()
            .map(|j| {
                j.wake_timeout
                    .as_secs()
                    .saturating_add(separation_timeout_secs)
                    .saturating_add(300)
            })
            .unwrap_or_else(|| separation_timeout_secs.saturating_add(900).max(1500));

        let remote_separation_timeout_secs =
            match env::var("SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS") {
                Ok(val) => {
                    let trimmed = val.trim();
                    if trimmed.is_empty() {
                        default_remote_timeout
                    } else {
                        let parsed: u64 = trimmed.parse().map_err(|e| {
                            format!(
                                "Invalid SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS '{trimmed}': {e}"
                            )
                        })?;
                        if parsed == 0 {
                            return Err(
                                "SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS must be strictly positive"
                                    .to_string(),
                            );
                        }
                        parsed
                    }
                }
                Err(_) => default_remote_timeout,
            };

        if let Some(ref jpc) = joshua_pc {
            let min_required = jpc
                .wake_timeout
                .as_secs()
                .saturating_add(separation_timeout_secs);
            if remote_separation_timeout_secs < min_required {
                return Err(format!(
                    "SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS ({remote_separation_timeout_secs}s) must be at least wake_timeout + separation_timeout ({min_required}s)"
                ));
            }
        }

        Ok(Self {
            bind_addr,
            data_dir,
            frontend_dir,
            groq,
            yt_dlp_bin,
            agent_timeout_secs,
            separation_timeout_secs,
            remote_separation_timeout_secs,
            joshua_pc,
        })
    }
}
