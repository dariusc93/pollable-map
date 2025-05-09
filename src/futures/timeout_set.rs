use crate::common::Timed;
use crate::futures::set::FutureSet;
use futures::stream::FusedStream;
use futures::{Stream, StreamExt};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub struct TimeoutFutureSet<S> {
    duration: Duration,
    set: FutureSet<Timed<S>>,
}

impl<S> Deref for TimeoutFutureSet<S> {
    type Target = FutureSet<Timed<S>>;
    fn deref(&self) -> &Self::Target {
        &self.set
    }
}

impl<S> DerefMut for TimeoutFutureSet<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.set
    }
}

impl<F> TimeoutFutureSet<F>
where
    F: Future + Send + Unpin + 'static,
{
    /// Create an empty [`TimeoutFutureSet`]
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            set: FutureSet::new(),
        }
    }

    /// Insert a future into the set of futures.
    pub fn insert(&mut self, future: F) -> bool {
        self.set.insert(Timed::new(future, self.duration))
    }
}

impl<F> Stream for TimeoutFutureSet<F>
where
    F: Future + Send + Unpin + 'static,
{
    type Item = std::io::Result<F::Output>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.set.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.set.size_hint()
    }
}

impl<F> FusedStream for TimeoutFutureSet<F>
where
    F: Future + Send + Unpin + 'static,
{
    fn is_terminated(&self) -> bool {
        self.set.is_terminated()
    }
}
