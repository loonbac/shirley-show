use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::process_vocal_separator::ProcessVocalSeparator;
use shirley_show::infra::vocal::process_vocal_separator_config::ProcessVocalSeparatorConfig;
use shirley_show::ports::vocal_separator::{VocalSeparationRequest, VocalSeparator};

#[tokio::test]
#[ignore = "manual real GPU test"]
async fn test_real_gpu_chorus_separation_with_karaoke_separator() {
    let source_path = PathBuf::from("/tmp/shirley_test_input_sine.wav");
    let output_path = PathBuf::from("/tmp/shirley_test_output_chorus.mp3");
    let work_root = PathBuf::from("/tmp/shirley_test_vocal_work");

    // Clean any remnants before starting
    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_dir_all(&work_root);
    std::fs::create_dir_all(&work_root).unwrap();

    // 1. Generate a short 3-second test WAV with ffmpeg (stereo, 44100Hz, 16-bit)
    let gen_status = Command::new("ffmpeg")
        .args([
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=3",
            "-ac",
            "2",
            "-ar",
            "44100",
            source_path.to_str().unwrap(),
            "-y",
        ])
        .status()
        .expect("failed to generate test audio with ffmpeg");
    assert!(gen_status.success(), "ffmpeg test audio generation failed");

    // 2. Configure ProcessVocalSeparator with real binaries from PATH
    let gpu = Arc::new(Semaphore::new(1));
    let config = ProcessVocalSeparatorConfig::from_env(work_root.clone(), gpu);
    let separator = ProcessVocalSeparator::new(config);

    let req = VocalSeparationRequest {
        source_path: source_path.clone(),
        output_path: output_path.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(600),
    };

    println!("Starting real GPU chorus separation with karaoke-separator...");
    let start = std::time::Instant::now();
    let result = separator.separate(req).await;
    let elapsed = start.elapsed();
    println!(
        "Separation finished in {:.2}s. Result: {:?}",
        elapsed.as_secs_f64(),
        result
    );

    assert!(
        result.is_ok(),
        "Real GPU chorus separation failed: {:?}",
        result.err()
    );
    let separated = result.unwrap();
    println!(
        "Separated audio: duration={}s, size={} bytes",
        separated.duration_secs, separated.size_bytes
    );

    // 3. Verify MP3 output file with ffprobe
    assert!(output_path.exists(), "Output MP3 must exist on disk");
    assert!(
        output_path.metadata().unwrap().len() > 0,
        "Output MP3 must not be empty"
    );

    let probe_out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name,bit_rate:format=duration",
            "-of",
            "json",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("ffprobe execution failed");

    assert!(probe_out.status.success(), "ffprobe failed on output MP3");
    let probe_str = String::from_utf8_lossy(&probe_out.stdout);
    println!("ffprobe verification output:\n{}", probe_str);

    let parsed: serde_json::Value =
        serde_json::from_slice(&probe_out.stdout).expect("parse ffprobe json");
    let codec = parsed["streams"][0]["codec_name"].as_str().unwrap();
    assert_eq!(codec, "mp3", "Codec must be mp3");
    let bitrate_str = parsed["streams"][0]["bit_rate"].as_str().unwrap_or("0");
    let bitrate: u64 = bitrate_str.parse().unwrap_or(0);
    assert!(
        (310000..=330000).contains(&bitrate),
        "Bitrate must be ~320k, got: {bitrate}"
    );

    // 4. Clean up everything
    let _ = std::fs::remove_file(&source_path);
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_dir_all(&work_root);
    println!("All temporary files cleaned up successfully.");
}
