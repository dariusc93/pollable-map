use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use crate::common::InnerMap;

pub struct FutureMap<K, S> {
    list: FuturesUnordered<InnerMap<K, S>>,
    waker: Option<Waker>,
}

impl<K, T> FutureMap<K, T>
where
    K: Clone + Unpin,
    T: Stream + Send + Unpin + 'static,
{
    pub fn new() -> Self {
        Self {
            list: FuturesUnordered::new(),
            waker: None,
        }
    }
}

impl<K, T> FutureMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Future + Send + Unpin + 'static,
{
    pub fn insert(&mut self, key: K, fut: T) -> bool {
        if self.contains_key(&key) {
            return false;
        }

        let st = InnerMap::new(key, fut);
        self.list.push(st);

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }

        true
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &T)> {
        self.list.iter().filter_map(|st| st.key_value())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (K, &mut T)> {
        self.list.iter_mut().filter_map(|st| st.key_value_mut())
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.list.iter().map(|st| st.key())
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.list.iter().filter_map(|st| st.as_ref())
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.list.iter_mut().filter_map(|st| st.as_mut())
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.list.iter().any(|st| st.key().eq(key))
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn get(&self, key: &K) -> Option<&T> {
        let st = self.list.iter().find(|st| st.key().eq(key))?;
        st.as_ref()
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut T> {
        let st = self.list.iter_mut().find(|st| st.key().eq(key))?;
        st.as_mut()
    }

    pub fn remove(&mut self, key: &K) -> Option<T> {
        let st = self.list.iter_mut().find(|st| st.key().eq(key))?;
        let inner = st.take_inner();
        inner
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl<K, T> Stream for FutureMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Future + Unpin + Send + 'static,
{
    type Item = (K, T::Output);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        loop {
            match this.list.poll_next_unpin(cx) {
                Poll::Ready(Some((key, Some(item)))) => return Poll::Ready(Some((key, item))),
                Poll::Ready(Some((key, None))) => {
                    this.remove(&key);
                    // We continue in case there is any progress on the set of streams
                    continue;
                }
                Poll::Ready(None) | Poll::Pending => {
                    // Returning `None` does not mean the stream is actually terminated
                    self.waker = Some(cx.waker().clone());
                    return Poll::Pending;
                }
            }
        }
    }
}