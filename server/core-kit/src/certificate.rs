use anyhow::Result;
use chrono::Datelike;
use quinn::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use rcgen::{CertificateParams, KeyPair, SanType};
use std::{fs, path::Path};
use tokio::io::AsyncWriteExt;

use crate::{
    common::{read_cert, read_key},
    inner_const::APP_HOME,
};

pub struct CertificateLoader;

impl CertificateLoader {
    pub async fn load_from_path(
        cert_path: Option<String>,
        key_path: Option<String>,
    ) -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>)> {
        // 尝试从指定路径加载
        if let (Some(cert), Some(key)) = (cert_path, key_path) {
            let cert_der = read_cert(Path::new(&cert)).await?;
            let key_der = read_key(Path::new(&key)).await?;
            return Ok((cert_der, key_der));
        }

        // 从应用数据目录加载
        let cert_dir = APP_HOME.data_dir().join("cert");
        tracing::info!("证书目录: {}", cert_dir.display());
        fs::create_dir_all(&cert_dir)?; // 确保目录存在

        let cert_pem_path = cert_dir.join("cert.pem");
        let key_pem_path = cert_dir.join("key.pem");

        if cert_pem_path.exists() && key_pem_path.exists() {
            let cert_der = read_cert(&cert_pem_path).await?;
            let key_der = read_key(&key_pem_path).await?;
            Ok((cert_der, key_der))
        } else {
            // 生成并保存新证书
            tracing::info!("证书不存在，生成新的自签名证书");
            let (cert_der, key_der) = Self::generate_certificate()?;

            Self::save_certificate(&cert_der, &key_der, &cert_pem_path, &key_pem_path).await?;

            Ok((cert_der, key_der))
        }
    }

    pub fn generate_certificate() -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>)>
    {
        let mut params = CertificateParams::new(vec!["touchpad.internal".to_string()])?;

        // 设置有效期
        let now = chrono::Local::now();
        params.not_before = rcgen::date_time_ymd(now.year(), now.month() as u8, now.day() as u8);
        params.not_after = rcgen::date_time_ymd(now.year() + 1, now.month() as u8, now.day() as u8);

        params
            .subject_alt_names
            .push(SanType::IpAddress("127.0.0.1".parse().unwrap()));

        // 生成密钥对和证书
        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;

        let cert_der = CertificateDer::from(cert.der().clone().into_owned());
        let key_der = PrivatePkcs8KeyDer::from(key_pair.serialize_der());

        Ok((cert_der, key_der))
    }

    /// 保存证书到文件
    async fn save_certificate(
        cert_der: &CertificateDer<'_>,
        key_der: &PrivatePkcs8KeyDer<'_>,
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<()> {
        // 将 DER 转换为 PEM 格式
        let cert_pem = pem::Pem::new("CERTIFICATE", cert_der.as_ref());
        let key_pem = pem::Pem::new("PRIVATE KEY", key_der.secret_pkcs8_der());

        // 异步写入
        let mut cert_file = tokio::fs::File::create(cert_path).await?;
        let mut key_file = tokio::fs::File::create(key_path).await?;

        cert_file
            .write_all(pem::encode(&cert_pem).as_bytes())
            .await?;
        key_file.write_all(pem::encode(&key_pem).as_bytes()).await?;

        tracing::info!("证书已保存到: {:?}", cert_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_certificate() {
        let (cert_der, key_der) = CertificateLoader::generate_certificate().unwrap();

        assert!(!cert_der.is_empty());
        println!("Certificate DER: {:?}", cert_der);
        println!("Private Key DER: {:?}", key_der.secret_pkcs8_der());
    }
}
