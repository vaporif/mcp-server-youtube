use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SearchOrder {
    Relevance,
    Date,
    Rating,
    ViewCount,
    Title,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VideoDuration {
    /// Less than 4 minutes
    Short,
    /// 4–20 minutes
    Medium,
    /// Longer than 20 minutes
    Long,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VideoDefinition {
    /// HD quality
    High,
    /// SD quality
    Standard,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    /// Active livestream
    Live,
    /// Scheduled but not yet started
    Upcoming,
    /// Past livestream
    Completed,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VideoCaption {
    /// Only videos with closed captions
    ClosedCaption,
    /// Only videos without closed captions
    None,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VideoLicense {
    /// Creative Commons license
    CreativeCommon,
    /// Standard `YouTube` license
    Youtube,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum VideoType {
    /// Any video type
    Any,
    /// Only regular episodes
    Episode,
    /// Only movies
    Movie,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SafeSearch {
    /// Strict filtering
    Strict,
    /// Moderate filtering (default)
    Moderate,
    /// No filtering
    None,
}

impl AsRef<str> for SearchOrder {
    fn as_ref(&self) -> &str {
        match self {
            Self::Relevance => "relevance",
            Self::Date => "date",
            Self::Rating => "rating",
            Self::ViewCount => "viewCount",
            Self::Title => "title",
        }
    }
}

impl AsRef<str> for VideoDuration {
    fn as_ref(&self) -> &str {
        match self {
            Self::Short => "short",
            Self::Medium => "medium",
            Self::Long => "long",
        }
    }
}

impl AsRef<str> for VideoDefinition {
    fn as_ref(&self) -> &str {
        match self {
            Self::High => "high",
            Self::Standard => "standard",
        }
    }
}

impl AsRef<str> for EventType {
    fn as_ref(&self) -> &str {
        match self {
            Self::Live => "live",
            Self::Upcoming => "upcoming",
            Self::Completed => "completed",
        }
    }
}

impl AsRef<str> for VideoCaption {
    fn as_ref(&self) -> &str {
        match self {
            Self::ClosedCaption => "closedCaption",
            Self::None => "none",
        }
    }
}

impl AsRef<str> for VideoLicense {
    fn as_ref(&self) -> &str {
        match self {
            Self::CreativeCommon => "creativeCommon",
            Self::Youtube => "youtube",
        }
    }
}

impl AsRef<str> for VideoType {
    fn as_ref(&self) -> &str {
        match self {
            Self::Any => "any",
            Self::Episode => "episode",
            Self::Movie => "movie",
        }
    }
}

impl AsRef<str> for SafeSearch {
    fn as_ref(&self) -> &str {
        match self {
            Self::Strict => "strict",
            Self::Moderate => "moderate",
            Self::None => "none",
        }
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct GetVideoParams {
    /// The video ID
    pub video_id: String,
    /// Resource parts to include (default: snippet, contentDetails, statistics)
    #[serde(default)]
    pub parts: Vec<String>,
}

#[derive(Clone, Default, Deserialize, JsonSchema)]
pub struct SearchVideosParams {
    /// Search query
    pub query: String,
    /// Maximum number of results (1-50, default: 10)
    #[serde(default = "default_search_max")]
    pub max_results: u32,
    /// Page token for pagination
    #[serde(default)]
    pub page_token: Option<String>,
    /// Sort order (default: relevance)
    #[serde(default)]
    pub order: Option<SearchOrder>,
    /// Filter by video duration
    #[serde(default)]
    pub video_duration: Option<VideoDuration>,
    /// Filter by video definition (HD/SD)
    #[serde(default)]
    pub video_definition: Option<VideoDefinition>,
    /// Filter by livestream event type
    #[serde(default)]
    pub event_type: Option<EventType>,
    /// Filter videos published after this date (RFC 3339, e.g. 2024-01-01T00:00:00Z)
    #[serde(default)]
    pub published_after: Option<String>,
    /// Filter videos published before this date (RFC 3339, e.g. 2024-12-31T23:59:59Z)
    #[serde(default)]
    pub published_before: Option<String>,
    /// Region code to restrict results (e.g. US, GB, JP)
    #[serde(default)]
    pub region_code: Option<String>,
    /// Filter by caption availability
    #[serde(default)]
    pub video_caption: Option<VideoCaption>,
    /// Filter by video license type
    #[serde(default)]
    pub video_license: Option<VideoLicense>,
    /// Filter by video type
    #[serde(default)]
    pub video_type: Option<VideoType>,
    /// Filter only embeddable videos
    #[serde(default)]
    pub embeddable_only: Option<bool>,
    /// Safe search filtering
    #[serde(default)]
    pub safe_search: Option<SafeSearch>,
    /// Video category ID
    #[serde(default)]
    pub video_category_id: Option<String>,
    /// Return results relevant to this language (ISO 639-1 code)
    #[serde(default)]
    pub relevance_language: Option<String>,
}

const fn default_search_max() -> u32 {
    10
}

#[derive(Deserialize, JsonSchema)]
pub struct GetCategoriesParams {
    /// Region code (default: US)
    #[serde(default = "default_region")]
    pub region_code: String,
}

fn default_region() -> String {
    "US".into()
}

#[derive(Deserialize, JsonSchema)]
pub struct GetChannelParams {
    /// The channel ID
    pub channel_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ListChannelVideosParams {
    /// The channel ID
    pub channel_id: String,
    /// Maximum number of results (1-50, default: 50)
    #[serde(default = "default_list_max")]
    pub max_results: u32,
    /// Page token for pagination
    #[serde(default)]
    pub page_token: Option<String>,
}

const fn default_list_max() -> u32 {
    50
}

#[derive(Deserialize, JsonSchema)]
pub struct GetPlaylistParams {
    /// The playlist ID
    pub playlist_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetPlaylistItemsParams {
    /// The playlist ID
    pub playlist_id: String,
    /// Maximum number of results (1-50, default: 50)
    #[serde(default = "default_list_max")]
    pub max_results: u32,
    /// Page token for pagination
    #[serde(default)]
    pub page_token: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetCommentsParams {
    /// The video ID
    pub video_id: String,
    /// Maximum number of results (1-50, default: 20)
    #[serde(default = "default_comments_max")]
    pub max_results: u32,
    /// Page token for pagination
    #[serde(default)]
    pub page_token: Option<String>,
}

const fn default_comments_max() -> u32 {
    20
}

#[derive(Deserialize, JsonSchema)]
pub struct GetTranscriptParams {
    /// The video ID
    pub video_id: String,
    /// Language code (uses server default if not specified)
    #[serde(default)]
    pub language: Option<String>,
    /// Include timestamps for each segment (default: false)
    #[serde(default)]
    pub include_timestamps: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetTrendingParams {
    /// Region code (default: US)
    #[serde(default = "default_region")]
    pub region_code: String,
    /// Video category ID to filter trending videos
    #[serde(default)]
    pub category_id: Option<String>,
    /// Maximum number of results (1-50, default: 10)
    #[serde(default = "default_search_max")]
    pub max_results: u32,
    /// Page token for pagination
    #[serde(default)]
    pub page_token: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetChannelByHandleParams {
    /// The channel handle (e.g. `@shura_stone`)
    pub handle: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchChannelsParams {
    /// Search query (channel name, topic, etc.)
    pub query: String,
    /// Maximum number of results (1-50, default: 10)
    #[serde(default = "default_search_max")]
    pub max_results: u32,
    /// Page token for pagination
    #[serde(default)]
    pub page_token: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetBatchTranscriptsParams {
    /// List of video IDs to fetch transcripts for
    pub video_ids: Vec<String>,
    /// Language code (uses server default if not specified)
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ListCaptionsParams {
    /// The video ID
    pub video_id: String,
}
