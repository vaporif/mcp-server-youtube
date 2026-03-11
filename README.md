# mcp-server-youtube

A Rust [MCP](https://modelcontextprotocol.io/) server for YouTube. Search videos, read channels, browse playlists, fetch comments, and get transcripts — all from your AI assistant.

### Why this one?

- **Full YouTube Data API v3 coverage** — videos, channels, playlists, comments, categories, trending, search with all filters and sorting options
- **Working transcripts** — uses [rustypipe](https://github.com/thedodd/rustypipe) (InnerTube API) for reliable subtitle/caption extraction, including auto-generated captions. No API key quota consumed for transcripts
- **Single binary, zero runtime dependencies** — written in Rust, starts instantly
- **Typed filter enums** — search filters (duration, definition, license, etc.) are exposed as typed enums in the JSON schema, so AI models know exactly which values are valid without guessing

## Tools

### Videos

| Tool | Description |
|------|-------------|
| `videos_getVideo` | Get video details (snippet, statistics, content details) |
| `videos_searchVideos` | Search videos with filters for duration, definition, date range, region, captions, license, category, and more. Supports sorting by relevance, date, rating, or view count |
| `videos_getCategories` | List video categories for a region |
| `videos_getTrending` | Get trending/most popular videos for a region, optionally filtered by category |

### Channels

| Tool | Description |
|------|-------------|
| `channels_getChannel` | Get channel info and statistics by channel ID |
| `channels_getByHandle` | Look up a channel by its handle (e.g. `@shura_stone`) |
| `channels_search` | Search for channels by name |
| `channels_listVideos` | List videos from a channel (by date) with pagination |

### Playlists

| Tool | Description |
|------|-------------|
| `playlists_getPlaylist` | Get playlist info |
| `playlists_getPlaylistItems` | List videos in a playlist with pagination |

### Comments

| Tool | Description |
|------|-------------|
| `comments_getComments` | Get comment threads on a video with pagination |

### Transcripts & Captions

| Tool | Description |
|------|-------------|
| `transcripts_getTranscript` | Get video transcript/subtitles. Works with both auto-generated and manual captions. Returns plain text by default; set `include_timestamps` for per-segment timing. Uses [rustypipe](https://github.com/thedodd/rustypipe) (InnerTube API) — no API key quota consumed |
| `transcripts_getBatch` | Fetch transcripts for multiple videos in one call with concurrent fetching. No API key quota consumed |
| `transcripts_listLanguages` | List available subtitle/caption languages for a video |

## Setup

### Get a YouTube API Key

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a project (or select an existing one)
3. Enable the **YouTube Data API v3**
4. Create an API key under **Credentials**

## Configuration

| Environment Variable | CLI Flag | Default | Description |
|---------------------|----------|---------|-------------|
| `YOUTUBE_API_KEY` | `--youtube-api-key` | *required* | YouTube Data API v3 key |
| `YOUTUBE_TRANSCRIPT_LANG` | `--transcript-lang` | `en` | Default transcript language |
| `MCP_TRANSPORT` | `--transport` | `stdio` | Transport: `stdio` or `streamable-http` |
| `HOST` | `--host` | `127.0.0.1` | HTTP bind address |
| `PORT` | `--port` | `3000` | HTTP port |
| `YOUTUBE_TRANSCRIPT_CONCURRENCY` | `--transcript-concurrency` | `50` | Max concurrent transcript fetches for batch operations |

The API key is stored as a `SecretString` and never appears in logs or debug output.

## Usage

### Claude Desktop / Claude Code

Add to your MCP config (requires [rvx](https://github.com/vaporif/rvx)):

```json
{
  "mcpServers": {
    "youtube": {
      "command": "rvx",
      "args": ["mcp-server-youtube"],
      "env": {
        "YOUTUBE_API_KEY": "YOUR_API_KEY"
      }
    }
  }
}
```

<details>
<summary>Other installation methods</summary>

**From source:**

```sh
cargo install --git https://github.com/vaporif/mcp-server-youtube
```

**With Nix:**

```sh
nix run github:vaporif/mcp-server-youtube
```

**From releases:**

Download a prebuilt binary from [GitHub Releases](https://github.com/vaporif/mcp-server-youtube/releases).

</details>

### HTTP Transport

```sh
YOUTUBE_API_KEY=your_key mcp-server-youtube --transport streamable-http --port 3000
```

The server listens on `http://127.0.0.1:3000/mcp`.

### Debugging

Enable debug logging to see tool invocations:

```sh
RUST_LOG=debug mcp-server-youtube
```

## Development

```sh
# Enter dev shell (requires Nix)
nix develop

# Build
cargo build

# Test
cargo test

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all
```

## License

GPL-3.0-or-later
