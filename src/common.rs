use futures::{FutureExt, Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct InnerMap<K, S> {
    key: K,
    inner: Option<S>,
}

impl<K, S> InnerMap<K, S> {
    pub fn new(key: K, inner: S) -> Self {
        Self {
            key,
            inner: Some(inner),
        }
    }
}

impl<K, S> InnerMap<K, S>
where
    K: Unpin + Clone,
    S: Unpin,
{
    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn key_value(&self) -> Option<(&K, &S)> {
        self.as_ref().map(|st| (self.key(), st))
    }

    pub fn key_value_mut(&mut self) -> Option<(K, &mut S)> {
        let key = self.key.clone();
        self.as_mut().map(|st| (key, st))
    }

    pub fn as_ref(&self) -> Option<&S> {
        self.inner.as_ref()
    }

    pub fn as_mut(&mut self) -> Option<&mut S> {
        self.inner.as_mut()
    }

    pub fn take_inner(&mut self) -> Option<S> {
        self.inner.take()
    }
}

impl<K, S> Future for InnerMap<K, S>
where
    K: Clone + Unpin,
    S: Future + Unpin,
{
    type Output = (K, Option<S::Output>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        let Some(st) = this.inner.as_mut() else {
            return Poll::Ready((this.key.clone(), None));
        };

        let output = futures::ready!(st.poll_unpin(cx));
        this.inner.take();
        Poll::Ready((this.key.clone(), Some(output)))
    }
}

impl<K, S> Stream for InnerMap<K, S>
where
    K: Clone + Unpin,
    S: Stream + Unpin,
{
    type Item = (K, Option<S::Item>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        let Some(st) = this.inner.as_mut() else {
            return Poll::Ready(None);
        };

        match futures::ready!(st.poll_next_unpin(cx)) {
            Some(value) => {
                cx.waker().wake_by_ref();
                Poll::Ready(Some((this.key.clone(), Some(value))))
            },
            None => {
                this.inner.take();
                Poll::Ready(Some((self.key.clone(), None)))
            }
        }
    }
}
