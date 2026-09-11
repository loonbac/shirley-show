use std::fs;
use std::io::{Seek, Write};
use std::path::Path;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

/// Error types for `avg_wave` processing.
///
/// Error messages intentionally omit internal filesystem paths to prevent information leakage.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum AvgWaveError {
    /// An input WAV file could not be found, opened, or read.
    #[error("input audio file not found or invalid")]
    InputReadError,
    /// An input WAV file contains zero samples.
    #[error("empty audio file provided")]
    EmptyAudio,
    /// Input WAV files have different channel counts.
    #[error("audio channels mismatch")]
    MismatchedChannels,
    /// Input WAV files have different sample rates.
    #[error("audio sample rate mismatch")]
    MismatchedSampleRate,
    /// Input WAV files have different sample lengths or an iterator terminated early.
    #[error("audio length mismatch")]
    MismatchedLength,
    /// Input WAV files use an unsupported sample format or bit depth.
    #[error("unsupported audio format")]
    UnsupportedFormat,
    /// The output WAV file could not be created, written to, or finalized.
    #[error("failed to create or write output wav")]
    OutputWriteError,
}

#[inline(always)]
fn write_clamped_sample_24<W: Write + Seek>(
    writer: &mut WavWriter<W>,
    v0: f32,
    v1: f32,
    v2: f32,
) -> Result<(), AvgWaveError> {
    let avg = (v0 + v1 + v2) / 3.0;
    let clamped = avg.clamp(-1.0, 1.0);
    let scaled = (clamped * 8_388_608.0).round() as i64;
    let sample_24 = scaled.clamp(-8_388_608, 8_388_607) as i32;
    writer
        .write_sample(sample_24)
        .map_err(|_| AvgWaveError::OutputWriteError)
}

/// Blends exactly three WAV files with equal 1:1:1 weights in streaming O(1) memory.
///
/// Input WAVs must have identical channel count, sample rate, format and sample count.
/// Output is written as deterministic 24-bit PCM WAV.
/// If any error occurs, the output file is deterministically purged from disk.
pub fn avg_wave(inputs: [&Path; 3], output: &Path) -> Result<(), AvgWaveError> {
    let mut r0 = WavReader::open(inputs[0]).map_err(|_| AvgWaveError::InputReadError)?;
    let mut r1 = WavReader::open(inputs[1]).map_err(|_| AvgWaveError::InputReadError)?;
    let mut r2 = WavReader::open(inputs[2]).map_err(|_| AvgWaveError::InputReadError)?;

    let s0 = r0.spec();
    let s1 = r1.spec();
    let s2 = r2.spec();

    if s0.channels == 0 || s1.channels == 0 || s2.channels == 0 {
        return Err(AvgWaveError::UnsupportedFormat);
    }
    if s0.channels != s1.channels || s0.channels != s2.channels {
        return Err(AvgWaveError::MismatchedChannels);
    }

    if s0.sample_rate == 0 || s1.sample_rate == 0 || s2.sample_rate == 0 {
        return Err(AvgWaveError::UnsupportedFormat);
    }
    if s0.sample_rate != s1.sample_rate || s0.sample_rate != s2.sample_rate {
        return Err(AvgWaveError::MismatchedSampleRate);
    }

    if s0.sample_format != s1.sample_format || s0.sample_format != s2.sample_format {
        return Err(AvgWaveError::UnsupportedFormat);
    }
    if s0.bits_per_sample != s1.bits_per_sample || s0.bits_per_sample != s2.bits_per_sample {
        return Err(AvgWaveError::UnsupportedFormat);
    }

    match (s0.sample_format, s0.bits_per_sample) {
        (SampleFormat::Int, 16) | (SampleFormat::Int, 24) | (SampleFormat::Float, 32) => {}
        _ => return Err(AvgWaveError::UnsupportedFormat),
    }

    let len0 = r0.len();
    let len1 = r1.len();
    let len2 = r2.len();

    if len0 == 0 || len1 == 0 || len2 == 0 {
        return Err(AvgWaveError::EmptyAudio);
    }
    if len0 != len1 || len0 != len2 {
        return Err(AvgWaveError::MismatchedLength);
    }

    let out_spec = WavSpec {
        channels: s0.channels,
        sample_rate: s0.sample_rate,
        bits_per_sample: 24,
        sample_format: SampleFormat::Int,
    };

    let mut run_pipeline = || -> Result<(), AvgWaveError> {
        let mut writer =
            WavWriter::create(output, out_spec).map_err(|_| AvgWaveError::OutputWriteError)?;

        match (s0.sample_format, s0.bits_per_sample) {
            (SampleFormat::Int, 16) => {
                let mut it0 = r0.samples::<i32>();
                let mut it1 = r1.samples::<i32>();
                let mut it2 = r2.samples::<i32>();
                loop {
                    match (it0.next(), it1.next(), it2.next()) {
                        (Some(s0), Some(s1), Some(s2)) => {
                            let v0 =
                                (s0.map_err(|_| AvgWaveError::InputReadError)? as f32) / 32768.0;
                            let v1 =
                                (s1.map_err(|_| AvgWaveError::InputReadError)? as f32) / 32768.0;
                            let v2 =
                                (s2.map_err(|_| AvgWaveError::InputReadError)? as f32) / 32768.0;
                            write_clamped_sample_24(&mut writer, v0, v1, v2)?;
                        }
                        (None, None, None) => break,
                        _ => return Err(AvgWaveError::MismatchedLength),
                    }
                }
            }
            (SampleFormat::Int, 24) => {
                let mut it0 = r0.samples::<i32>();
                let mut it1 = r1.samples::<i32>();
                let mut it2 = r2.samples::<i32>();
                loop {
                    match (it0.next(), it1.next(), it2.next()) {
                        (Some(s0), Some(s1), Some(s2)) => {
                            let v0 =
                                (s0.map_err(|_| AvgWaveError::InputReadError)? as f32) / 8_388_608.0;
                            let v1 =
                                (s1.map_err(|_| AvgWaveError::InputReadError)? as f32) / 8_388_608.0;
                            let v2 =
                                (s2.map_err(|_| AvgWaveError::InputReadError)? as f32) / 8_388_608.0;
                            write_clamped_sample_24(&mut writer, v0, v1, v2)?;
                        }
                        (None, None, None) => break,
                        _ => return Err(AvgWaveError::MismatchedLength),
                    }
                }
            }
            (SampleFormat::Float, 32) => {
                let mut it0 = r0.samples::<f32>();
                let mut it1 = r1.samples::<f32>();
                let mut it2 = r2.samples::<f32>();
                loop {
                    match (it0.next(), it1.next(), it2.next()) {
                        (Some(s0), Some(s1), Some(s2)) => {
                            let v0 = s0.map_err(|_| AvgWaveError::InputReadError)?;
                            let v1 = s1.map_err(|_| AvgWaveError::InputReadError)?;
                            let v2 = s2.map_err(|_| AvgWaveError::InputReadError)?;
                            write_clamped_sample_24(&mut writer, v0, v1, v2)?;
                        }
                        (None, None, None) => break,
                        _ => return Err(AvgWaveError::MismatchedLength),
                    }
                }
            }
            _ => return Err(AvgWaveError::UnsupportedFormat),
        }

        writer
            .finalize()
            .map_err(|_| AvgWaveError::OutputWriteError)?;
        Ok(())
    };

    let result = run_pipeline();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result
}
