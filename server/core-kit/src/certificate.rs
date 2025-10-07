use anyhow::Result;
use rcgen::{Certificate, CertifiedKey, KeyPair, generate_simple_self_signed};
use std::net::IpAddr;

use crate::inner_const;

pub struct CertificateGenerator;

impl CertificateGenerator {
    pub fn generate_certificate(
        domain: &str,
        ip_address: Option<IpAddr>,
    ) -> Result<(Certificate, KeyPair)> {
        let ip_addrress = if let Some(ip) = ip_address {
            ip
        } else {
            inner_const::LOCALHOST_V4
        };
        let subject_alt_names = vec![domain.to_string(), ip_addrress.to_string()];

        let CertifiedKey { cert, signing_key } =
            generate_simple_self_signed(subject_alt_names).unwrap();

        Ok((cert, signing_key))
    }
}
