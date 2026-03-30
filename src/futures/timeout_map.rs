use crate::common::Timed;
use crate::futures::FutureMap;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use futures::stream::FusedStream;
use futures::{Stream, StreamExt};

pub struct TimeoutFutureMap<K, F> {
    duration: Duration,
    map: FutureMap<K, Timed<F>>,
}

impl<K, F> Deref for TimeoutFutureMap<K, F> {
    type Target = FutureMap<K, Timed<F>>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<K, F> DerefMut for TimeoutFutureMap<K, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<K, F> TimeoutFutureMap<K, F>
where
    K: Clone + PartialEq + Unpin,
    F: Future + Unpin,
{
    /// Create an empty [`TimeoutFutureMap`]
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            map: FutureMap::new(),
        }
    }

    /// Insert a future into the map with a unique key.
    /// The function will return true if the map does not have the key present,
    /// otherwise it will return false
    pub fn insert(&mut self, key: K, future: F) -> bool {
        self.map.insert(key, Timed::new(future, self.duration))
    }
}

impl<K, F> Stream for TimeoutFutureMap<K, F>
where
    K: Clone + PartialEq + Unpin,
    F: Future + Unpin,
{
    type Item = (K, std::io::Result<F::Output>);
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.map.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.map.size_hint()
    }
}

impl<K, F> FusedStream for TimeoutFutureMap<K, F>
where
    K: Clone + PartialEq + Unpin,
    F: Future + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.map.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use crate::futures::timeout_map::TimeoutFutureMap;
    use futures::StreamExt;
    use std::time::Duration;

    #[test]
    fn timeout_map() {
        let mut list = TimeoutFutureMap::new(Duration::from_millis(100));
        assert!(list.insert(0, futures::future::pending::<()>()));

        futures::executor::block_on(async move {
            let result = list.next().await;
            let Some((0, Err(e))) = result else {
                unreachable!("result is err");
            };

            assert_eq!(e.kind(), std::io::ErrorKind::TimedOut);
        });
    }

    #[test]
    fn valid_stream() {
        let mut list = TimeoutFutureMap::new(Duration::from_secs(10));
        assert!(list.insert(1, futures::future::ready(0)));

        futures::executor::block_on(async move {
            let result = list.next().await;
            let Some((1, Ok(val))) = result else {
                unreachable!("result is err");
            };

            assert_eq!(val, 0);
        });
    }
}
