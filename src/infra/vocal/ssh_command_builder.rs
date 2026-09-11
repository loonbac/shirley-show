use std::path::{Path, PathBuf};
use tokio::process::Command;

pub const REMOTE_PIPELINE_SCRIPT: &str = r#"set -eu

MODE="$1"
DIR="$2"
INPUT_NAME="$3"
INPUT="$DIR/$INPUT_NAME"
OUTPUT="$DIR/output.mp3"

find_stem() {
    model_dir="$1"
    non_vocal=""
    vocal=""
    count=0
    for f in "$model_dir"/*; do
        [ -f "$f" ] || continue
        case "$f" in
            *.wav|*.flac) ;;
            *) continue ;;
        esac
        count=$((count + 1))
        fname=$(basename "$f")
        case "$(echo "$fname" | tr '[:upper:]' '[:lower:]')" in
            *vocal*) vocal="$f" ;;
            *) non_vocal="$f" ;;
        esac
    done
    if [ "$count" -ne 2 ] || [ -z "$vocal" ] || [ -z "$non_vocal" ]; then
        echo "Error resolving stem in $model_dir" >&2
        exit 1
    fi
    case "$non_vocal" in
        *.flac)
            wav_path="${non_vocal%.flac}.wav"
            ffmpeg -y -v error -i "$non_vocal" "$wav_path"
            echo "$wav_path"
            ;;
        *)
            echo "$non_vocal"
            ;;
    esac
}

if [ "$MODE" = "chorus" ]; then
    mkdir -p "$DIR/m1" "$DIR/m2" "$DIR/m3"
    karaoke-separator "gabox-invertido" "$INPUT" "$DIR/m1"
    karaoke-separator "frazer-becruily" "$INPUT" "$DIR/m2"
    karaoke-separator "anvuew" "$INPUT" "$DIR/m3"

    s1=$(find_stem "$DIR/m1")
    s2=$(find_stem "$DIR/m2")
    s3=$(find_stem "$DIR/m3")

    if command -v AVG_WAVE >/dev/null 2>&1; then
        AVG_WAVE "$s1" "$s2" "$s3" "$DIR/avg.wav"
        avg_file="$DIR/avg.wav"
    else
        ffmpeg -y -v error -i "$s1" -i "$s2" -i "$s3" -filter_complex "amix=inputs=3:duration=first:dropout_transition=0:weights=1 1 1,volume=1" "$DIR/avg.wav"
        avg_file="$DIR/avg.wav"
    fi

    ffmpeg -y -v error -i "$avg_file" -codec:a libmp3lame -b:a 320k -f mp3 "$OUTPUT"

elif [ "$MODE" = "instrumental" ]; then
    mkdir -p "$DIR/m_inst"
    karaoke-separator "polarformer" "$INPUT" "$DIR/m_inst"
    s_inst=$(find_stem "$DIR/m_inst")
    ffmpeg -y -v error -i "$s_inst" -codec:a libmp3lame -b:a 320k -f mp3 "$OUTPUT"
else
    echo "Unknown mode: $MODE" >&2
    exit 1
fi
"#;

#[derive(Clone, Debug)]
pub struct SshCommandBuilder {
    pub ssh_bin: PathBuf,
    pub scp_bin: PathBuf,
    pub user: String,
    pub host: String,
    pub key_path: PathBuf,
    pub port: u16,
}

impl SshCommandBuilder {
    pub fn new(
        ssh_bin: PathBuf,
        scp_bin: PathBuf,
        user: String,
        host: String,
        key_path: PathBuf,
        port: u16,
    ) -> Self {
        Self {
            ssh_bin,
            scp_bin,
            user,
            host,
            key_path,
            port,
        }
    }

    pub fn target(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }

    fn base_ssh_args(&self) -> Vec<String> {
        vec![
            "-p".to_string(),
            self.port.to_string(),
            "-i".to_string(),
            self.key_path.to_string_lossy().into_owned(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "IdentitiesOnly=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=accept-new".to_string(),
        ]
    }

    fn base_scp_args(&self) -> Vec<String> {
        vec![
            "-P".to_string(),
            self.port.to_string(),
            "-i".to_string(),
            self.key_path.to_string_lossy().into_owned(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "IdentitiesOnly=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=accept-new".to_string(),
        ]
    }

    pub fn build_probe(&self, connect_timeout_secs: u32) -> Command {
        let mut cmd = Command::new(&self.ssh_bin);
        cmd.args(self.base_ssh_args());
        cmd.args([
            "-o",
            &format!("ConnectTimeout={connect_timeout_secs}"),
            &self.target(),
            "true",
        ]);
        cmd
    }

    pub fn build_mkdir(&self, remote_dir: &str) -> Command {
        let mut cmd = Command::new(&self.ssh_bin);
        cmd.args(self.base_ssh_args());
        cmd.args([&self.target(), "mkdir", "-p", "-m", "700", remote_dir]);
        cmd
    }

    pub fn build_upload(&self, local_source: &Path, remote_path: &str) -> Command {
        let mut cmd = Command::new(&self.scp_bin);
        cmd.args(self.base_scp_args());
        cmd.arg(local_source);
        cmd.arg(format!("{}:{}", self.target(), remote_path));
        cmd
    }

    pub fn build_pipeline(
        &self,
        mode: &str,
        remote_dir: &str,
        input_filename: &str,
    ) -> (Command, &'static str) {
        let mut cmd = Command::new(&self.ssh_bin);
        cmd.args(self.base_ssh_args());
        cmd.args([
            &self.target(),
            "sh",
            "-s",
            "--",
            mode,
            remote_dir,
            input_filename,
        ]);
        (cmd, REMOTE_PIPELINE_SCRIPT)
    }

    pub fn build_download(&self, remote_path: &str, local_dest: &Path) -> Command {
        let mut cmd = Command::new(&self.scp_bin);
        cmd.args(self.base_scp_args());
        cmd.arg(format!("{}:{}", self.target(), remote_path));
        cmd.arg(local_dest);
        cmd
    }

    pub fn build_cleanup(&self, remote_dir: &str) -> Command {
        let mut cmd = Command::new(&self.ssh_bin);
        cmd.args(self.base_ssh_args());
        cmd.args([&self.target(), "rm", "-rf", remote_dir]);
        cmd
    }
}
