# Multi-Key Rotation with Quota Exhaustion Tracking

## Overview

Support multiple YouTube API keys with automatic failover when one hits quota, and persistence of exhaustion state across restarts.

## Configuration

- `YOUTUBE_API_KEY` accepts comma-separated values: `key1,key2,key3`
- Backwards-compatible: a single key works as before
- `YoutubeConfig` stores `Vec<SecretString>` instead of a single `SecretString`
- Validation requires at least one key

## KeyPool (`src/key_pool.rs`)

Central struct managing key selection, rotation, and exhaustion tracking.

### Fields

- `keys: Vec<SecretString>` — all configured keys
- `exhausted: RwLock<HashMap<String, DateTime<Utc>>>` — key hash to exhaustion timestamp
- `invalid: RwLock<HashSet<usize>>` — keys that returned `keyInvalid` (in-memory only, reset on restart)
- `cache_path: PathBuf` — path to persistence file

Key hashes use the first 8 characters of SHA-256 of the key value. This avoids writing secrets to disk while remaining stable across key reordering.

### `execute_with_key<F, T>(&self, f: F) -> Result<T>`

Async method that wraps individual YouTube API calls. The closure `f` takes `&str` (the API key) and must be re-invocable (`Fn`, not `FnOnce`) — captured params should be borrowed or cloned.

1. Auto-reset: clear exhausted keys whose timestamp is before the most recent midnight Pacific
2. Select the first non-exhausted, non-invalid key
3. If all keys exhausted/invalid: try the oldest-exhausted key (quota may have freed up)
4. Execute the closure with the selected key
5. On success: return result
6. On `quotaExceeded`: mark key exhausted with current timestamp, persist to disk, retry with next key
7. On `keyInvalid`: mark key invalid in-memory, log warning ("API key #N is invalid — skipping for this session"), retry with next key
8. On `rateLimitExceeded`: bubble up immediately (transient, no rotation)
9. If all keys fail: return error ("All YouTube API keys exhausted. Daily quota resets at midnight Pacific Time.")

**Concurrency note**: Under concurrent requests, two tasks may both select the same key, both get `quotaExceeded`, and both mark it exhausted. This is acceptable — the outcome is correct (key gets marked), just slightly wasteful. A full mutex around key selection is unnecessary for this use case.

## Persistence

**Location**: `dirs::cache_dir() / mcp-youtube / exhausted_keys.json`
- macOS: `~/Library/Caches/mcp-youtube/exhausted_keys.json`
- Linux: `~/.cache/mcp-youtube/exhausted_keys.json`

**Format**:
```json
{
  "exhausted_keys": {
    "a1b2c3d4": "2026-03-13T15:30:00Z",
    "e5f6g7h8": "2026-03-13T18:45:00Z"
  }
}
```

- Keys identified by first 8 chars of SHA-256 hash (stable across reordering, no secrets on disk)
- Loaded on startup; written on each exhaustion event
- If file doesn't exist or is corrupt: start fresh (no error)
- `keyInvalid` state is NOT persisted — retried fresh on restart

**Auto-reset**: Keys whose exhaustion timestamp is before the most recent midnight Pacific are automatically cleared on access.

## Server Integration

- `YoutubeMcpServer` gets `key_pool: Arc<KeyPool>` field
- `api_key()` method removed
- Each individual API call is wrapped in `execute_with_key` separately:

```rust
// Simple single-call handler
self.key_pool.execute_with_key(|key| async {
    self.hub.videos().list(&parts)
        .param("key", key)
        .doit().await
        .map_err(Error::from)
}).await
```

**Multi-call handlers** (e.g. `list_channel_videos` which first resolves the uploads playlist ID, then lists videos): each API call is wrapped individually. If the second call hits quota, only it retries with a new key — the first call's result is preserved.

```rust
// First call: get uploads playlist ID
let (_, channel) = self.key_pool.execute_with_key(|key| async {
    self.hub.channels().list(&parts)
        .param("key", key)
        .doit().await
        .map_err(Error::from)
}).await?;

let uploads_id = /* extract from channel */;

// Second call: list playlist items (independently wrapped)
let (_, items) = self.key_pool.execute_with_key(|key| async {
    self.hub.playlist_items().list(&parts)
        .for_playlist(&uploads_id)
        .param("key", key)
        .doit().await
        .map_err(Error::from)
}).await?;
```

## Error Handling Changes

- New helper: `is_quota_exceeded(err: &Error) -> bool` — extracted from existing `extract_api_error_message` logic
- New helper: `is_key_invalid(err: &Error) -> bool`
- `friendly_youtube_error` updated for all-keys-exhausted case
- `rateLimitExceeded` behavior unchanged (transient, no rotation)

## New Dependencies

- `dirs` — platform-appropriate cache directory
- (`chrono` already in Cargo.toml)
