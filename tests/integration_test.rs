use std::sync::Arc;

use mcp_server_youtube::config::{Config, Transport, YoutubeConfig};
use mcp_server_youtube::params::SearchVideosParams;
use mcp_server_youtube::server::YoutubeMcpServer;
use rmcp::handler::server::wrapper::Parameters;
use secrecy::SecretString;

/// These tests require a valid `YOUTUBE_API_KEY` environment variable.
/// Run with: `cargo nextest run --run-ignored ignored-only`
/// Or: `cargo test -- --ignored`
fn api_key() -> String {
    std::env::var("YOUTUBE_API_KEY").expect("YOUTUBE_API_KEY must be set for integration tests")
}

fn create_server() -> YoutubeMcpServer {
    let config = Arc::new(Config {
        youtube: YoutubeConfig {
            api_key: SecretString::from(api_key()),
            transcript_lang: "en".into(),
            transcript_concurrency: 50,
        },
        transport: Transport::Stdio,
    });
    YoutubeMcpServer::new(config)
}

// Well-known test fixtures:
// Video: "dQw4w9WgXcQ" (Rick Astley - Never Gonna Give You Up)
// Channel: "UCuAXFkgsw1L7xaCfnd5JJOw" (Rick Astley)
// Playlist: "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf" (YouTube Rewind)

// ── Videos ──

fn search_params(query: &str, max_results: u32) -> SearchVideosParams {
    SearchVideosParams {
        query: query.into(),
        max_results,
        ..Default::default()
    }
}

mod videos {
    use super::*;
    use mcp_server_youtube::params::{GetCategoriesParams, GetVideoParams};

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_video_returns_details() {
        let server = create_server();
        let result = server
            .call_get_video(Parameters(GetVideoParams {
                video_id: "dQw4w9WgXcQ".into(),
                parts: vec!["snippet".into(), "statistics".into()],
            }))
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        let text = extract_text(&result);
        assert!(text.contains("Rick Astley"), "should contain channel name");
        assert!(text.contains("viewCount"), "should contain statistics");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_video_with_default_parts() {
        let server = create_server();
        let result = server
            .call_get_video(Parameters(GetVideoParams {
                video_id: "dQw4w9WgXcQ".into(),
                parts: vec![],
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("snippet"));
        assert!(text.contains("contentDetails"));
        assert!(text.contains("statistics"));
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_video_not_found() {
        let server = create_server();
        let result = server
            .call_get_video(Parameters(GetVideoParams {
                video_id: "nonexistent_video_id_xyz".into(),
                parts: vec![],
            }))
            .await;

        // API returns empty items for nonexistent videos, not an error
        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\": []") || text.contains("\"items\":[]"));
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn search_videos_returns_results() {
        let server = create_server();
        let result = server
            .call_search_videos(Parameters(search_params("rust programming language", 3)))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
        assert!(text.contains("videoId"), "should contain video IDs");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn search_videos_pagination() {
        let server = create_server();

        // First page
        let result = server
            .call_search_videos(Parameters(search_params("music", 2)))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(
            text.contains("nextPageToken"),
            "first page should have nextPageToken"
        );

        // Extract nextPageToken
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let next_token = json["nextPageToken"].as_str().unwrap().to_string();

        // Second page
        let result = server
            .call_search_videos(Parameters({
                let mut p = search_params("music", 2);
                p.page_token = Some(next_token);
                p
            }))
            .await;

        assert!(result.is_ok());
        let text2 = extract_text(&result.unwrap());
        assert!(text2.contains("\"items\""), "second page should have items");
    }

    // Search filter tests are consolidated to minimize API quota usage.
    // Each search.list call costs 100 quota units.

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn search_videos_with_sorting_and_video_filters() {
        use mcp_server_youtube::params::{
            SafeSearch, SearchOrder, VideoCaption, VideoDefinition, VideoDuration,
        };

        let server = create_server();
        let mut params = search_params("rust programming", 3);
        params.order = Some(SearchOrder::ViewCount);
        params.video_duration = Some(VideoDuration::Medium);
        params.video_definition = Some(VideoDefinition::High);
        params.video_caption = Some(VideoCaption::ClosedCaption);
        params.safe_search = Some(SafeSearch::Strict);
        params.embeddable_only = Some(true);
        params.region_code = Some("US".into());
        params.relevance_language = Some("en".into());
        let result = server.call_search_videos(Parameters(params)).await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
        assert!(text.contains("videoId"), "should contain video IDs");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn search_videos_with_date_range_and_metadata_filters() {
        use mcp_server_youtube::params::{SearchOrder, VideoLicense, VideoType};

        let server = create_server();
        let mut params = search_params("nature documentary", 3);
        params.order = Some(SearchOrder::Date);
        params.published_after = Some("2023-01-01T00:00:00Z".into());
        params.published_before = Some("2024-12-31T23:59:59Z".into());
        params.video_license = Some(VideoLicense::CreativeCommon);
        params.video_type = Some(VideoType::Any);
        params.video_category_id = Some("10".into());
        let result = server.call_search_videos(Parameters(params)).await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn search_videos_invalid_published_after_returns_error() {
        let server = create_server();
        let mut params = search_params("test", 1);
        params.published_after = Some("not-a-date".into());
        let result = server.call_search_videos(Parameters(params)).await;

        assert!(result.is_err(), "invalid date should return an error");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_categories_returns_list() {
        let server = create_server();
        let result = server
            .call_get_categories(Parameters(GetCategoriesParams {
                region_code: "US".into(),
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("Music"), "should contain Music category");
        assert!(text.contains("Gaming"), "should contain Gaming category");
    }
}

// ── Channels ──

mod channels {
    use super::*;
    use mcp_server_youtube::params::{GetChannelParams, ListChannelVideosParams};

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_channel_returns_info() {
        let server = create_server();
        let result = server
            .call_get_channel(Parameters(GetChannelParams {
                channel_id: "UCuAXFkgsw1L7xaCfnd5JJOw".into(),
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("Rick Astley"), "should contain channel name");
        assert!(
            text.contains("subscriberCount"),
            "should contain statistics"
        );
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn list_channel_videos_returns_results() {
        let server = create_server();
        let result = server
            .call_list_channel_videos(Parameters(ListChannelVideosParams {
                channel_id: "UCuAXFkgsw1L7xaCfnd5JJOw".into(),
                max_results: 5,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
        assert!(text.contains("videoId"), "should contain video IDs");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn list_channel_videos_pagination() {
        let server = create_server();
        let result = server
            .call_list_channel_videos(Parameters(ListChannelVideosParams {
                channel_id: "UCuAXFkgsw1L7xaCfnd5JJOw".into(),
                max_results: 2,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());

        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        if let Some(next_token) = json["nextPageToken"].as_str() {
            let result = server
                .call_list_channel_videos(Parameters(ListChannelVideosParams {
                    channel_id: "UCuAXFkgsw1L7xaCfnd5JJOw".into(),
                    max_results: 2,
                    page_token: Some(next_token.to_string()),
                }))
                .await;

            assert!(result.is_ok());
        }
    }
}

// ── Playlists ──

mod playlists {
    use super::*;
    use mcp_server_youtube::params::{GetPlaylistItemsParams, GetPlaylistParams};

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_playlist_returns_info() {
        let server = create_server();
        let result = server
            .call_get_playlist(Parameters(GetPlaylistParams {
                playlist_id: "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf".into(),
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
        assert!(text.contains("snippet"), "should contain snippet");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_playlist_items_returns_videos() {
        let server = create_server();
        let result = server
            .call_get_playlist_items(Parameters(GetPlaylistItemsParams {
                playlist_id: "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf".into(),
                max_results: 5,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
        assert!(text.contains("videoId"), "should contain video IDs");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_playlist_items_pagination() {
        let server = create_server();
        let result = server
            .call_get_playlist_items(Parameters(GetPlaylistItemsParams {
                playlist_id: "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf".into(),
                max_results: 2,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());

        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        if let Some(next_token) = json["nextPageToken"].as_str() {
            let result = server
                .call_get_playlist_items(Parameters(GetPlaylistItemsParams {
                    playlist_id: "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf".into(),
                    max_results: 2,
                    page_token: Some(next_token.to_string()),
                }))
                .await;

            assert!(result.is_ok());
        }
    }
}

// ── Comments ──

mod comments {
    use super::*;
    use mcp_server_youtube::params::GetCommentsParams;

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_comments_returns_threads() {
        let server = create_server();
        let result = server
            .call_get_comments(Parameters(GetCommentsParams {
                video_id: "dQw4w9WgXcQ".into(),
                max_results: 5,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
        assert!(text.contains("textDisplay"), "should contain comment text");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_comments_pagination() {
        let server = create_server();
        let result = server
            .call_get_comments(Parameters(GetCommentsParams {
                video_id: "dQw4w9WgXcQ".into(),
                max_results: 2,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());

        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        if let Some(next_token) = json["nextPageToken"].as_str() {
            let result = server
                .call_get_comments(Parameters(GetCommentsParams {
                    video_id: "dQw4w9WgXcQ".into(),
                    max_results: 2,
                    page_token: Some(next_token.to_string()),
                }))
                .await;

            assert!(result.is_ok());
        }
    }
}

// ── Transcripts ──

mod transcripts {
    use super::*;
    use mcp_server_youtube::params::GetTranscriptParams;

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_transcript_returns_text() {
        let server = create_server();
        let result = server
            .call_get_transcript(Parameters(GetTranscriptParams {
                video_id: "dQw4w9WgXcQ".into(),
                language: Some("en".into()),
                include_timestamps: false,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"text\""), "should contain text field");
        assert!(!text.contains("\"start\""), "should not contain timestamps");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_transcript_with_timestamps() {
        let server = create_server();
        let result = server
            .call_get_transcript(Parameters(GetTranscriptParams {
                video_id: "dQw4w9WgXcQ".into(),
                language: Some("en".into()),
                include_timestamps: true,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("segments"), "should contain segments");
        assert!(text.contains("\"text\""), "should contain text field");
        assert!(text.contains("\"start\""), "should contain start field");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_transcript_uses_default_language() {
        let server = create_server();
        let result = server
            .call_get_transcript(Parameters(GetTranscriptParams {
                video_id: "dQw4w9WgXcQ".into(),
                language: None,
                include_timestamps: false,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(
            text.contains("\"language\": \"en\""),
            "should use default language"
        );
    }
}

mod trending {
    use super::*;
    use mcp_server_youtube::params::GetTrendingParams;

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_trending_returns_videos() {
        let server = create_server();
        let result = server
            .call_get_trending(Parameters(GetTrendingParams {
                region_code: "US".into(),
                category_id: None,
                max_results: 5,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
        assert!(text.contains("viewCount"), "should contain statistics");
    }

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_trending_with_category() {
        let server = create_server();
        let result = server
            .call_get_trending(Parameters(GetTrendingParams {
                region_code: "US".into(),
                category_id: Some("10".into()), // Music
                max_results: 5,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
    }
}

mod channel_handle {
    use super::*;
    use mcp_server_youtube::params::GetChannelByHandleParams;

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_channel_by_handle() {
        let server = create_server();
        let result = server
            .call_get_channel_by_handle(Parameters(GetChannelByHandleParams {
                handle: "@RickAstleyYT".into(),
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("Rick Astley"), "should contain channel name");
        assert!(
            text.contains("subscriberCount"),
            "should contain statistics"
        );
    }
}

mod captions {
    use super::*;
    use mcp_server_youtube::params::ListCaptionsParams;

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn list_captions_returns_languages() {
        let server = create_server();
        let result = server
            .call_list_captions(Parameters(ListCaptionsParams {
                video_id: "dQw4w9WgXcQ".into(),
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("languages"), "should contain languages");
        assert!(text.contains("\"lang\""), "should contain lang field");
    }
}

// ── Channel Search ──

mod channel_search {
    use super::*;
    use mcp_server_youtube::params::SearchChannelsParams;

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn search_channels_returns_results() {
        let server = create_server();
        let result = server
            .call_search_channels(Parameters(SearchChannelsParams {
                query: "Rick Astley".into(),
                max_results: 3,
                page_token: None,
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("\"items\""), "should contain items");
        assert!(text.contains("channelId"), "should contain channel IDs");
    }
}

// ── Batch Transcripts ──

mod batch_transcripts {
    use super::*;
    use mcp_server_youtube::params::GetBatchTranscriptsParams;

    #[tokio::test]
    #[ignore = "requires YOUTUBE_API_KEY"]
    async fn get_batch_transcripts_returns_texts() {
        let server = create_server();
        let result = server
            .call_get_batch_transcripts(Parameters(GetBatchTranscriptsParams {
                video_ids: vec!["dQw4w9WgXcQ".into()],
                language: Some("en".into()),
            }))
            .await;

        assert!(result.is_ok());
        let text = extract_text(&result.unwrap());
        assert!(text.contains("transcripts"), "should contain transcripts");
        assert!(text.contains("\"text\""), "should contain text field");
    }
}

// ── Helpers ──

fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| c.as_text())
        .map(|t| t.text.clone())
        .collect::<String>()
}
