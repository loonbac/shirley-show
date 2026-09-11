pub mod avg_wave;
pub mod encode_mp3;

pub use avg_wave::{avg_wave, AvgWaveError};
pub use encode_mp3::{encode_mp3_320k, EncodeMp3Error};
