use rmcp::ErrorData as McpError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("YouTube API error: {0}")]
    YoutubeApi(Box<google_apis_common::Error>),

    #[error("transcript error: {0}")]
    Transcript(#[from] rustypipe::error::Error),

    #[error("transcript fetch error: {0}")]
    TranscriptFetch(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("no subtitles available for video {0}")]
    NoSubtitles(String),

    #[error("config: {0}")]
    Config(String),
}

impl From<google_apis_common::Error> for Error {
    fn from(err: google_apis_common::Error) -> Self {
        Self::YoutubeApi(Box::new(err))
    }
}

fn extract_api_error_message(value: &serde_json::Value) -> Option<(u32, &str, &str)> {
    let err = value.get("error")?;
    #[allow(clippy::cast_possible_truncation)]
    let code = err.get("code")?.as_u64()? as u32;
    let reason = err
        .get("errors")?
        .as_array()?
        .first()?
        .get("reason")?
        .as_str()?;
    let message = err.get("message")?.as_str()?;
    Some((code, reason, message))
}

fn friendly_youtube_error(err: &google_apis_common::Error) -> String {
    if let google_apis_common::Error::BadRequest(ref value) = *err
        && let Some((code, reason, _)) = extract_api_error_message(value)
    {
        return match reason {
            "quotaExceeded" => format!(
                "YouTube API quota exceeded (HTTP {code}). \
                 Daily quota resets at midnight Pacific Time. \
                 Check usage at https://console.cloud.google.com/apis/dashboard"
            ),
            "rateLimitExceeded" | "userRateLimitExceeded" => format!(
                "YouTube API rate limit hit (HTTP {code}). Please retry after a short wait."
            ),
            "forbidden" | "accessNotConfigured" => format!(
                "YouTube Data API access denied (HTTP {code}). \
                 Ensure the YouTube Data API v3 is enabled for your API key."
            ),
            "keyInvalid" => {
                "Invalid YouTube API key. Check your YOUTUBE_API_KEY configuration.".into()
            }
            "notFound" => format!("YouTube resource not found (HTTP {code})."),
            _ => format!("YouTube API error (HTTP {code}, {reason})."),
        };
    }
    err.to_string()
}

impl From<Error> for McpError {
    fn from(err: Error) -> Self {
        let message = match &err {
            Error::YoutubeApi(api_err) => friendly_youtube_error(api_err),
            _ => err.to_string(),
        };
        tracing::error!("{message}");
        Self::internal_error(message, None)
    }
}
