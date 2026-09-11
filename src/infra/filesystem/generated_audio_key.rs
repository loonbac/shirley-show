use crate::domain::audio_format::AudioFormat;
use crate::error::AppError;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedAudioKey {
    raw: String,
    format: AudioFormat,
}

impl GeneratedAudioKey {
    pub fn generate(format: AudioFormat) -> Self {
        let id = Uuid::new_v4().to_string();
        let raw = format!("{id}.{}", format.extension());
        Self { raw, format }
    }

    pub fn parse(key: &str) -> Result<Self, AppError> {
        let trimmed = key.trim();
        if trimmed.is_empty()
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains("..")
        {
            return Err(AppError::validation(
                "invalid_audio_key",
                "Clave de audio inválida o no segura.",
                "audio_key",
            ));
        }

        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() != 2 {
            return Err(AppError::validation(
                "invalid_audio_key",
                "Formato de clave de audio inválido.",
                "audio_key",
            ));
        }

        let uuid_part = parts[0];
        let ext_part = parts[1];

        Uuid::parse_str(uuid_part).map_err(|_| {
            AppError::validation(
                "invalid_audio_key",
                "La clave de audio debe contener un UUID válido.",
                "audio_key",
            )
        })?;

        let format = AudioFormat::from_extension(ext_part).ok_or_else(|| {
            AppError::validation(
                "unsupported_media_type",
                "Extensión de audio no soportada.",
                "audio_key",
            )
        })?;

        Ok(Self {
            raw: trimmed.to_string(),
            format,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn format(&self) -> AudioFormat {
        self.format
    }
}
