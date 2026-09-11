use rand::rngs::OsRng;
use rand::RngCore;
use uuid::Uuid;

use crate::domain::ticketing::{QrSecret, ScanToken};
use crate::error::AppError;
use crate::ports::ticketing::{CredentialGenerator, GeneratedCredentials};

#[derive(Default, Clone)]
pub struct OsRandomCredentials;

impl OsRandomCredentials {
    pub fn new() -> Self {
        Self
    }
}

impl CredentialGenerator for OsRandomCredentials {
    fn generate(&self) -> Result<GeneratedCredentials, AppError> {
        let mut scan_bytes = [0u8; 32];
        let mut qr_bytes = [0u8; 16];

        OsRng.fill_bytes(&mut scan_bytes);
        OsRng.fill_bytes(&mut qr_bytes);

        let mut scan_hex = String::with_capacity(67);
        scan_hex.push_str("st_");
        for b in &scan_bytes {
            use std::fmt::Write;
            write!(&mut scan_hex, "{:02x}", b).map_err(|e| AppError::Internal(e.to_string()))?;
        }

        let mut qr_hex = String::with_capacity(32);
        for b in &qr_bytes {
            use std::fmt::Write;
            write!(&mut qr_hex, "{:02x}", b).map_err(|e| AppError::Internal(e.to_string()))?;
        }

        let scan_token = ScanToken::new(scan_hex)?;
        let qr_secret = QrSecret::new(qr_hex)?;
        let id = Uuid::new_v4().to_string();

        Ok(GeneratedCredentials {
            id,
            scan_token,
            qr_secret,
        })
    }
}
