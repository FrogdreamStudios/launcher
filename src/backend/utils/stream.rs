//! Lightweight stream utilities for reqwest.

pub struct ChunkStream {
    response: reqwest::Response,
}

impl ChunkStream {
    /// Create a new chunk stream from a reqwest `Response`.
    pub fn new(response: reqwest::Response) -> Self {
        Self { response }
    }

    /// Get the next chunk asynchronously.
    pub async fn next(&mut self) -> Option<Result<Vec<u8>, reqwest::Error>> {
        match self.response.chunk().await {
            Ok(Some(chunk)) => Some(Ok(chunk.to_vec())),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Extension trait for easier chunk streaming.
///
/// This trait extends `reqwest::Response` with a simple method to create
/// a chunk stream for reading response data without requiring futures-util.
pub trait ResponseChunkExt {
    /// Convert our response into a chunk stream.
    fn chunk_stream(self) -> ChunkStream;
}

impl ResponseChunkExt for reqwest::Response {
    fn chunk_stream(self) -> ChunkStream {
        ChunkStream::new(self)
    }
}
