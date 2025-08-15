//! Lightweight stream utilities.

use crate::backend::utils::net::http::Response;

/// Stream implementation.
pub struct ChunkStream {
    response: Response,
    finished: bool,
}

impl ChunkStream {
    /// Create a new chunk stream from our custom HTTP Response.
    pub fn new(response: Response) -> Self {
        Self {
            response,
            finished: false,
        }
    }

    /// Get the next chunk asynchronously.
    pub fn next(&mut self) -> Option<Result<Vec<u8>, String>> {
        if self.finished {
            return None;
        }

        match self.response.chunk() {
            Ok(Some(chunk)) => Some(Ok(chunk)),
            Ok(None) => {
                self.finished = true;
                None
            }
            Err(e) => {
                self.finished = true;
                Some(Err(format!("Chunk read error: {e}")))
            }
        }
    }
}

/// Extension trait for easier chunk streaming.
///
/// This trait extends `Response` with a simple method to create
/// a chunk stream for reading response data.
pub trait ResponseChunkExt {
    /// Convert our response into a chunk stream.
    fn chunk_stream(self) -> ChunkStream;
}

impl ResponseChunkExt for Response {
    fn chunk_stream(self) -> ChunkStream {
        ChunkStream::new(self)
    }
}
