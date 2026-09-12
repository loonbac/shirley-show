use crate::domain::audio_format::AudioFormat;
use crate::error::AppError;

pub fn detect_audio_format(header: &[u8], extension: &str) -> Result<AudioFormat, AppError> {
    let expected_format = AudioFormat::from_extension(extension).ok_or_else(|| {
        AppError::unsupported_media_type(
            "unsupported_media_type",
            "Extensión de archivo no soportada. Solo se admiten mp3, wav y m4a.",
        )
    })?;

    let detected = match expected_format {
        AudioFormat::Mp3 => {
            let is_id3 = header.len() >= 3 && &header[0..3] == b"ID3";
            let is_sync = header.len() >= 2 && header[0] == 0xFF && (header[1] & 0xE0) == 0xE0;
            if is_id3 || is_sync {
                Some(AudioFormat::Mp3)
            } else {
                None
            }
        }
        AudioFormat::Wav => {
            if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE" {
                Some(AudioFormat::Wav)
            } else {
                None
            }
        }
        AudioFormat::M4a => {
            if header.len() >= 8 && &header[4..8] == b"ftyp" {
                Some(AudioFormat::M4a)
            } else {
                None
            }
        }
    };

    match detected {
        Some(fmt) if fmt == expected_format => Ok(fmt),
        _ => Err(AppError::unsupported_media_type(
            "unsupported_media_type",
            "La firma de audio (magic bytes) no coincide con el formato esperado.",
        )),
    }
}

pub fn detect_audio_format_from_bytes(header: &[u8]) -> Option<AudioFormat> {
    let is_id3 = header.len() >= 3 && &header[0..3] == b"ID3";
    let is_sync = header.len() >= 2 && header[0] == 0xFF && (header[1] & 0xE0) == 0xE0;
    if is_id3 || is_sync {
        Some(AudioFormat::Mp3)
    } else if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE" {
        Some(AudioFormat::Wav)
    } else if header.len() >= 8 && &header[4..8] == b"ftyp" {
        Some(AudioFormat::M4a)
    } else {
        None
    }
}
