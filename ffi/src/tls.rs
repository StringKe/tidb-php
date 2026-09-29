use std::path::Path;
use std::sync::Arc;

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};

use crate::dsn::{Config, SslMode};
use crate::error::{Error, Result, client};

fn tls_error(message: impl Into<String>) -> Error {
    Error { sqlstate: *b"08001", code: client::SSL_CONNECTION, message: message.into(), fatal: true }
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

pub fn server_name(host: &str) -> Result<ServerName<'static>> {
    ServerName::try_from(host.to_string()).map_err(|e| tls_error(format!("invalid TLS server name \"{host}\": {e}")))
}

/// Builds the rustls client configuration for the DSN's `sslmode` and certificate options.
pub fn client_config(cfg: &Config) -> Result<Arc<ClientConfig>> {
    let provider = provider();
    let versions: &[&'static rustls::SupportedProtocolVersion] = match cfg.tls_min_version.as_deref().map(str::to_ascii_lowercase).as_deref() {
        None | Some("tlsv1.2") | Some("1.2") => &[&rustls::version::TLS13, &rustls::version::TLS12],
        Some("tlsv1.3") | Some("1.3") => &[&rustls::version::TLS13],
        Some(other) => return Err(Error::invalid(format!("unsupported tls_version \"{other}\"; use TLSv1.2 or TLSv1.3"))),
    };
    let builder = ClientConfig::builder_with_provider(provider.clone()).with_protocol_versions(versions).map_err(|e| tls_error(e.to_string()))?;
    let verifier = verifier(cfg, provider)?;
    let builder = builder.dangerous().with_custom_certificate_verifier(verifier);

    let config = match (&cfg.ssl_cert, &cfg.ssl_key) {
        (Some(cert), Some(key)) => {
            let chain = CertificateDer::pem_file_iter(cert).map_err(|e| tls_error(format!("cannot read ssl_cert {cert}: {e}")))?.collect::<std::result::Result<Vec<_>, _>>().map_err(|e| tls_error(format!("cannot parse ssl_cert {cert}: {e}")))?;
            let key = PrivateKeyDer::from_pem_file(key).map_err(|e| tls_error(format!("cannot read ssl_key {key}: {e}")))?;
            builder.with_client_auth_cert(chain, key).map_err(|e| tls_error(format!("invalid client certificate: {e}")))?
        }
        (None, None) => builder.with_no_client_auth(),
        _ => return Err(Error::invalid("ssl_cert and ssl_key must be given together")),
    };
    Ok(Arc::new(config))
}

fn verifier(cfg: &Config, provider: Arc<CryptoProvider>) -> Result<Arc<dyn ServerCertVerifier>> {
    let custom_roots = custom_roots(cfg)?;
    let identity: Arc<dyn ServerCertVerifier> = match (cfg.ssl_mode, custom_roots) {
        (SslMode::Disable | SslMode::Preferred | SslMode::Required, None) => return Ok(Arc::new(AcceptAny(provider))),
        (_, Some(roots)) => WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider.clone()).build().map_err(|e| tls_error(e.to_string()))?,
        (_, None) => Arc::new(rustls_platform_verifier::Verifier::new(provider.clone()).map_err(|e| tls_error(e.to_string()))?),
    };
    Ok(match cfg.ssl_mode {
        SslMode::VerifyIdentity => identity,
        _ => Arc::new(IgnoreName(identity)),
    })
}

/// A CA file or directory given explicitly replaces the system trust store.
fn custom_roots(cfg: &Config) -> Result<Option<RootCertStore>> {
    let mut files: Vec<String> = cfg.ssl_ca.iter().cloned().collect();
    if let Some(dir) = &cfg.ssl_capath {
        let entries = std::fs::read_dir(dir).map_err(|e| tls_error(format!("cannot read ssl_capath {dir}: {e}")))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path.to_string_lossy().into_owned());
            }
        }
    }
    if files.is_empty() {
        return Ok(None);
    }
    let mut roots = RootCertStore::empty();
    for file in files {
        for cert in CertificateDer::pem_file_iter(Path::new(&file)).map_err(|e| tls_error(format!("cannot read CA {file}: {e}")))? {
            let cert = cert.map_err(|e| tls_error(format!("cannot parse CA {file}: {e}")))?;
            roots.add(cert).map_err(|e| tls_error(format!("invalid CA certificate in {file}: {e}")))?;
        }
    }
    if roots.is_empty() {
        return Err(tls_error("no CA certificates found in ssl_ca / ssl_capath"));
    }
    Ok(Some(roots))
}

/// `sslmode=preferred|required` without a CA: encrypt, do not authenticate the server (the libmysqlclient behaviour).
#[derive(Debug)]
struct AcceptAny(Arc<CryptoProvider>);

impl ServerCertVerifier for AcceptAny {
    fn verify_server_cert(&self, _: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// `sslmode=verify_ca`: the chain must be trusted, the host name is not checked.
#[derive(Debug)]
struct IgnoreName(Arc<dyn ServerCertVerifier>);

impl ServerCertVerifier for IgnoreName {
    fn verify_server_cert(&self, end_entity: &CertificateDer<'_>, intermediates: &[CertificateDer<'_>], server_name: &ServerName<'_>, ocsp: &[u8], now: UnixTime) -> std::result::Result<ServerCertVerified, rustls::Error> {
        match self.0.verify_server_cert(end_entity, intermediates, server_name, ocsp, now) {
            Err(rustls::Error::InvalidCertificate(rustls::CertificateError::NotValidForName | rustls::CertificateError::NotValidForNameContext { .. })) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.0.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.0.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_verify_schemes()
    }
}
