//! Response-size capping shared by every HTTP-backed connector — a
//! malicious or misconfigured source shouldn't be able to exhaust
//! memory (`docs/22-testing-strategy.md`'s "oversized response"
//! security test, `docs/14-source-connectors.md`'s "response-size
//! limit" networking control). Split out of `feed.rs` (the first
//! connector to need it) when `booru.rs` needed the identical guard.

/// Reads `response`'s body up to `max_bytes`, rejecting it as soon as
/// that limit is exceeded — checked against a declared `Content-Length`
/// up front where present, but enforced against the actual bytes
/// streamed regardless, since a server can omit or lie about that
/// header (e.g. chunked transfer-encoding).
pub async fn read_capped_body(
    mut response: reqwest::Response,
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    if let Some(len) = response.content_length() {
        if len > max_bytes {
            return Err(format!(
                "response declared {len} bytes, exceeding the {max_bytes}-byte limit"
            ));
        }
    }
    let mut buf = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        buf.extend_from_slice(&chunk);
        if buf.len() as u64 > max_bytes {
            return Err(format!(
                "response exceeded the {max_bytes}-byte limit while streaming"
            ));
        }
    }
    Ok(buf)
}
