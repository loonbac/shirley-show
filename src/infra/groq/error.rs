use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider_auth")]
    Authentication,

    #[error("provider_rate_limited")]
    RateLimited,

    #[error("provider_unavailable")]
    Unavailable,

    #[error("agent_timeout")]
    Timeout,

    #[error("provider_error: {0}")]
    Rejected(String),

    #[error("provider_error: {0}")]
    MalformedResponse(String),
}

impl ProviderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Authentication => "provider_auth",
            Self::RateLimited => "provider_rate_limited",
            Self::Unavailable => "provider_unavailable",
            Self::Timeout => "agent_timeout",
            Self::Rejected(_) | Self::MalformedResponse(_) => "provider_error",
        }
    }

    pub fn friendly_message(&self) -> &'static str {
        match self {
            Self::Authentication => {
                "No se pudo autenticar con el proveedor de IA. Por favor, verifica la clave de API configurada en el servidor."
            }
            Self::RateLimited => {
                "Ressy está experimentando una alta demanda en este momento. Por favor, intenta de nuevo en unos instantes o utiliza los controles directos."
            }
            Self::Unavailable => {
                "El proveedor de IA no está disponible temporalmente. Por favor, intenta de nuevo más tarde."
            }
            Self::Timeout => {
                "La solicitud al asistente de IA tardó demasiado tiempo en responder. Por favor, intenta de nuevo."
            }
            Self::Rejected(_) | Self::MalformedResponse(_) => {
                "Ocurrió un error al procesar la respuesta del proveedor de IA. Por favor, intenta de nuevo."
            }
        }
    }

    pub fn retryable(&self) -> bool {
        match self {
            Self::Authentication => false,
            Self::RateLimited | Self::Unavailable | Self::Timeout => true,
            Self::Rejected(_) | Self::MalformedResponse(_) => false,
        }
    }
}
