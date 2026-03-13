use std::sync::Arc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_handler, tool_router};

use crate::config::Config;
use crate::errors::Error;
use crate::key_pool::KeyPool;
use crate::params::{
    GetBatchTranscriptsParams, GetCategoriesParams, GetChannelByHandleParams, GetChannelParams,
    GetCommentsParams, GetPlaylistItemsParams, GetPlaylistParams, GetTranscriptParams,
    GetTrendingParams, GetVideoParams, ListCaptionsParams, ListChannelVideosParams,
    SearchChannelsParams, SearchVideosParams,
};
use crate::youtube::{YoutubeHub, create_hub};
use google_youtube3::api::{
    CommentThreadListCall, PlaylistItemListCall, SearchListCall, VideoListCall,
};

const YOUTUBE_MAX_RESULTS: u32 = 50;

fn parts(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).into()).collect()
}

fn clamp_max_results(n: u32) -> u32 {
    n.min(YOUTUBE_MAX_RESULTS)
}

struct OptionalFilters<C>(C);

impl<C> OptionalFilters<C> {
    fn apply(self, opt: Option<&(impl AsRef<str> + ?Sized)>, f: impl FnOnce(C, &str) -> C) -> Self {
        Self(match opt {
            Some(v) => f(self.0, v.as_ref()),
            None => self.0,
        })
    }

    fn build(self) -> C {
        self.0
    }
}

fn to_json_result(body: &impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(body).map_err(Error::from)?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn extract_event_text(event: &serde_json::Value) -> Option<String> {
    let segs = event["segs"].as_array()?;
    let text: String = segs.iter().filter_map(|s| s["utf8"].as_str()).collect();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[derive(Clone)]
pub struct YoutubeMcpServer {
    hub: Arc<YoutubeHub>,
    config: Arc<Config>,
    key_pool: Arc<KeyPool>,
    http: reqwest::Client,
    rustypipe: Arc<rustypipe::client::RustyPipe>,
    tool_router: ToolRouter<Self>,
}

impl YoutubeMcpServer {
    #[must_use]
    pub fn new(config: Arc<Config>, key_pool: Arc<KeyPool>) -> Self {
        let hub = Arc::new(create_hub());
        let tool_router = Self::tool_router();
        Self {
            hub,
            config,
            key_pool,
            http: reqwest::Client::new(),
            rustypipe: Arc::new(rustypipe::client::RustyPipe::new()),
            tool_router,
        }
    }
}

#[tool_router]
impl YoutubeMcpServer {
    #[tool(
        name = "videos_getVideo",
        description = "Get detailed information about a YouTube video including snippet, content details, and statistics. Takes a video ID (the 'v' parameter from YouTube URLs, e.g. dQw4w9WgXcQ from youtube.com/watch?v=dQw4w9WgXcQ). Prefer this over search when you already have a video URL or ID."
    )]
    async fn get_video(
        &self,
        Parameters(params): Parameters<GetVideoParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(video_id = %params.video_id, "get_video");
        let parts = if params.parts.is_empty() {
            parts(&["snippet", "contentDetails", "statistics"])
        } else {
            params.parts
        };

        let hub = self.hub.clone();
        let video_id = params.video_id.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let video_id = video_id.clone();
                async move {
                    hub.videos()
                        .list(&parts)
                        .add_id(&video_id)
                        .param("key", &key)
                        .doit()
                        .await
                        .map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "videos_searchVideos",
        description = "Search for videos on YouTube with optional filters: sort order (relevance/date/viewCount/rating), duration (short/medium/long), definition (HD/SD), date range, region, caption availability, license type, category, and safe search. Returns paginated results with nextPageToken."
    )]
    async fn search_videos(
        &self,
        Parameters(params): Parameters<SearchVideosParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(query = %params.query, max_results = params.max_results, "search_videos");
        let parts = parts(&["snippet"]);

        let hub = self.hub.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let params = params.clone();
                async move {
                    let call = hub
                        .search()
                        .list(&parts)
                        .q(&params.query)
                        .max_results(clamp_max_results(params.max_results))
                        .add_type("video")
                        .param("key", &key);

                    let mut call = OptionalFilters(call)
                        .apply(params.page_token.as_deref(), SearchListCall::page_token)
                        .apply(params.order.as_ref(), SearchListCall::order)
                        .apply(
                            params.video_duration.as_ref(),
                            SearchListCall::video_duration,
                        )
                        .apply(
                            params.video_definition.as_ref(),
                            SearchListCall::video_definition,
                        )
                        .apply(params.event_type.as_ref(), SearchListCall::event_type)
                        .apply(params.region_code.as_deref(), SearchListCall::region_code)
                        .apply(params.video_caption.as_ref(), SearchListCall::video_caption)
                        .apply(params.video_license.as_ref(), SearchListCall::video_license)
                        .apply(params.video_type.as_ref(), SearchListCall::video_type)
                        .apply(params.safe_search.as_ref(), SearchListCall::safe_search)
                        .apply(
                            params.video_category_id.as_deref(),
                            SearchListCall::video_category_id,
                        )
                        .apply(
                            params.relevance_language.as_deref(),
                            SearchListCall::relevance_language,
                        )
                        .build();

                    if params.embeddable_only == Some(true) {
                        call = call.video_embeddable("true");
                    }
                    if let Some(after) = &params.published_after {
                        let dt = after
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .map_err(|e| {
                                Error::Config(format!("invalid published_after date: {e}"))
                            })?;
                        call = call.published_after(dt);
                    }
                    if let Some(before) = &params.published_before {
                        let dt = before
                            .parse::<chrono::DateTime<chrono::Utc>>()
                            .map_err(|e| {
                                Error::Config(format!("invalid published_before date: {e}"))
                            })?;
                        call = call.published_before(dt);
                    }

                    call.doit().await.map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "videos_getCategories",
        description = "List video categories with their IDs for a specific region. Use the returned category IDs with videos_searchVideos or videos_getTrending to filter by category."
    )]
    async fn get_categories(
        &self,
        Parameters(params): Parameters<GetCategoriesParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(region_code = %params.region_code, "get_categories");
        let parts = parts(&["snippet"]);

        let hub = self.hub.clone();
        let region_code = params.region_code.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let region_code = region_code.clone();
                async move {
                    hub.video_categories()
                        .list(&parts)
                        .region_code(&region_code)
                        .param("key", &key)
                        .doit()
                        .await
                        .map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "channels_getChannel",
        description = "Get information about a YouTube channel by ID, including snippet, statistics, and content details with uploads playlist ID"
    )]
    async fn get_channel(
        &self,
        Parameters(params): Parameters<GetChannelParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(channel_id = %params.channel_id, "get_channel");
        let parts = parts(&["snippet", "contentDetails", "statistics"]);

        let hub = self.hub.clone();
        let channel_id = params.channel_id.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let channel_id = channel_id.clone();
                async move {
                    hub.channels()
                        .list(&parts)
                        .add_id(&channel_id)
                        .param("key", &key)
                        .doit()
                        .await
                        .map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "channels_listVideos",
        description = "List videos from a specific YouTube channel, ordered by date (newest first). Returns paginated results. Tip: use channels_getByHandle first to resolve a handle to a channel ID."
    )]
    async fn list_channel_videos(
        &self,
        Parameters(params): Parameters<ListChannelVideosParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(channel_id = %params.channel_id, max_results = params.max_results, "list_channel_videos");

        // First call: resolve the channel's uploads playlist ID
        let hub = self.hub.clone();
        let channel_id = params.channel_id.clone();
        let (_, channel) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let channel_id = channel_id.clone();
                async move {
                    hub.channels()
                        .list(&parts(&["contentDetails"]))
                        .add_id(&channel_id)
                        .param("key", &key)
                        .doit()
                        .await
                        .map_err(Error::from)
                }
            })
            .await?;

        let uploads_id = channel
            .items
            .as_ref()
            .and_then(|items| items.first())
            .and_then(|ch| ch.content_details.as_ref())
            .and_then(|cd| cd.related_playlists.as_ref())
            .and_then(|rp| rp.uploads.as_deref())
            .ok_or_else(|| {
                Error::Config(format!(
                    "no uploads playlist for channel {}",
                    params.channel_id
                ))
            })?
            .to_string();

        // Second call: list videos from the uploads playlist
        let hub = self.hub.clone();
        let video_parts = parts(&["snippet", "contentDetails"]);
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let video_parts = video_parts.clone();
                let uploads_id = uploads_id.clone();
                let page_token = params.page_token.clone();
                async move {
                    let call = OptionalFilters(
                        hub.playlist_items()
                            .list(&video_parts)
                            .playlist_id(&uploads_id)
                            .max_results(clamp_max_results(params.max_results))
                            .param("key", &key),
                    )
                    .apply(page_token.as_deref(), PlaylistItemListCall::page_token)
                    .build();

                    call.doit().await.map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "playlists_getPlaylist",
        description = "Get information about a YouTube playlist"
    )]
    async fn get_playlist(
        &self,
        Parameters(params): Parameters<GetPlaylistParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(playlist_id = %params.playlist_id, "get_playlist");
        let parts = parts(&["snippet", "contentDetails"]);

        let hub = self.hub.clone();
        let playlist_id = params.playlist_id.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let playlist_id = playlist_id.clone();
                async move {
                    hub.playlists()
                        .list(&parts)
                        .add_id(&playlist_id)
                        .param("key", &key)
                        .doit()
                        .await
                        .map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "playlists_getPlaylistItems",
        description = "List videos in a YouTube playlist. Returns paginated results. Use with the uploads playlist ID from channels_getChannel or channels_getByHandle to list all channel videos."
    )]
    async fn get_playlist_items(
        &self,
        Parameters(params): Parameters<GetPlaylistItemsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(playlist_id = %params.playlist_id, max_results = params.max_results, "get_playlist_items");
        let parts = parts(&["snippet", "contentDetails"]);

        let hub = self.hub.clone();
        let playlist_id = params.playlist_id.clone();
        let page_token = params.page_token.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let playlist_id = playlist_id.clone();
                let page_token = page_token.clone();
                async move {
                    let call = OptionalFilters(
                        hub.playlist_items()
                            .list(&parts)
                            .playlist_id(&playlist_id)
                            .max_results(clamp_max_results(params.max_results))
                            .param("key", &key),
                    )
                    .apply(page_token.as_deref(), PlaylistItemListCall::page_token)
                    .build();

                    call.doit().await.map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "comments_getComments",
        description = "Get comment threads on a YouTube video. Returns paginated results."
    )]
    async fn get_comments(
        &self,
        Parameters(params): Parameters<GetCommentsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(video_id = %params.video_id, max_results = params.max_results, "get_comments");
        let parts = parts(&["snippet", "replies"]);

        let hub = self.hub.clone();
        let video_id = params.video_id.clone();
        let page_token = params.page_token.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let video_id = video_id.clone();
                let page_token = page_token.clone();
                async move {
                    let call = OptionalFilters(
                        hub.comment_threads()
                            .list(&parts)
                            .video_id(&video_id)
                            .max_results(clamp_max_results(params.max_results))
                            .param("key", &key),
                    )
                    .apply(page_token.as_deref(), CommentThreadListCall::page_token)
                    .build();

                    call.doit().await.map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "transcripts_getTranscript",
        description = "Get the transcript/captions of a YouTube video. Works with both auto-generated and manual captions. Returns plain text by default; set include_timestamps=true for per-segment timing. Use transcripts_listLanguages first to check available languages. Does not consume API key quota."
    )]
    async fn get_transcript(
        &self,
        Parameters(params): Parameters<GetTranscriptParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(video_id = %params.video_id, language = ?params.language, "get_transcript");
        let lang = params
            .language
            .as_deref()
            .unwrap_or(&self.config.youtube.transcript_lang);

        let player = self
            .rustypipe
            .query()
            .player(&params.video_id)
            .await
            .map_err(Error::from)?;

        let subtitle = player
            .subtitles
            .iter()
            .find(|s| s.lang == lang)
            .or_else(|| player.subtitles.first())
            .ok_or_else(|| Error::NoSubtitles(params.video_id.clone()))?;

        let actual_lang = &subtitle.lang;
        let url = format!("{}&fmt=json3", subtitle.url);
        let json: serde_json::Value = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(Error::from)?
            .json()
            .await
            .map_err(Error::from)?;

        let empty = vec![];
        let events = json["events"].as_array().unwrap_or(&empty);

        if params.include_timestamps {
            let segments: Vec<serde_json::Value> = events
                .iter()
                .filter_map(|event| {
                    let text = extract_event_text(event)?;
                    #[allow(clippy::cast_precision_loss)] // ms values are small
                    let start = event["tStartMs"].as_u64().unwrap_or(0) as f64 / 1000.0;
                    #[allow(clippy::cast_precision_loss)]
                    let duration = event["dDurationMs"].as_u64().unwrap_or(0) as f64 / 1000.0;
                    Some(serde_json::json!({
                        "text": text,
                        "start": start,
                        "duration": duration,
                    }))
                })
                .collect();

            to_json_result(&serde_json::json!({
                "video_id": params.video_id,
                "language": actual_lang,
                "segments": segments,
            }))
        } else {
            let text = events
                .iter()
                .filter_map(extract_event_text)
                .collect::<Vec<_>>()
                .join(" ");

            to_json_result(&serde_json::json!({
                "video_id": params.video_id,
                "language": actual_lang,
                "text": text,
            }))
        }
    }

    #[tool(
        name = "videos_getTrending",
        description = "Get trending/most popular videos for a region, optionally filtered by category. Use videos_getCategories first to find valid category IDs for the region."
    )]
    async fn get_trending(
        &self,
        Parameters(params): Parameters<GetTrendingParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(region_code = %params.region_code, category_id = ?params.category_id, "get_trending");
        let parts = parts(&["snippet", "contentDetails", "statistics"]);

        let hub = self.hub.clone();
        let region_code = params.region_code.clone();
        let category_id = params.category_id.clone();
        let page_token = params.page_token.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let region_code = region_code.clone();
                let category_id = category_id.clone();
                let page_token = page_token.clone();
                async move {
                    let call = OptionalFilters(
                        hub.videos()
                            .list(&parts)
                            .chart("mostPopular")
                            .region_code(&region_code)
                            .max_results(clamp_max_results(params.max_results))
                            .param("key", &key),
                    )
                    .apply(category_id.as_deref(), VideoListCall::video_category_id)
                    .apply(page_token.as_deref(), VideoListCall::page_token)
                    .build();

                    call.doit().await.map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "channels_getByHandle",
        description = "Look up a YouTube channel by its handle (e.g. @shura_stone). Returns channel info, statistics, and content details including the uploads playlist ID. Use this first when you have a channel name/handle, then use channels_listVideos or playlists_getPlaylistItems with the uploads playlist ID for full video listings."
    )]
    async fn get_channel_by_handle(
        &self,
        Parameters(params): Parameters<GetChannelByHandleParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(handle = %params.handle, "get_channel_by_handle");
        let parts = parts(&["snippet", "contentDetails", "statistics"]);

        let hub = self.hub.clone();
        let handle = params.handle.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let handle = handle.clone();
                async move {
                    hub.channels()
                        .list(&parts)
                        .for_handle(&handle)
                        .param("key", &key)
                        .doit()
                        .await
                        .map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "channels_search",
        description = "Search for YouTube channels by name or topic. Returns channel IDs, titles, descriptions, and subscriber counts. Use this when you don't know the exact channel handle."
    )]
    async fn search_channels(
        &self,
        Parameters(params): Parameters<SearchChannelsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(query = %params.query, max_results = params.max_results, "search_channels");
        let parts = parts(&["snippet"]);

        let hub = self.hub.clone();
        let query = params.query.clone();
        let page_token = params.page_token.clone();
        let (_, body) = self
            .key_pool
            .execute_with_key(|key: String| {
                let hub = hub.clone();
                let parts = parts.clone();
                let query = query.clone();
                let page_token = page_token.clone();
                async move {
                    let call = OptionalFilters(
                        hub.search()
                            .list(&parts)
                            .q(&query)
                            .add_type("channel")
                            .max_results(clamp_max_results(params.max_results))
                            .param("key", &key),
                    )
                    .apply(page_token.as_deref(), SearchListCall::page_token)
                    .build();

                    call.doit().await.map_err(Error::from)
                }
            })
            .await?;

        to_json_result(&body)
    }

    #[tool(
        name = "transcripts_listLanguages",
        description = "List available subtitle/caption languages for a YouTube video. Use before transcripts_getTranscript to check which languages are available. Does not consume API key quota."
    )]
    async fn list_captions(
        &self,
        Parameters(params): Parameters<ListCaptionsParams>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!(video_id = %params.video_id, "list_captions");
        let player = self
            .rustypipe
            .query()
            .player(&params.video_id)
            .await
            .map_err(Error::from)?;

        let languages: Vec<serde_json::Value> = player
            .subtitles
            .iter()
            .map(|s| {
                serde_json::json!({
                    "lang": s.lang,
                    "lang_name": s.lang_name,
                    "auto_generated": s.auto_generated,
                })
            })
            .collect();

        to_json_result(&serde_json::json!({
            "video_id": params.video_id,
            "languages": languages,
        }))
    }

    #[tool(
        name = "transcripts_getBatch",
        description = "Get transcripts for multiple videos in a single call. Returns plain text transcripts for each video. Use this instead of calling transcripts_getTranscript repeatedly. Does not consume API key quota."
    )]
    async fn get_batch_transcripts(
        &self,
        Parameters(params): Parameters<GetBatchTranscriptsParams>,
    ) -> Result<CallToolResult, McpError> {
        use futures::stream::{self, StreamExt};

        tracing::debug!(count = params.video_ids.len(), "get_batch_transcripts");
        let lang = params
            .language
            .as_deref()
            .unwrap_or(&self.config.youtube.transcript_lang);

        let results: Vec<_> = stream::iter(params.video_ids)
            .map(|video_id| {
                let this = self;
                async move {
                    match this.fetch_transcript_text(&video_id, lang).await {
                        Ok(text) => serde_json::json!({
                            "video_id": video_id,
                            "text": text,
                        }),
                        Err(e) => serde_json::json!({
                            "video_id": video_id,
                            "error": e.to_string(),
                        }),
                    }
                }
            })
            .buffer_unordered(self.config.youtube.transcript_concurrency)
            .collect()
            .await;

        to_json_result(&serde_json::json!({ "transcripts": results }))
    }
}

impl YoutubeMcpServer {
    async fn fetch_transcript_text(&self, video_id: &str, lang: &str) -> Result<String, Error> {
        let player = self.rustypipe.query().player(video_id).await?;

        let subtitle = player
            .subtitles
            .iter()
            .find(|s| s.lang == lang)
            .or_else(|| player.subtitles.first())
            .ok_or_else(|| Error::NoSubtitles(video_id.to_string()))?;

        let url = format!("{}&fmt=json3", subtitle.url);
        let json: serde_json::Value = self.http.get(&url).send().await?.json().await?;

        let empty = vec![];
        let events = json["events"].as_array().unwrap_or(&empty);
        let text = events
            .iter()
            .filter_map(extract_event_text)
            .collect::<Vec<_>>()
            .join(" ");

        Ok(text)
    }
}

#[cfg(any(test, feature = "test-helpers"))]
#[allow(clippy::missing_errors_doc)]
impl YoutubeMcpServer {
    pub async fn call_get_video(
        &self,
        params: Parameters<GetVideoParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_video(params).await
    }

    pub async fn call_search_videos(
        &self,
        params: Parameters<SearchVideosParams>,
    ) -> Result<CallToolResult, McpError> {
        self.search_videos(params).await
    }

    pub async fn call_get_categories(
        &self,
        params: Parameters<GetCategoriesParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_categories(params).await
    }

    pub async fn call_get_channel(
        &self,
        params: Parameters<GetChannelParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_channel(params).await
    }

    pub async fn call_list_channel_videos(
        &self,
        params: Parameters<ListChannelVideosParams>,
    ) -> Result<CallToolResult, McpError> {
        self.list_channel_videos(params).await
    }

    pub async fn call_get_playlist(
        &self,
        params: Parameters<GetPlaylistParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_playlist(params).await
    }

    pub async fn call_get_playlist_items(
        &self,
        params: Parameters<GetPlaylistItemsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_playlist_items(params).await
    }

    pub async fn call_get_comments(
        &self,
        params: Parameters<GetCommentsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_comments(params).await
    }

    pub async fn call_get_transcript(
        &self,
        params: Parameters<GetTranscriptParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_transcript(params).await
    }

    pub async fn call_get_trending(
        &self,
        params: Parameters<GetTrendingParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_trending(params).await
    }

    pub async fn call_get_channel_by_handle(
        &self,
        params: Parameters<GetChannelByHandleParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_channel_by_handle(params).await
    }

    pub async fn call_list_captions(
        &self,
        params: Parameters<ListCaptionsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.list_captions(params).await
    }

    pub async fn call_search_channels(
        &self,
        params: Parameters<SearchChannelsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.search_channels(params).await
    }

    pub async fn call_get_batch_transcripts(
        &self,
        params: Parameters<GetBatchTranscriptsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.get_batch_transcripts(params).await
    }
}

#[tool_handler]
impl ServerHandler for YoutubeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new("mcp-server-youtube", env!("CARGO_PKG_VERSION")),
        )
    }
}
