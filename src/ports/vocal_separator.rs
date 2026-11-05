use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use crate::domain::vocal_separation_mode::VocalSeparationMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocalSeparationRequest {
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub mode: VocalSeparationMode,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparatedAudio {
    pub duration_secs: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VocalSeparationError {
    #[error("No se pudo verificar la duración del audio fuente")]
    InvalidAudioDuration,
    #[error("El audio supera el límite máximo de 10 minutos para separación vocal")]
    DurationExceeded,
    #[error("El modo instrumental no está disponible en el servidor")]
    InstrumentalUnavailable,
    #[error(
        "El procesador de audio está ocupado con otra separación. Por favor, intenta de nuevo en unos minutos"
    )]
    GpuBusy,
    #[error("La separación produjo archivos de salida inválidos o incompletos")]
    InvalidSeparatorOutput,
    #[error("Los canales de audio producidos son incompatibles entre sí")]
    IncompatibleStems,
    #[error("Falló la codificación final del audio a MP3")]
    TranscodeFailed,
    #[error("La separación vocal tardó demasiado tiempo y fue cancelada")]
    SeparationTimeout,
    #[error("El archivo de credenciales de Google Colab no existe o no contiene un token válido")]
    ColabTokenMissing,
    #[error("Google Colab no está disponible o la cuota de GPU está agotada")]
    ColabUnavailable,
    #[error("Falló el proceso de separación vocal")]
    SeparationFailed,
    #[error(
        "La separación se completó pero no se pudo traer el resultado: ningún servidor de transferencia aceptó el archivo"
    )]
    TransferFailed,
}

impl VocalSeparationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidAudioDuration => "invalid_audio_duration",
            Self::DurationExceeded => "duration_exceeded",
            Self::InstrumentalUnavailable => "instrumental_unavailable",
            Self::GpuBusy => "gpu_busy",
            Self::InvalidSeparatorOutput => "invalid_separator_output",
            Self::IncompatibleStems => "incompatible_stems",
            Self::TranscodeFailed => "transcode_failed",
            Self::SeparationTimeout => "separation_timeout",
            Self::ColabTokenMissing => "colab_token_missing",
            Self::ColabUnavailable => "colab_unavailable",
            Self::TransferFailed => "transfer_failed",
            Self::SeparationFailed => "separation_failed",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidAudioDuration => "No se pudo verificar la duración del audio fuente",
            Self::DurationExceeded => {
                "El audio supera el límite máximo de 10 minutos para separación vocal"
            }
            Self::InstrumentalUnavailable => {
                "El modo instrumental no está disponible en el servidor"
            }
            Self::GpuBusy => {
                "El procesador de audio está ocupado con otra separación. Por favor, intenta de nuevo en unos minutos"
            }
            Self::InvalidSeparatorOutput => {
                "La separación produjo archivos de salida inválidos o incompletos"
            }
            Self::IncompatibleStems => "Los canales de audio producidos son incompatibles entre sí",
            Self::TranscodeFailed => "Falló la codificación final del audio a MP3",
            Self::SeparationTimeout => "La separación vocal tardó demasiado tiempo y fue cancelada",
            Self::ColabTokenMissing => {
                "El archivo de credenciales de Google Colab no existe o no contiene un token válido"
            }
            Self::ColabUnavailable => {
                "Google Colab no está disponible o la cuota de GPU está agotada"
            }
            Self::SeparationFailed => "Falló el proceso de separación vocal",
            Self::TransferFailed => {
                "La separación se completó pero no se pudo traer el resultado: ningún servidor de transferencia aceptó el archivo"
            }
        }
    }
}

pub trait VocalSeparator: Send + Sync {
    fn is_remote(&self) -> bool {
        false
    }

    fn probe(&self, _timeout: Duration) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        Box::pin(async { true })
    }

    fn separate(
        &self,
        request: VocalSeparationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<SeparatedAudio, VocalSeparationError>> + Send>>;
}
