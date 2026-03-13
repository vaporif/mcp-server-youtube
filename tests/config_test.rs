use clap::Parser;
use mcp_server_youtube::config::{Cli, Config};
use secrecy::ExposeSecret;

#[test]
fn config_from_cli_with_api_key() {
    let cli = Cli::parse_from(["mcp-server-youtube", "--youtube-api-key", "test-key-123"]);
    let config = Config::from_cli(cli).unwrap();
    assert_eq!(config.youtube.api_keys.len(), 1);
    assert_eq!(config.youtube.api_keys[0].expose_secret(), "test-key-123");
    assert_eq!(config.youtube.transcript_lang, "en");
}

#[test]
fn config_from_cli_with_multiple_api_keys() {
    let cli = Cli::parse_from(["mcp-server-youtube", "--youtube-api-key", "key1,key2,key3"]);
    let config = Config::from_cli(cli).unwrap();
    assert_eq!(config.youtube.api_keys.len(), 3);
    assert_eq!(config.youtube.api_keys[0].expose_secret(), "key1");
    assert_eq!(config.youtube.api_keys[1].expose_secret(), "key2");
    assert_eq!(config.youtube.api_keys[2].expose_secret(), "key3");
}

#[test]
fn config_from_cli_trims_whitespace_around_keys() {
    let cli = Cli::parse_from([
        "mcp-server-youtube",
        "--youtube-api-key",
        "key1 , key2 , key3",
    ]);
    let config = Config::from_cli(cli).unwrap();
    assert_eq!(config.youtube.api_keys.len(), 3);
    assert_eq!(config.youtube.api_keys[0].expose_secret(), "key1");
    assert_eq!(config.youtube.api_keys[1].expose_secret(), "key2");
    assert_eq!(config.youtube.api_keys[2].expose_secret(), "key3");
}

#[test]
fn config_rejects_missing_api_key() {
    let cli = Cli::parse_from(["mcp-server-youtube"]);
    assert!(Config::from_cli(cli).is_err());
}

#[test]
fn config_custom_transcript_lang() {
    let cli = Cli::parse_from([
        "mcp-server-youtube",
        "--youtube-api-key",
        "key",
        "--transcript-lang",
        "es",
    ]);
    let config = Config::from_cli(cli).unwrap();
    assert_eq!(config.youtube.transcript_lang, "es");
}

#[test]
fn config_http_transport() {
    let cli = Cli::parse_from([
        "mcp-server-youtube",
        "--youtube-api-key",
        "key",
        "--transport",
        "streamable-http",
        "--host",
        "0.0.0.0",
        "--port",
        "9000",
    ]);
    let config = Config::from_cli(cli).unwrap();
    assert!(matches!(
        config.transport,
        mcp_server_youtube::config::Transport::Http { .. }
    ));
}

#[test]
fn config_defaults() {
    let cli = Cli::parse_from(["mcp-server-youtube", "--youtube-api-key", "key"]);
    let config = Config::from_cli(cli).unwrap();
    assert_eq!(config.youtube.transcript_lang, "en");
    assert!(matches!(
        config.transport,
        mcp_server_youtube::config::Transport::Stdio
    ));
}
