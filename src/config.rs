use std::net::IpAddr;

use clap::{Parser, ValueEnum};
use secrecy::{ExposeSecret, SecretString};

#[derive(Parser, Debug)]
#[command(name = "mcp-server-youtube", about = "MCP server for YouTube")]
pub struct Cli {
    /// `YouTube` Data API key (comma-separated for multiple keys)
    #[arg(long, env = "YOUTUBE_API_KEY")]
    pub youtube_api_key: Option<String>,

    /// Default transcript language
    #[arg(long, default_value = "en", env = "YOUTUBE_TRANSCRIPT_LANG")]
    pub transcript_lang: String,

    /// Transport protocol
    #[arg(long, default_value = "stdio", env = "MCP_TRANSPORT")]
    pub transport: TransportArg,

    /// Host to bind for HTTP transport
    #[arg(long, default_value = "127.0.0.1", env = "HOST")]
    pub host: IpAddr,

    /// Port for HTTP transport
    #[arg(long, default_value = "3000", env = "PORT")]
    pub port: u16,

    /// Max concurrent transcript fetches for batch operations
    #[arg(long, default_value = "50", env = "YOUTUBE_TRANSCRIPT_CONCURRENCY")]
    pub transcript_concurrency: usize,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum TransportArg {
    Stdio,
    StreamableHttp,
}

pub enum Transport {
    Stdio,
    Http { host: IpAddr, port: u16 },
}

pub struct YoutubeConfig {
    pub api_keys: Vec<SecretString>,
    pub transcript_lang: String,
    pub transcript_concurrency: usize,
}

impl YoutubeConfig {
    #[must_use]
    pub fn first_key_as_str(&self) -> &str {
        self.api_keys[0].expose_secret()
    }
}

pub struct Config {
    pub youtube: YoutubeConfig,
    pub transport: Transport,
}

impl Config {
    /// # Errors
    /// Returns an error if `YOUTUBE_API_KEY` is not provided or is empty.
    pub fn from_cli(cli: Cli) -> Result<Self, crate::errors::Error> {
        let raw = cli
            .youtube_api_key
            .ok_or_else(|| crate::errors::Error::Config("YOUTUBE_API_KEY is required".into()))?;

        let api_keys: Vec<SecretString> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| SecretString::from(s.to_string()))
            .collect();

        if api_keys.is_empty() {
            return Err(crate::errors::Error::Config(
                "YOUTUBE_API_KEY must contain at least one key".into(),
            ));
        }

        let transport = match cli.transport {
            TransportArg::Stdio => Transport::Stdio,
            TransportArg::StreamableHttp => Transport::Http {
                host: cli.host,
                port: cli.port,
            },
        };

        Ok(Self {
            youtube: YoutubeConfig {
                api_keys,
                transcript_lang: cli.transcript_lang,
                transcript_concurrency: cli.transcript_concurrency,
            },
            transport,
        })
    }
}
