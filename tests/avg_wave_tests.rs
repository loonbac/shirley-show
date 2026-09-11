use std::fs;
use std::path::Path;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use tempfile::tempdir;

use shirley_show::audio::avg_wave::{avg_wave, AvgWaveError};
use shirley_show::audio::encode_mp3::{encode_mp3_320k, EncodeMp3Error};
use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;

/// Helper to create a synthetic WAV file on disk with specified spec and samples.
fn write_synthetic_wav_i16(path: &Path, spec: WavSpec, samples: &[i16]) {
    let mut writer = WavWriter::create(path, spec).expect("failed to create wav writer");
    for &sample in samples {
        writer.write_sample(sample).expect("failed to write sample");
    }
    writer.finalize().expect("failed to finalize wav");
}

fn write_synthetic_wav_i32_24(path: &Path, spec: WavSpec, samples: &[i32]) {
    let mut writer = WavWriter::create(path, spec).expect("failed to create wav writer");
    for &sample in samples {
        writer.write_sample(sample).expect("failed to write sample");
    }
    writer.finalize().expect("failed to finalize wav");
}

fn write_synthetic_wav_f32(path: &Path, spec: WavSpec, samples: &[f32]) {
    let mut writer = WavWriter::create(path, spec).expect("failed to create wav writer");
    for &sample in samples {
        writer.write_sample(sample).expect("failed to write sample");
    }
    writer.finalize().expect("failed to finalize wav");
}

// ============================================================================
// VocalSeparationMode Tests
// ============================================================================

#[test]
fn test_vocal_separation_mode_serde_roundtrip() {
    let chorus: VocalSeparationMode = serde_json::from_str("\"chorus\"").expect("should parse chorus");
    assert_eq!(chorus, VocalSeparationMode::Coros);
    assert_eq!(serde_json::to_string(&chorus).unwrap(), "\"chorus\"");
    assert_eq!(chorus.as_str(), "chorus");
    assert_eq!(chorus.title_suffix(), " (Coros)");

    let instrumental: VocalSeparationMode =
        serde_json::from_str("\"instrumental\"").expect("should parse instrumental");
    assert_eq!(instrumental, VocalSeparationMode::Instrumental);
    assert_eq!(
        serde_json::to_string(&instrumental).unwrap(),
        "\"instrumental\""
    );
    assert_eq!(instrumental.as_str(), "instrumental");
    assert_eq!(instrumental.title_suffix(), " (Instrumental)");
}

#[test]
fn test_vocal_separation_mode_rejects_aliases_and_unknown() {
    let invalid_inputs = [
        "\"coro\"",
        "\"coros\"",
        "\"vocals\"",
        "\"vocal\"",
        "\"acapella\"",
        "\"karaoke\"",
        "\"CHORUS\"",
        "\"INSTRUMENTAL\"",
        "\"Chorus\"",
        "\"\"",
        "123",
        "null",
        "true",
    ];

    for input in invalid_inputs {
        let result: Result<VocalSeparationMode, _> = serde_json::from_str(input);
        assert!(
            result.is_err(),
            "input {input} should have been rejected by serde"
        );
    }
}

#[test]
fn test_vocal_separation_mode_from_str() {
    assert_eq!(
        "chorus".parse::<VocalSeparationMode>().unwrap(),
        VocalSeparationMode::Coros
    );
    assert_eq!(
        "instrumental".parse::<VocalSeparationMode>().unwrap(),
        VocalSeparationMode::Instrumental
    );
    assert!("coro".parse::<VocalSeparationMode>().is_err());
    assert!("invalid".parse::<VocalSeparationMode>().is_err());
}

// ============================================================================
// avg_wave Unit & Integration Tests
// ============================================================================

#[test]
fn test_avg_wave_three_mono_16bit_wavs() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("stem1.wav");
    let p2 = dir.path().join("stem2.wav");
    let p3 = dir.path().join("stem3.wav");
    let out = dir.path().join("avg_output.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    // S1: 16384 (norm ~ +0.5)
    // S2: -16384 (norm ~ -0.5)
    // S3: 0 (norm ~ 0.0)
    // Avg = (0.5 - 0.5 + 0.0)/3.0 = 0.0
    let s1 = [16384, -32768, 32767, 0];
    let s2 = [-16384, -32768, 32767, 1000];
    let s3 = [0, -32768, 32767, -1000];

    write_synthetic_wav_i16(&p1, spec, &s1);
    write_synthetic_wav_i16(&p2, spec, &s2);
    write_synthetic_wav_i16(&p3, spec, &s3);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(res.is_ok(), "avg_wave should succeed on valid inputs: {res:?}");
    assert!(out.exists(), "output WAV file must exist on success");

    let mut reader = WavReader::open(&out).expect("output must be valid WAV");
    let out_spec = reader.spec();
    assert_eq!(out_spec.channels, 1);
    assert_eq!(out_spec.sample_rate, 44100);
    assert_eq!(out_spec.bits_per_sample, 24);
    assert_eq!(out_spec.sample_format, SampleFormat::Int);

    let out_samples: Vec<i32> = reader.samples::<i32>().map(|s| s.unwrap()).collect();
    assert_eq!(out_samples.len(), 4);

    // Sample 0: (0.5 - 0.5 + 0.0) / 3 = 0.0 -> 0
    assert_eq!(out_samples[0], 0);

    // Sample 1: all -32768 (-1.0) -> (-1 + -1 + -1)/3 = -1.0 -> -8388608
    assert_eq!(out_samples[1], -8_388_608);

    // Sample 2: all +32767 (~+1.0) -> ~+1.0 -> close to +8388607
    assert!((out_samples[2] - 8_388_607).abs() <= 500);

    // Sample 3: (0 + 1000 - 1000) / 3 = 0 -> 0
    assert_eq!(out_samples[3], 0);
}

#[test]
fn test_avg_wave_three_stereo_24bit_wavs() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("s1.wav");
    let p2 = dir.path().join("s2.wav");
    let p3 = dir.path().join("s3.wav");
    let out = dir.path().join("avg_stereo.wav");

    let spec = WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 24,
        sample_format: SampleFormat::Int,
    };

    // 2 stereo frames = 4 samples (L0, R0, L1, R1)
    let s1 = [4_194_304, -4_194_304, 2_000_000, -2_000_000];
    let s2 = [4_194_304, -4_194_304, 1_000_000, -1_000_000];
    let s3 = [4_194_304, -4_194_304, 3_000_000, -3_000_000];

    write_synthetic_wav_i32_24(&p1, spec, &s1);
    write_synthetic_wav_i32_24(&p2, spec, &s2);
    write_synthetic_wav_i32_24(&p3, spec, &s3);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(res.is_ok(), "stereo avg_wave failed: {res:?}");

    let mut reader = WavReader::open(&out).expect("output must be valid WAV");
    let out_spec = reader.spec();
    assert_eq!(out_spec.channels, 2);
    assert_eq!(out_spec.sample_rate, 48000);
    assert_eq!(out_spec.bits_per_sample, 24);

    let out_samples: Vec<i32> = reader.samples::<i32>().map(|s| s.unwrap()).collect();
    assert_eq!(out_samples.len(), 4);

    // L0: 4194304 / 8388608 = 0.5; avg of 3 identical = 0.5 -> 4194304 (within roundoff)
    assert!((out_samples[0] - 4_194_304).abs() <= 2);
    // R0: -4194304 / 8388608 = -0.5 -> -4194304
    assert!((out_samples[1] - -4_194_304).abs() <= 2);
    // L1: avg(2M, 1M, 3M) = 2M -> 2000000
    assert!((out_samples[2] - 2_000_000).abs() <= 2);
    // R1: avg(-2M, -1M, -3M) = -2M -> -2000000
    assert!((out_samples[3] - -2_000_000).abs() <= 2);
}

#[test]
fn test_avg_wave_three_32bit_float_wavs() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("f1.wav");
    let p2 = dir.path().join("f2.wav");
    let p3 = dir.path().join("f3.wav");
    let out = dir.path().join("avg_float.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let s1 = [0.3f32, -0.6f32];
    let s2 = [0.6f32, -0.3f32];
    let s3 = [0.3f32, 0.0f32];

    write_synthetic_wav_f32(&p1, spec, &s1);
    write_synthetic_wav_f32(&p2, spec, &s2);
    write_synthetic_wav_f32(&p3, spec, &s3);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(res.is_ok(), "float avg_wave failed: {res:?}");

    let mut reader = WavReader::open(&out).unwrap();
    let out_samples: Vec<i32> = reader.samples::<i32>().map(|s| s.unwrap()).collect();
    assert_eq!(out_samples.len(), 2);

    // Sample 0: avg(0.3, 0.6, 0.3) = 0.4 -> 0.4 * 8388607 = 3355443
    assert!((out_samples[0] - 3_355_443).abs() <= 10);
    // Sample 1: avg(-0.6, -0.3, 0.0) = -0.3 -> -0.3 * 8388607 = -2516582
    assert!((out_samples[1] - -2_516_582).abs() <= 10);
}

#[test]
fn test_avg_wave_clamps_extreme_peaks() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("peak1.wav");
    let p2 = dir.path().join("peak2.wav");
    let p3 = dir.path().join("peak3.wav");
    let out = dir.path().join("avg_clamped.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    // Extreme float values that would exceed +1.0 or <-1.0
    let s1 = [2.0f32, -3.0f32];
    let s2 = [1.5f32, -2.0f32];
    let s3 = [1.0f32, -1.0f32];

    write_synthetic_wav_f32(&p1, spec, &s1);
    write_synthetic_wav_f32(&p2, spec, &s2);
    write_synthetic_wav_f32(&p3, spec, &s3);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(res.is_ok());

    let mut reader = WavReader::open(&out).unwrap();
    let out_samples: Vec<i32> = reader.samples::<i32>().map(|s| s.unwrap()).collect();
    assert_eq!(out_samples.len(), 2);

    // Sample 0: avg = 4.5/3 = 1.5 -> clamped to +1.0 -> 8388607
    assert_eq!(out_samples[0], 8_388_607);
    // Sample 1: avg = -6.0/3 = -2.0 -> clamped to -1.0 -> -8388608
    assert_eq!(out_samples[1], -8_388_608);
}

#[test]
fn test_avg_wave_rejects_sample_rate_disparity_and_purges_output() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("sr1.wav");
    let p2 = dir.path().join("sr2.wav");
    let p3 = dir.path().join("sr3.wav");
    let out = dir.path().join("rejected_sr.wav");

    let spec1 = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let spec2 = WavSpec {
        channels: 1,
        sample_rate: 48000, // Disparity!
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    write_synthetic_wav_i16(&p1, spec1, &[100, 200]);
    write_synthetic_wav_i16(&p2, spec2, &[100, 200]);
    write_synthetic_wav_i16(&p3, spec1, &[100, 200]);

    let res = avg_wave([&p1, &p2, &p3], &out);
    match res {
        Err(AvgWaveError::MismatchedSampleRate) => (),
        other => panic!("expected MismatchedSampleRate, got: {other:?}"),
    }
    assert!(!out.exists(), "output file must not exist after error");
}

#[test]
fn test_avg_wave_rejects_channel_disparity_and_purges_output() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("ch1.wav");
    let p2 = dir.path().join("ch2.wav");
    let p3 = dir.path().join("ch3.wav");
    let out = dir.path().join("rejected_ch.wav");

    let spec_mono = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let spec_stereo = WavSpec {
        channels: 2, // Disparity!
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    write_synthetic_wav_i16(&p1, spec_mono, &[100, 200]);
    write_synthetic_wav_i16(&p2, spec_stereo, &[100, 200, 300, 400]);
    write_synthetic_wav_i16(&p3, spec_mono, &[100, 200]);

    let res = avg_wave([&p1, &p2, &p3], &out);
    match res {
        Err(AvgWaveError::MismatchedChannels) => (),
        other => panic!("expected MismatchedChannels, got: {other:?}"),
    }
    assert!(!out.exists(), "output file must not exist after error");
}

#[test]
fn test_avg_wave_rejects_length_disparity_and_purges_output() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("len1.wav");
    let p2 = dir.path().join("len2.wav");
    let p3 = dir.path().join("len3.wav");
    let out = dir.path().join("rejected_len.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    write_synthetic_wav_i16(&p1, spec, &[100, 200, 300]);
    write_synthetic_wav_i16(&p2, spec, &[100, 200]); // Disparity: only 2 samples!
    write_synthetic_wav_i16(&p3, spec, &[100, 200, 300]);

    let res = avg_wave([&p1, &p2, &p3], &out);
    match res {
        Err(AvgWaveError::MismatchedLength) => (),
        other => panic!("expected MismatchedLength, got: {other:?}"),
    }
    assert!(!out.exists(), "output file must not exist after error");
}

// ============================================================================
// TRIANGULATE: Edge Cases & Robustness Tests
// ============================================================================

#[test]
fn test_avg_wave_absolute_silence() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("silence1.wav");
    let p2 = dir.path().join("silence2.wav");
    let p3 = dir.path().join("silence3.wav");
    let out = dir.path().join("silence_out.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let zeros = vec![0i16; 100];
    write_synthetic_wav_i16(&p1, spec, &zeros);
    write_synthetic_wav_i16(&p2, spec, &zeros);
    write_synthetic_wav_i16(&p3, spec, &zeros);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(res.is_ok());

    let mut reader = WavReader::open(&out).unwrap();
    let samples: Vec<i32> = reader.samples::<i32>().map(|s| s.unwrap()).collect();
    assert_eq!(samples.len(), 100);
    assert!(samples.iter().all(|&s| s == 0), "all samples must be strictly 0");
}

#[test]
fn test_avg_wave_empty_zero_sample_wavs_rejected() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("empty1.wav");
    let p2 = dir.path().join("empty2.wav");
    let p3 = dir.path().join("empty3.wav");
    let out = dir.path().join("empty_out.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let empty: [i16; 0] = [];
    write_synthetic_wav_i16(&p1, spec, &empty);
    write_synthetic_wav_i16(&p2, spec, &empty);
    write_synthetic_wav_i16(&p3, spec, &empty);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(matches!(res, Err(AvgWaveError::EmptyAudio)));
    assert!(!out.exists(), "output file must not exist after EmptyAudio rejection");
}

#[test]
fn test_avg_wave_quantization_extremes_24bit() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("ext1.wav");
    let p2 = dir.path().join("ext2.wav");
    let p3 = dir.path().join("ext3.wav");
    let out = dir.path().join("ext_out.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 24,
        sample_format: SampleFormat::Int,
    };

    // Exactly min and max 24-bit samples
    let s1 = [-8_388_608, 8_388_607];
    let s2 = [-8_388_608, 8_388_607];
    let s3 = [-8_388_608, 8_388_607];

    write_synthetic_wav_i32_24(&p1, spec, &s1);
    write_synthetic_wav_i32_24(&p2, spec, &s2);
    write_synthetic_wav_i32_24(&p3, spec, &s3);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(res.is_ok());

    let mut reader = WavReader::open(&out).unwrap();
    let samples: Vec<i32> = reader.samples::<i32>().map(|s| s.unwrap()).collect();
    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0], -8_388_608, "minimum 24-bit clamp");
    assert_eq!(samples[1], 8_388_607, "maximum 24-bit clamp");
}

#[test]
fn test_avg_wave_nonexistent_input_file_returns_read_error() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("exists1.wav");
    let p2 = dir.path().join("missing.wav");
    let p3 = dir.path().join("exists3.wav");
    let out = dir.path().join("should_not_exist.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    write_synthetic_wav_i16(&p1, spec, &[10, 20]);
    write_synthetic_wav_i16(&p3, spec, &[10, 20]);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(matches!(res, Err(AvgWaveError::InputReadError)));
    assert!(!out.exists(), "output file must not exist when an input is missing");
}

#[test]
fn test_avg_wave_corrupt_truncated_file_purges_output() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("valid1.wav");
    let p2 = dir.path().join("truncated.wav");
    let p3 = dir.path().join("valid3.wav");
    let out = dir.path().join("corrupt_out.wav");

    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    // Valid files have 500 samples
    let long_samples = vec![1000i16; 500];
    write_synthetic_wav_i16(&p1, spec, &long_samples);
    write_synthetic_wav_i16(&p3, spec, &long_samples);

    // Create a WAV with 500 samples, then truncate the file to cut off data mid-stream
    write_synthetic_wav_i16(&p2, spec, &long_samples);
    let original_size = fs::metadata(&p2).unwrap().len();
    let file = fs::OpenOptions::new().write(true).open(&p2).unwrap();
    file.set_len(original_size / 4).unwrap();
    drop(file);

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(res.is_err(), "should fail on truncated stream: {res:?}");
    assert!(!out.exists(), "output file must be cleaned up after mid-stream read failure");
}

#[test]
fn test_avg_wave_unsupported_format_rejected() {
    let dir = tempdir().unwrap();
    let p1 = dir.path().join("unsupported1.wav");
    let p2 = dir.path().join("unsupported2.wav");
    let p3 = dir.path().join("unsupported3.wav");
    let out = dir.path().join("unsupported_out.wav");

    // 8-bit WAV (unsupported)
    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 8,
        sample_format: SampleFormat::Int,
    };

    let mut writer1 = WavWriter::create(&p1, spec).unwrap();
    writer1.write_sample(100i8).unwrap();
    writer1.finalize().unwrap();

    let mut writer2 = WavWriter::create(&p2, spec).unwrap();
    writer2.write_sample(100i8).unwrap();
    writer2.finalize().unwrap();

    let mut writer3 = WavWriter::create(&p3, spec).unwrap();
    writer3.write_sample(100i8).unwrap();
    writer3.finalize().unwrap();

    let res = avg_wave([&p1, &p2, &p3], &out);
    assert!(matches!(res, Err(AvgWaveError::UnsupportedFormat)));
    assert!(!out.exists());
}

// ============================================================================
// encode_mp3_320k Tests
// ============================================================================

#[tokio::test]
async fn test_encode_mp3_320k_success() {
    let dir = tempdir().unwrap();
    let wav_path = dir.path().join("input.wav");
    let mp3_path = dir.path().join("output.mp3");

    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    // 1 second of stereo audio
    let mut samples = Vec::with_capacity(88200);
    for i in 0..44100 {
        let val = ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0).sin() * 16000.0) as i16;
        samples.push(val);
        samples.push(val);
    }
    write_synthetic_wav_i16(&wav_path, spec, &samples);

    let res = encode_mp3_320k(None, &wav_path, &mp3_path).await;
    assert!(res.is_ok(), "encode_mp3_320k should succeed: {res:?}");
    assert!(mp3_path.exists(), "mp3 output must exist");

    let metadata = fs::metadata(&mp3_path).unwrap();
    assert!(metadata.len() > 1000, "mp3 size should be substantial");

    // Verify magic bytes (either ID3 tag or 0xFF, 0xFB frame sync)
    let bytes = fs::read(&mp3_path).unwrap();
    let is_id3 = bytes.starts_with(b"ID3");
    let is_mp3_frame = bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0;
    assert!(
        is_id3 || is_mp3_frame,
        "encoded file must have valid MP3 header"
    );
}

#[tokio::test]
async fn test_encode_mp3_320k_nonexistent_input_fails() {
    let dir = tempdir().unwrap();
    let wav_path = dir.path().join("nonexistent.wav");
    let mp3_path = dir.path().join("output.mp3");

    let res = encode_mp3_320k(None, &wav_path, &mp3_path).await;
    assert!(matches!(res, Err(EncodeMp3Error::InputNotFound)));
    assert!(!mp3_path.exists(), "output mp3 must not exist on failure");
}
