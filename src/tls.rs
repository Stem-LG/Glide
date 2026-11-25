use crate::config;
use anyhow::{Context, Result};
use std::{fs, io::Write, net::IpAddr, os::unix::fs::OpenOptionsExt, sync::Arc};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    },
};

/// Creates a self-signed certificate naming this machine's LAN address.
fn generate(ip: IpAddr) -> Result<(String, String)> {
    let mut params = rcgen::CertificateParams::new(vec!["localhost".into(), ip.to_string()])?;
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Glide");
    params.not_before = rcgen::date_time_ymd(2025, 1, 1);
    params.not_after = rcgen::date_time_ymd(2045, 1, 1);
    let key = rcgen::KeyPair::generate()?;
    Ok((params.self_signed(&key)?.pem(), key.serialize_pem()))
}

/// Loads the saved certificate, replacing it only when the LAN address has
/// changed so a phone's accepted exception survives restarts.
pub fn acceptor(ip: IpAddr) -> Result<TlsAcceptor> {
    let dir = config::directory();
    let (cert_path, key_path, host_path) = (
        dir.join("tls-cert.pem"),
        dir.join("tls-key.pem"),
        dir.join("tls-host"),
    );
    let saved = (|| {
        if fs::read_to_string(&host_path).ok()? != ip.to_string() {
            return None;
        }
        Some((
            fs::read_to_string(&cert_path).ok()?,
            fs::read_to_string(&key_path).ok()?,
        ))
    })();
    let (cert, key) = match saved {
        Some(pair) => pair,
        None => {
            let (cert, key) = generate(ip)?;
            fs::create_dir_all(&dir)?;
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&key_path)?
                .write_all(key.as_bytes())?;
            fs::write(&cert_path, &cert)?;
            fs::write(&host_path, ip.to_string())?;
            (cert, key)
        }
    };
    let certs = CertificateDer::pem_slice_iter(cert.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .context("Unreadable TLS certificate")?;
    let key = PrivateKeyDer::from_pem_slice(key.as_bytes()).context("Unreadable TLS key")?;
    let mut tls = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_no_client_auth()
    .with_single_cert(certs, key)?;
    // WebSocket upgrades are served over HTTP/1.1.
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(TlsAcceptor::from(Arc::new(tls)))
}
