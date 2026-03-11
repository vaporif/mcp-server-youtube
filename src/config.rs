use std::net::IpAddr;

use clap::{Parser, ValueEnum};
use secrecy::{ExposeSecret, SecretString};

#[derive(Parser, Debug)]
#[command(name = "mcp-server-youtube", about = "MCP server for YouTube")]
pub struct Cli {
    /// `YouTube` Data API key
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
    pub api_key: SecretString,
    pub transcript_lang: String,
}

impl YoutubeConfig {
    #[must_use]
    pub fn api_key_as_str(&self) -> &str {
        self.api_key.expose_secret()
    }
}

pub struct Config {
    pub youtube: YoutubeConfig,
    pub transport: Transport,
}

impl Config {
    /// # Errors
    /// Returns an error if `YOUTUBE_API_KEY` is not provided.
    pub fn from_cli(cli: Cli) -> Result<Self, crate::errors::Error> {
        let api_key = cli
            .youtube_api_key
            .ok_or_else(|| crate::errors::Error::Config("YOUTUBE_API_KEY is required".into()))?;

        let transport = match cli.transport {
            TransportArg::Stdio => Transport::Stdio,
            TransportArg::StreamableHttp => Transport::Http {
                host: cli.host,
                port: cli.port,
            },
        };

        Ok(Self {
            youtube: YoutubeConfig {
                api_key: SecretString::from(api_key),
                transcript_lang: cli.transcript_lang,
            },
            transport,
        })
    }
}
