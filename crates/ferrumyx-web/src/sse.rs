//! Server-Sent Events (SSE) streaming for real-time UI updates.

use axum::response::sse::{Event, KeepAlive, Sse};
use axum::extract::State;
use futures_core::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::SharedState;

/// SSE endpoint — clients subscribe here for real-time updates.
pub async fn sse_handler(
    State(state): State<SharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            result.ok().and_then(|event| {
                serde_json::to_string(&event).ok().map(|data| {
                    Ok(Event::default().data(data))
                })
            })
        });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}
