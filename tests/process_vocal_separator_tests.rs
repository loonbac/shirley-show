use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::Semaphore;

use shirley_show::domain::vocal_separation_mode::VocalSeparationMode;
use shirley_show::infra::vocal::process_vocal_separator_config::ProcessVocalSeparatorConfig;
use shirley_show::infra::vocal::ProcessVocalSeparator;
use shirley_show::ports::vocal_separator::{
    VocalSeparationError, VocalSeparationRequest, VocalSeparator,
};

fn create_executable_script(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    {
        let mut file = File::create(&path).expect("failed to create script");
        file.write_all(content.as_bytes())
            .expect("failed to write script");
        file.sync_all().expect("failed to sync script");
    }
    let mut perms = fs::metadata(&path)
        .expect("failed to get metadata")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).expect("failed to set script executable");
    path
}

/// Creates a minimal valid WAV file with 16-bit PCM silence using hound.
fn create_test_wav(path: &Path, duration_samples: u32, sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("failed to create wav");
    for _ in 0..(duration_samples * 2) {
        writer.write_sample(0i16).expect("failed to write sample");
    }
    writer.finalize().expect("failed to finalize wav");
}

struct TestSeparatorEnv {
    pub temp_dir: TempDir,
    pub bin_dir: PathBuf,
    pub work_root: PathBuf,
    pub staging_dir: PathBuf,
    pub log_file: PathBuf,
    pub sample_wav: PathBuf,
    pub checkpoint_file: PathBuf,
    pub config_file: PathBuf,
    pub gpu: Arc<Semaphore>,
}

impl TestSeparatorEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("failed to create tempdir");
        let bin_dir = temp_dir.path().join("bin");
        let work_root = temp_dir.path().join("vocal-work");
        let staging_dir = temp_dir.path().join("staging");
        fs::create_dir_all(&bin_dir).expect("failed to create bin dir");
        fs::create_dir_all(&work_root).expect("failed to create work root");
        fs::create_dir_all(&staging_dir).expect("failed to create staging dir");

        let log_file = temp_dir.path().join("args.log");
        let sample_wav = temp_dir.path().join("sample.wav");
        create_test_wav(&sample_wav, 4410, 44100);

        let checkpoint_file = temp_dir
            .path()
            .join("model_bs_polarformer_float16.ckpt");
        File::create(&checkpoint_file).expect("failed to create fake ckpt");

        let config_file = temp_dir
            .path()
            .join("model_bs_polarformer_float16.yaml");
        File::create(&config_file).expect("failed to create fake yaml");

        let gpu = Arc::new(Semaphore::new(1));

        Self {
            temp_dir,
            bin_dir,
            work_root,
            staging_dir,
            log_file,
            sample_wav,
            checkpoint_file,
            config_file,
            gpu,
        }
    }

    fn setup_default_stubs(&self) -> (PathBuf, PathBuf, PathBuf) {
        // Stub for ffprobe: by default returns duration 180.0, audio stream, and 320000 bitrate
        let ffprobe_script = r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  echo "ffprobe version 7.0"
  exit 0
fi
echo '{"streams":[{"codec_type":"audio","codec_name":"mp3"}],"format":{"duration":"180.0","bit_rate":"320000"}}'
exit 0
"#;
        let ffprobe_bin = create_executable_script(&self.bin_dir, "ffprobe", ffprobe_script);

        // Stub for ffmpeg: creates a valid MP3 file on output
        let ffmpeg_script = r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  echo "ffmpeg version 7.0"
  exit 0
fi
out=""
while [ "$#" -gt 0 ]; do
  out="$1"
  shift
done
# Write minimal valid MP3
printf "ID3\x03\x00\x00\x00\x00\x00\x00\xff\xfb\x90\x64" > "$out"
# Pad with zeros
dd if=/dev/zero bs=1024 count=4 >> "$out" 2>/dev/null
exit 0
"#;
        let ffmpeg_bin = create_executable_script(&self.bin_dir, "ffmpeg", ffmpeg_script);

        // Stub for karaoke-separator: logs args and copies sample WAV into model_dir
        let sample_wav_str = self.sample_wav.to_str().unwrap();
        let log_file_str = self.log_file.to_str().unwrap();
        let separator_script = format!(
            r#"#!/bin/sh
model="$1"
input="$2"
outdir="$3"
mkdir -p "$outdir"
printf '%s\n' "$@" >> "{log_file_str}"
echo "---END---" >> "{log_file_str}"

cp "{sample_wav_str}" "$outdir/track_${{model}}_vocals.wav"
cp "{sample_wav_str}" "$outdir/track_${{model}}_karaoke.wav"
exit 0
"#
        );
        let vocal_sep_bin =
            create_executable_script(&self.bin_dir, "karaoke-separator", &separator_script);

        (vocal_sep_bin, ffprobe_bin, ffmpeg_bin)
    }

    fn make_config(
        &self,
        vocal_sep_bin: PathBuf,
        ffprobe_bin: PathBuf,
        ffmpeg_bin: PathBuf,
    ) -> ProcessVocalSeparatorConfig {
        ProcessVocalSeparatorConfig {
            vocal_sep_bin: Some(vocal_sep_bin),
            ffprobe_bin,
            ffmpeg_bin,
            polarformer_checkpoint: Some(self.checkpoint_file.clone()),
            polarformer_config: Some(self.config_file.clone()),
            work_root: self.work_root.clone(),
            gpu: self.gpu.clone(),
        }
    }
}

#[tokio::test]
async fn test_ffprobe_duration_rejection_over_600s_before_gpu() {
    let env = TestSeparatorEnv::new();
    let (vocal_sep_bin, _, ffmpeg_bin) = env.setup_default_stubs();

    // Custom ffprobe stub that returns duration 601.0
    let ffprobe_script = r#"#!/bin/sh
echo '{"streams":[{"codec_type":"audio","codec_name":"mp3"}],"format":{"duration":"601.0","bit_rate":"320000"}}'
exit 0
"#;
    let ffprobe_bin = create_executable_script(&env.bin_dir, "ffprobe_over", ffprobe_script);
    let config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    let separator = ProcessVocalSeparator::new(config);

    let output_path = env.staging_dir.join("test_out.part");
    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: output_path.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(req).await;
    match result {
        Err(VocalSeparationError::DurationExceeded) => {}
        other => panic!("expected DurationExceeded, got: {:?}", other),
    }

    // GPU permit was never consumed
    assert_eq!(env.gpu.available_permits(), 1);
    // Separator was never called
    assert!(!env.log_file.exists());
    // No output created
    assert!(!output_path.exists());
}

#[tokio::test]
async fn test_ffprobe_duration_invalid_before_gpu() {
    let env = TestSeparatorEnv::new();
    let (vocal_sep_bin, _, ffmpeg_bin) = env.setup_default_stubs();

    // ffprobe fails or outputs unparseable json
    let ffprobe_script = "#!/bin/sh\nexit 1\n";
    let ffprobe_bin = create_executable_script(&env.bin_dir, "ffprobe_fail", ffprobe_script);
    let config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    let separator = ProcessVocalSeparator::new(config);

    let output_path = env.staging_dir.join("test_out.part");
    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path,
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(req).await;
    match result {
        Err(VocalSeparationError::InvalidAudioDuration) => {}
        other => panic!("expected InvalidAudioDuration, got: {:?}", other),
    }

    assert_eq!(env.gpu.available_permits(), 1);
    assert!(!env.log_file.exists());
}

#[tokio::test]
async fn test_polarformer_checkpoint_missing_returns_instrumental_unavailable_before_gpu() {
    let env = TestSeparatorEnv::new();
    let (vocal_sep_bin, ffprobe_bin, ffmpeg_bin) = env.setup_default_stubs();

    // Point polarformer to nonexistent files
    let mut config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    config.polarformer_checkpoint = Some(env.temp_dir.path().join("missing_checkpoint.ckpt"));
    let separator = ProcessVocalSeparator::new(config);

    let output_path = env.staging_dir.join("test_out.part");
    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path,
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(req).await;
    match result {
        Err(VocalSeparationError::InstrumentalUnavailable) => {}
        other => panic!("expected InstrumentalUnavailable, got: {:?}", other),
    }

    // GPU permit was never taken
    assert_eq!(env.gpu.available_permits(), 1);
    // Separator was not invoked
    assert!(!env.log_file.exists());
}

#[tokio::test]
async fn test_coros_sequential_models_and_resolve_stem_excluding_vocal() {
    let env = TestSeparatorEnv::new();
    let (vocal_sep_bin, ffprobe_bin, ffmpeg_bin) = env.setup_default_stubs();
    let config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    let separator = ProcessVocalSeparator::new(config);

    let output_path = env.staging_dir.join("coros_out.part");
    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: output_path.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(req).await;
    assert!(result.is_ok(), "separation failed: {:?}", result.err());
    let separated = result.unwrap();
    assert_eq!(separated.duration_secs, 180);
    assert!(separated.size_bytes > 0);
    assert!(output_path.is_file());

    // Check log file: models were called in exact order
    let log_content = fs::read_to_string(&env.log_file).expect("read log");
    let invocations: Vec<&str> = log_content.split("---END---\n").collect();
    assert!(invocations.len() >= 3);
    assert!(invocations[0].starts_with("gabox-invertido\n"));
    assert!(invocations[1].starts_with("frazer-becruily\n"));
    assert!(invocations[2].starts_with("anvuew\n"));

    // GPU permit released
    assert_eq!(env.gpu.available_permits(), 1);

    // Vocal work directory was purged
    let work_entries: Vec<_> = fs::read_dir(&env.work_root)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(work_entries.len(), 0, "work_root should be empty after run");
}

#[tokio::test]
async fn test_instrumental_polarformer_success() {
    let env = TestSeparatorEnv::new();
    let (vocal_sep_bin, ffprobe_bin, ffmpeg_bin) = env.setup_default_stubs();
    let config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    let separator = ProcessVocalSeparator::new(config);

    let output_path = env.staging_dir.join("polarformer_out.part");
    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: output_path.clone(),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(req).await;
    assert!(result.is_ok(), "polarformer failed: {:?}", result.err());
    let separated = result.unwrap();
    assert_eq!(separated.duration_secs, 180);
    assert!(output_path.is_file());

    let log_content = fs::read_to_string(&env.log_file).expect("read log");
    assert!(log_content.starts_with("polarformer\n"));

    // GPU permit released
    assert_eq!(env.gpu.available_permits(), 1);
}

#[tokio::test]
async fn test_cancellation_kills_child_purges_workdir_and_holds_permit() {
    let env = TestSeparatorEnv::new();
    let (_, ffprobe_bin, ffmpeg_bin) = env.setup_default_stubs();

    // Blocking separator stub that sleeps 30 seconds
    let blocking_script = r#"#!/bin/sh
sleep 30
exit 0
"#;
    let vocal_sep_bin =
        create_executable_script(&env.bin_dir, "blocking_sep", blocking_script);

    let config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    let separator = Arc::new(ProcessVocalSeparator::new(config));

    let output_path = env.staging_dir.join("cancelled.part");
    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: output_path.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let sep_clone = separator.clone();
    let gpu_clone = env.gpu.clone();

    // Spawn separation task
    let handle = tokio::spawn(async move {
        sep_clone.separate(req).await
    });

    // Wait until child is spawned and permit is taken
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(gpu_clone.available_permits(), 0, "GPU permit should be held by running task");

    // Cancel the future by aborting the join handle
    handle.abort();

    // Give background drop task time to kill-and-wait and cleanup
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Permit must be released back to 1 AFTER kill and wait
    assert_eq!(gpu_clone.available_permits(), 1, "Permit should be restored to 1");

    // Partial output must be purged
    assert!(!output_path.exists());

    // Work dir must be purged
    let work_entries: Vec<_> = fs::read_dir(&env.work_root)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(work_entries.len(), 0, "work_root must be cleaned after cancellation");
}

// ---------------------------------------------------------------------------
// TRIANGULATION TESTS (Task 3)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_coros_second_model_fails_cleans_up_and_returns_error() {
    let env = TestSeparatorEnv::new();
    let (_, ffprobe_bin, ffmpeg_bin) = env.setup_default_stubs();

    // Stub where the second model (frazer-becruily) fails with exit code 1
    let sample_wav_str = env.sample_wav.to_str().unwrap();
    let separator_script = format!(
        r#"#!/bin/sh
model="$1"
outdir="$3"
mkdir -p "$outdir"

if [ "$model" = "frazer-becruily" ]; then
  exit 1
fi

cp "{sample_wav_str}" "$outdir/track_${{model}}_vocals.wav"
cp "{sample_wav_str}" "$outdir/track_${{model}}_karaoke.wav"
exit 0
"#
    );
    let vocal_sep_bin =
        create_executable_script(&env.bin_dir, "separator_fail_step2", &separator_script);

    let config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    let separator = ProcessVocalSeparator::new(config);

    let output_path = env.staging_dir.join("fail_coros.part");
    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: output_path.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(req).await;
    match result {
        Err(VocalSeparationError::SeparationFailed) => {}
        other => panic!("expected SeparationFailed, got: {:?}", other),
    }

    // Permit released
    assert_eq!(env.gpu.available_permits(), 1);

    // Partial output does not exist
    assert!(!output_path.exists());

    // Workdir was completely purged (including outputs of model 1!)
    let work_entries: Vec<_> = fs::read_dir(&env.work_root)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(work_entries.len(), 0, "work_root must be purged after intermediate failure");
}

#[tokio::test]
async fn test_vocal_sep_bin_nonexistent_or_not_executable_returns_separator_not_configured() {
    let env = TestSeparatorEnv::new();
    let (_, ffprobe_bin, ffmpeg_bin) = env.setup_default_stubs();

    // Point binary to nonexistent path
    let mut config = env.make_config(
        env.bin_dir.join("nonexistent_binary"),
        ffprobe_bin.clone(),
        ffmpeg_bin.clone(),
    );
    let separator = ProcessVocalSeparator::new(config.clone());

    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: env.staging_dir.join("sep_nonexistent.part"),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(req).await;
    match result {
        Err(VocalSeparationError::SeparatorNotConfigured) => {}
        other => panic!("expected SeparatorNotConfigured, got: {:?}", other),
    }

    // Non-executable binary
    let non_exec = env.bin_dir.join("non_exec_binary");
    File::create(&non_exec).expect("create file");
    let mut perms = fs::metadata(&non_exec).unwrap().permissions();
    perms.set_mode(0o644); // NOT executable
    fs::set_permissions(&non_exec, perms).unwrap();

    config.vocal_sep_bin = Some(non_exec);
    let separator = ProcessVocalSeparator::new(config);

    let req2 = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: env.staging_dir.join("sep_non_exec.part"),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result2 = separator.separate(req2).await;
    match result2 {
        Err(VocalSeparationError::SeparatorNotConfigured) => {}
        other => panic!("expected SeparatorNotConfigured, got: {:?}", other),
    }

    assert_eq!(env.gpu.available_permits(), 1);
}

#[tokio::test]
async fn test_post_verify_wrong_bitrate_or_codec_fails_with_transcode_failed() {
    let env = TestSeparatorEnv::new();
    let (vocal_sep_bin, _, ffmpeg_bin) = env.setup_default_stubs();

    // ffprobe returns wrong bitrate (128000 instead of 320000) on post-verify
    // To distinguish preflight vs post-verify: preflight inspects source_path (sample.wav),
    // post-verify inspects output_path (.part).
    let ffprobe_script = r#"#!/bin/sh
file="$0"
while [ "$#" -gt 0 ]; do
  file="$1"
  shift
done

case "$file" in
  *.part)
    # Output post-verify check: return 128 kbps instead of 320 kbps!
    echo '{"streams":[{"codec_name":"mp3"}],"format":{"duration":"180.0","bit_rate":"128000"}}'
    ;;
  *)
    # Source preflight check: valid
    echo '{"streams":[{"codec_type":"audio","codec_name":"wav"}],"format":{"duration":"180.0"}}'
    ;;
esac
exit 0
"#;
    let ffprobe_bin = create_executable_script(&env.bin_dir, "ffprobe_bad_bitrate", ffprobe_script);
    let config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    let separator = ProcessVocalSeparator::new(config);

    let output_path = env.staging_dir.join("bad_bitrate.part");
    let req = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: output_path.clone(),
        mode: VocalSeparationMode::Coros,
        timeout: Duration::from_secs(30),
    };

    let result = separator.separate(req).await;
    match result {
        Err(VocalSeparationError::TranscodeFailed) => {}
        other => panic!("expected TranscodeFailed, got: {:?}", other),
    }

    assert_eq!(env.gpu.available_permits(), 1);
    assert!(!output_path.exists());
}

#[tokio::test]
async fn test_two_concurrent_separations_serialized_without_vram_overlap() {
    let env = TestSeparatorEnv::new();
    let (_, ffprobe_bin, ffmpeg_bin) = env.setup_default_stubs();

    let counter_file = env.temp_dir.path().join("concurrency.log");
    let counter_str = counter_file.to_str().unwrap();
    let sample_wav_str = env.sample_wav.to_str().unwrap();

    // Stub that increments active workers count in counter_file, sleeps, then decrements
    let concurrency_script = format!(
        r#"#!/bin/sh
model="$1"
outdir="$3"
mkdir -p "$outdir"

# Log start
echo "START" >> "{counter_str}"
sleep 0.15
echo "END" >> "{counter_str}"

cp "{sample_wav_str}" "$outdir/track_${{model}}_vocals.wav"
cp "{sample_wav_str}" "$outdir/track_${{model}}_karaoke.wav"
exit 0
"#
    );
    let vocal_sep_bin =
        create_executable_script(&env.bin_dir, "conc_sep", &concurrency_script);

    let config = env.make_config(vocal_sep_bin, ffprobe_bin, ffmpeg_bin);
    let separator = Arc::new(ProcessVocalSeparator::new(config));

    let sep1 = separator.clone();
    let sep2 = separator.clone();

    let out1 = env.staging_dir.join("conc1.part");
    let out2 = env.staging_dir.join("conc2.part");

    let req1 = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: out1.clone(),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(30),
    };
    let req2 = VocalSeparationRequest {
        source_path: env.sample_wav.clone(),
        output_path: out2.clone(),
        mode: VocalSeparationMode::Instrumental,
        timeout: Duration::from_secs(30),
    };

    let (res1, res2) = tokio::join!(
        tokio::spawn(async move { sep1.separate(req1).await }),
        tokio::spawn(async move { sep2.separate(req2).await }),
    );

    assert!(res1.unwrap().is_ok());
    assert!(res2.unwrap().is_ok());

    assert!(out1.is_file());
    assert!(out2.is_file());

    // Check concurrency log: lines must be START, END, START, END (never START, START)
    let log_lines = fs::read_to_string(&counter_file).expect("read concurrency log");
    let lines: Vec<&str> = log_lines.lines().collect();
    assert_eq!(lines, vec!["START", "END", "START", "END"]);
}
