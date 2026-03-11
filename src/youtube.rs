use google_apis_common::NoToken;
use google_youtube3::api::YouTube;
use google_youtube3::hyper_rustls;
use google_youtube3::hyper_util;

pub type YoutubeHub =
    YouTube<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>>;

/// # Panics
/// Panics if native root TLS certificates cannot be loaded.
#[must_use]
pub fn create_hub() -> YoutubeHub {
    // Install default crypto provider (no-op if already set).
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .expect("native root certs")
                .https_only()
                .enable_http2()
                .build(),
        );

    YouTube::new(client, NoToken)
}
