use anyhow::Result;
use quinn::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, pem::PemObject};
use std::path::Path;
use tracing::info;

pub async fn read_cert(cert_file: &Path) -> Result<CertificateDer<'static>> {
    let cert_file = if cert_file.is_relative() {
        cert_file.canonicalize()?
    } else {
        cert_file.to_path_buf()
    };
    info!("Certificate key file found at {:?}", cert_file);
    if !cert_file.exists() {
        return Err(anyhow::anyhow!("certificate key file not found"));
    }

    info!("Certificate file loaded: {}", cert_file.display());
    let cert = CertificateDer::from_pem_file(cert_file)?; // 这里 move 进 pki-types，内部用 Cow<'static, [u8]>
    Ok(cert)
}

pub async fn read_key(key_file: &Path) -> Result<PrivatePkcs8KeyDer<'static>> {
    // 1. 先把文件整个读成 Vec<u8>（owned）
    let key_der_path = if key_file.is_relative() {
        key_file.canonicalize()?
    } else {
        key_file.to_path_buf()
    };
    info!("Private key file found at {:?}", key_der_path);
    if !key_der_path.exists() {
        return Err(anyhow::anyhow!("Private key file not found"));
    }
    let key_der = PrivatePkcs8KeyDer::from_pem_file(key_der_path)?;
    Ok(key_der)
}
