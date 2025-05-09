use crate::common::Timed;
use crate::stream::StreamMap;
use futures::stream::FusedStream;
use futures::{Stream, StreamExt};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub struct TimeoutStreamMap<K, S> {
    duration: Duration,
    map: StreamMap<K, Timed<S>>,
}

impl<K, S> Deref for TimeoutStreamMap<K, S> {
    type Target = StreamMap<K, Timed<S>>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<K, S> DerefMut for TimeoutStreamMap<K, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<K, S> TimeoutStreamMap<K, S>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    S: Stream + Send + Unpin + 'static,
{
    /// Create an empty [`TimeoutStreamMap`]
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            map: StreamMap::new(),
        }
    }

    /// Insert a stream into the map with a unique key.
    /// The function will return true if the map does not have the key present,
    /// otherwise it will return false
    pub fn insert(&mut self, key: K, stream: S) -> bool {
        self.map.insert(key, Timed::new(stream, self.duration))
    }
}

impl<K, S> Stream for TimeoutStreamMap<K, S>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    S: Stream + Send + Unpin + 'static,
{
    type Item = (K, std::io::Result<S::Item>);
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.map.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.map.size_hint()
    }
}

impl<K, S> FusedStream for TimeoutStreamMap<K, S>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    S: Stream + Send + Unpin + 'static,
{
    fn is_terminated(&self) -> bool {
        self.map.is_terminated()
    }
}
