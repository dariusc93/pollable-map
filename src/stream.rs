use crate::common::InnerMap;
use futures::stream::{FusedStream, SelectAll};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// Combining multiple streams into one, with each stream having a unique key.
pub struct StreamMap<K, S> {
    list: SelectAll<InnerMap<K, S>>,
    empty: bool,
    waker: Option<Waker>,
}

impl<K, T> Default for StreamMap<K, T>
where
    K: Clone + Unpin,
    T: Stream + Send + Unpin + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T> StreamMap<K, T>
where
    K: Clone + Unpin,
    T: Stream + Send + Unpin + 'static,
{
    /// Creates an empty [`StreamMap`]
    pub fn new() -> Self {
        Self {
            list: SelectAll::new(),
            empty: true,
            waker: None,
        }
    }
}

impl<K, T> StreamMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Stream + Send + Unpin + 'static,
{
    /// Insert a stream into the map with a unique key.
    /// The function will return true if the map does not have the key present,
    /// otherwise it will return false
    pub fn insert(&mut self, key: K, stream: T) -> bool {
        if self.contains_key(&key) {
            return false;
        }

        let st = InnerMap::new(key, stream, false);
        self.list.push(st);

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }

        self.empty = false;
        true
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &T)> {
        self.list.iter().filter_map(|st| st.key_value())
    }

    /// An iterator visiting all key-value pairs mutably in arbitrary order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut T)> {
        self.list.iter_mut().filter_map(|st| st.key_value_mut())
    }

    /// Returns an iterator visiting all keys in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.list.iter().map(|st| st.key())
    }

    /// An iterator visiting all values in arbitrary order.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.list.iter().filter_map(|st| st.inner())
    }

    /// An iterator visiting all values mutably in arbitrary order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.list.iter_mut().filter_map(|st| st.inner_mut())
    }

    /// Returns `true` if the map contains a stream for the specified key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.list.iter().any(|st| st.key().eq(key))
    }

    /// Clears the map.
    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// Returns a reference to the stream corresponding to the key.
    pub fn get(&self, key: &K) -> Option<&T> {
        let st = self.list.iter().find(|st| st.key().eq(key))?;
        st.inner()
    }

    /// Returns a mutable stream to the value corresponding to the key.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut T> {
        let st = self.list.iter_mut().find(|st| st.key().eq(key))?;
        st.inner_mut()
    }

    /// Removes a key from the map, returning the stream.
    pub fn remove(&mut self, key: &K) -> Option<T> {
        let st = self.list.iter_mut().find(|st| st.key().eq(key))?;
        st.take_inner()
    }

    /// Returns the number of streams in the map.
    pub fn len(&self) -> usize {
        self.list.iter().filter(|st| st.inner().is_some()).count()
    }

    /// Return `true` map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty() || self.list.iter().all(|st| st.inner().is_none())
    }
}

impl<K, T> Stream for StreamMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Stream + Unpin + Send + 'static,
{
    type Item = (K, T::Item);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        if this.list.is_empty() {
            self.waker = Some(cx.waker().clone());
            return Poll::Pending;
        }

        loop {
            match this.list.poll_next_unpin(cx) {
                Poll::Ready(Some((key, Some(item)))) => return Poll::Ready(Some((key, item))),
                // We continue in case there is any progress on the set of streams
                Poll::Ready(Some((key, None))) => {
                    this.remove(&key);
                }
                Poll::Ready(None) => {
                    // While we could allow the stream to continue to be pending, it would make more sense to notify that the stream
                    // is empty without needing to explicitly check while polling the actual "map" itself
                    // So we would mark a field to notify that the state is finished and return `Poll::Ready(None)` so the stream
                    // can be terminated while on the next poll, we could let it be return pending.
                    // We do this so that we are not returning `Poll::Ready(None)` each time the map is polled
                    // as that may be seen as UB and may cause an increase in cpu usage
                    if self.empty {
                        self.waker = Some(cx.waker().clone());
                        return Poll::Pending;
                    }

                    self.empty = true;
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    // Returning `None` does not mean the stream is actually terminated
                    self.waker = Some(cx.waker().clone());
                    return Poll::Pending;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.list.size_hint()
    }
}

impl<K, T> FusedStream for StreamMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Stream + Unpin + Send + 'static,
{
    fn is_terminated(&self) -> bool {
        self.list.is_terminated()
    }
}
