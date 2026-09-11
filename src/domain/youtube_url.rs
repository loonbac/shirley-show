use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YouTubeUrl {
    video_id: String,
    canonical_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum YouTubeUrlError {
    #[error("La URL contiene caracteres no permitidos")]
    InvalidCharacters,
    #[error("Solo se admiten enlaces públicos de YouTube (youtube.com o youtu.be)")]
    UnsupportedHost,
    #[error("Solo se admiten enlaces públicos de YouTube (youtube.com o youtu.be)")]
    UnsupportedPath,
    #[error("El enlace no contiene un identificador de video válido de 11 caracteres")]
    InvalidVideoId,
    #[error("Solo se admiten enlaces públicos de YouTube (youtube.com o youtu.be)")]
    InvalidScheme,
    #[error("Solo se admiten enlaces públicos de YouTube (youtube.com o youtu.be)")]
    Malformed,
}

impl YouTubeUrlError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidCharacters => "invalid_characters",
            Self::InvalidVideoId => "invalid_video_id",
            Self::UnsupportedHost | Self::UnsupportedPath | Self::InvalidScheme | Self::Malformed => {
                "unsupported_url"
            }
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidCharacters => "La URL contiene caracteres no permitidos",
            Self::InvalidVideoId => {
                "El enlace no contiene un identificador de video válido de 11 caracteres"
            }
            Self::UnsupportedHost | Self::UnsupportedPath | Self::InvalidScheme | Self::Malformed => {
                "Solo se admiten enlaces públicos de YouTube (youtube.com o youtu.be)"
            }
        }
    }
}

impl YouTubeUrl {
    pub fn parse(raw: &str) -> Result<Self, YouTubeUrlError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(YouTubeUrlError::Malformed);
        }

        // Reject shell metacharacters strictly
        const FORBIDDEN_CHARS: &[char] = &[';', '|', '$', '`', '\\', '"', '\''];
        if trimmed.chars().any(|c| FORBIDDEN_CHARS.contains(&c)) {
            return Err(YouTubeUrlError::InvalidCharacters);
        }

        let parsed = Url::parse(trimmed).map_err(|_| YouTubeUrlError::Malformed)?;

        // Only http and https
        match parsed.scheme() {
            "http" | "https" => {}
            _ => return Err(YouTubeUrlError::InvalidScheme),
        }

        // Disallow credentials
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(YouTubeUrlError::UnsupportedHost);
        }

        // Disallow non-default ports
        if parsed.port().is_some() {
            return Err(YouTubeUrlError::UnsupportedHost);
        }

        let host = parsed.host_str().ok_or(YouTubeUrlError::UnsupportedHost)?;

        let video_id = match host {
            "youtu.be" => {
                // Shortened URL: https://youtu.be/{id}
                let path = parsed.path().trim_start_matches('/');
                // Ensure there are no additional path segments
                if path.contains('/') {
                    return Err(YouTubeUrlError::InvalidVideoId);
                }
                path.to_string()
            }
            "youtube.com" | "www.youtube.com" | "m.youtube.com" => {
                let path = parsed.path();
                if path == "/watch" {
                    let mut found_id: Option<String> = None;
                    for (key, value) in parsed.query_pairs() {
                        if key == "v" {
                            found_id = Some(value.into_owned());
                            break;
                        }
                    }
                    found_id.ok_or(YouTubeUrlError::InvalidVideoId)?
                } else if let Some(shorts_id) = path.strip_prefix("/shorts/") {
                    let clean_id = shorts_id.trim_end_matches('/');
                    if clean_id.contains('/') {
                        return Err(YouTubeUrlError::InvalidVideoId);
                    }
                    clean_id.to_string()
                } else {
                    return Err(YouTubeUrlError::UnsupportedPath);
                }
            }
            _ => return Err(YouTubeUrlError::UnsupportedHost),
        };

        // Validate that video_id has exactly 11 ASCII characters matching [A-Za-z0-9_-]
        if !Self::is_valid_video_id(&video_id) {
            return Err(YouTubeUrlError::InvalidVideoId);
        }

        let canonical_url = format!("https://www.youtube.com/watch?v={video_id}");

        Ok(Self {
            video_id,
            canonical_url,
        })
    }

    pub fn video_id(&self) -> &str {
        &self.video_id
    }

    pub fn canonical_url(&self) -> &str {
        &self.canonical_url
    }

    fn is_valid_video_id(id: &str) -> bool {
        if id.len() != 11 {
            return false;
        }
        id.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }
}
