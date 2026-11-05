use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";
pub const DEFAULT_GROQ_MODEL: &str = "openai/gpt-oss-120b";
pub const DEFAULT_OPENCODE_BASE_URL: &str = "https://opencode.ai/inference/go/openai/v1";
// Cheapest model that still completes a full multi-step tool flow. The Go plan
// charges per token against a monthly cap; deepseek-v4.1-flash has the largest
// cap of the cheap tier, so a long show day does not exhaust it.
pub const DEFAULT_OPENCODE_MODEL: &str = "deepseek-v4.1-flash";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LlmProviderKind {
    #[default]
    OpenCode,
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
            base_url: DEFAULT_OPENCODE_BASE_URL.to_string(),
            model: DEFAULT_OPENCODE_MODEL.to_string(),
            provider: LlmProviderKind::OpenCode,
        }
    }
}

impl GroqConfig {
    pub fn provider_name(&self) -> &'static str {
        match self.provider {
            LlmProviderKind::OpenCode => "OpenCode Go",
            LlmProviderKind::Groq => "Groq",
        }
    }

    pub fn is_opencode(&self) -> bool {
        self.provider == LlmProviderKind::OpenCode
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
                LlmProviderKind::OpenCode => DEFAULT_OPENCODE_BASE_URL.to_string(),
                LlmProviderKind::Groq => DEFAULT_GROQ_BASE_URL.to_string(),
            };
        } else {
            self.base_url = trimmed_url.to_string();
        }

        let trimmed_model = self.model.trim();
        if trimmed_model.is_empty() {
            self.model = match self.provider {
                LlmProviderKind::OpenCode => DEFAULT_OPENCODE_MODEL.to_string(),
                LlmProviderKind::Groq => DEFAULT_GROQ_MODEL.to_string(),
            };
        } else {
            self.model = trimmed_model.to_string();
        }
    }

    pub fn from_env() -> Self {
        // Legacy names stay readable so an operator upgrading from the
        // Command Code build does not lose the assistant silently.
        let opencode_key = env::var("OPENCODE_API_KEY")
            .or_else(|_| env::var("CMD_API_KEY"))
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

        let mut config = if let Some(key) = opencode_key {
            let base_url = env::var("OPENCODE_BASE_URL")
                .or_else(|_| env::var("CMD_BASE_URL"))
                .unwrap_or_else(|_| DEFAULT_OPENCODE_BASE_URL.to_string());
            let model = env::var("OPENCODE_MODEL")
                .or_else(|_| env::var("CMD_MODEL"))
                .unwrap_or_else(|_| DEFAULT_OPENCODE_MODEL.to_string());
            Self {
                api_key: Some(key),
                base_url,
                model,
                provider: LlmProviderKind::OpenCode,
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
            let base_url = env::var("OPENCODE_BASE_URL")
                .or_else(|_| env::var("CMD_BASE_URL"))
                .unwrap_or_else(|_| DEFAULT_OPENCODE_BASE_URL.to_string());
            let model = env::var("OPENCODE_MODEL")
                .or_else(|_| env::var("CMD_MODEL"))
                .unwrap_or_else(|_| DEFAULT_OPENCODE_MODEL.to_string());
            Self {
                api_key: None,
                base_url,
                model,
                provider: LlmProviderKind::OpenCode,
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
    /// Which host serves the public landing and which one serves the panel.
    pub surface: SurfaceConfig,
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

        let surface = SurfaceConfig::new(
            env::var("SHIRLEY_PUBLIC_BASE_URL").ok().as_deref(),
            env::var("SHIRLEY_PANEL_BASE_URL").ok().as_deref(),
        );

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
            surface,
        })
    }
}

/// Which surface a request is allowed to reach, decided from its `Host`.
///
/// The public landing and the management application share one container. The
/// landing makes no `/api` calls at all, but `/api/agent/chat` has no
/// authentication layer, so a single rule at the proxy must never be the only
/// thing keeping the API closed: a misconfigured proxy, or anything that
/// reaches the container directly, would publish it. The cut lives here.
///
/// Hosts are supplied explicitly rather than read from the environment so the
/// rule can be tested without mutating process state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceConfig {
    /// Host of the public landing, for example `shirleyshow.site`.
    pub site_host: String,
    /// Host of the management application, for example
    /// `panel.shirleyshow.site`.
    pub panel_host: String,
}

/// What a given request may reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicSurface {
    /// The public landing: only the landing document and its assets.
    Landing,
    /// The management application: panel, API, tickets and the door.
    Application,
    /// Unknown or missing host: everything stays closed.
    Closed,
}

impl SurfaceConfig {
    /// Builds the rule from configuration, tolerating a missing panel host.
    ///
    /// With no panel host the deployment behaves as a single host: everything
    /// is served, which is the state production was in before the split. That
    /// keeps an existing deployment working if the variable is not set yet.
    pub fn new(site_base_url: Option<&str>, panel_base_url: Option<&str>) -> Self {
        Self {
            site_host: host_of(site_base_url).unwrap_or_default(),
            panel_host: host_of(panel_base_url).unwrap_or_default(),
        }
    }

    /// Classifies a request by the `Host` header it arrived with.
    pub fn classify(&self, host: Option<&str>) -> PublicSurface {
        // No split configured: one host serves everything, as before. This is
        // checked first on purpose: otherwise a deployment that never opted in
        // would close on every request without a Host header, which is how the
        // existing tests (and any direct call) behave.
        if self.panel_host.is_empty() {
            return PublicSurface::Application;
        }

        let Some(host) = normalize_host(host) else {
            return PublicSurface::Closed;
        };

        if host == self.panel_host {
            return PublicSurface::Application;
        }

        if !self.site_host.is_empty()
            && (host == self.site_host || host == format!("www.{}", self.site_host))
        {
            return PublicSurface::Landing;
        }

        PublicSurface::Closed
    }
}

/// Extracts the bare hostname from a base URL such as `https://a.b:8080/x`.
fn host_of(base_url: Option<&str>) -> Option<String> {
    let url = base_url?.trim().trim_end_matches('/');
    let without_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = without_scheme.split('/').next()?.split(':').next()?;
    let host = host.trim().to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// Lowercases a `Host` header and drops any port.
fn normalize_host(host: Option<&str>) -> Option<String> {
    let host = host?.trim().to_ascii_lowercase();
    let host = host.split(':').next()?.trim();
    (!host.is_empty()).then(|| host.to_string())
}

impl PublicSurface {
    /// The landing document and its assets.
    pub fn serves_landing(self) -> bool {
        matches!(self, Self::Landing | Self::Application)
    }

    pub fn serves_panel(self) -> bool {
        matches!(self, Self::Application)
    }

    /// The API, including `/api/agent/chat`, which has no auth layer.
    pub fn serves_api(self) -> bool {
        matches!(self, Self::Application)
    }
}
