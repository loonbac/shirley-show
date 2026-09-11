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
    #[error("La herramienta de separación vocal no está disponible en el servidor")]
    SeparatorNotConfigured,
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
    #[error(
        "Por favor, enciende la PC de Joshua; estaré esperando hasta que encienda y cuando lo haga haré el trabajo. Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo."
    )]
    WakeWait,
    #[error("No se pudo conectar con la PC remota dentro del tiempo de espera")]
    PcWakeTimeout,
    #[error("Permiso denegado para ejecutar comandos requeridos en la PC remota")]
    RemoteSudoDenied,
    #[error("La clave SSH para conectar con la PC remota no existe o no es legible")]
    SshKeyMissing,
    #[error("Falló el proceso de separación vocal")]
    SeparationFailed,
}

impl VocalSeparationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidAudioDuration => "invalid_audio_duration",
            Self::DurationExceeded => "duration_exceeded",
            Self::SeparatorNotConfigured => "separator_not_configured",
            Self::InstrumentalUnavailable => "instrumental_unavailable",
            Self::GpuBusy => "gpu_busy",
            Self::InvalidSeparatorOutput => "invalid_separator_output",
            Self::IncompatibleStems => "incompatible_stems",
            Self::TranscodeFailed => "transcode_failed",
            Self::SeparationTimeout => "separation_timeout",
            Self::WakeWait => "wake_wait",
            Self::PcWakeTimeout => "pc_wake_timeout",
            Self::RemoteSudoDenied => "remote_sudo_denied",
            Self::SshKeyMissing => "ssh_key_missing",
            Self::SeparationFailed => "separation_failed",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidAudioDuration => "No se pudo verificar la duración del audio fuente",
            Self::DurationExceeded => {
                "El audio supera el límite máximo de 10 minutos para separación vocal"
            }
            Self::SeparatorNotConfigured => {
                "La herramienta de separación vocal no está disponible en el servidor"
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
            Self::WakeWait => {
                "Por favor, enciende la PC de Joshua; estaré esperando hasta que encienda y cuando lo haga haré el trabajo. Cuando termine, la PC de Joshua se apagará automáticamente después de 1 hora sin trabajo."
            }
            Self::PcWakeTimeout => {
                "No se pudo conectar con la PC remota dentro del tiempo de espera"
            }
            Self::RemoteSudoDenied => {
                "Permiso denegado para ejecutar comandos requeridos en la PC remota"
            }
            Self::SshKeyMissing => {
                "La clave SSH para conectar con la PC remota no existe o no es legible"
            }
            Self::SeparationFailed => "Falló el proceso de separación vocal",
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
