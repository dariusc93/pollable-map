use futures::stream::FusedStream;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// A reusable stream that is the equivalent to an `Option`.
///
/// By default, this future will be empty, which would return  [`Poll::Pending`] when polled,
/// but if a [`Stream`] is supplied either upon construction via [`OptionalStream::new`] or
/// is set via [`OptionalStream::replace`], the stream could later polled once [`OptionalStream`]
/// is polled. Once the stream is polled to completion, [`OptionalStream`] will be empty.
pub struct OptionalStream<S> {
    stream: Option<S>,
    waker: Option<Waker>,
}

impl<S: Unpin> Unpin for OptionalStream<S> {}

impl<S> Default for OptionalStream<S> {
    fn default() -> Self {
        Self {
            stream: None,
            waker: None,
        }
    }
}

impl<S> From<Option<S>> for OptionalStream<S> {
    fn from(st: Option<S>) -> Self {
        Self {
            stream: st,
            waker: None,
        }
    }
}

impl<S: Stream> From<S> for OptionalStream<S> {
    fn from(st: S) -> Self {
        Self {
            stream: Some(st),
            waker: None,
        }
    }
}

impl<S> OptionalStream<S> {
    /// Constructs a new `OptionalStream` with an existing `Stream`.
    pub fn new(st: S) -> Self {
        Self {
            stream: Some(st),
            waker: None,
        }
    }

    /// Take the stream out, leaving the `OptionalStream` empty
    pub fn take(&mut self) -> Option<S> {
        let fut = self.stream.take();
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        fut
    }

    /// Returns true if stream still exist.
    pub fn is_some(&self) -> bool {
        self.stream.is_some()
    }

    /// Returns false if stream doesnt exist or has been completed.
    pub fn is_none(&self) -> bool {
        self.stream.is_none()
    }

    /// Returns reference of a stream.
    pub fn as_ref(&self) -> Option<&S> {
        self.stream.as_ref()
    }

    /// Returns mutable reference of a stream.
    pub fn as_mut(&mut self) -> Option<&mut S> {
        self.stream.as_mut()
    }

    /// Replaces the current stream with a new one, returning the previous stream if present.
    pub fn replace(&mut self, st: S) -> Option<S> {
        let fut = self.stream.replace(st);
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        fut
    }
}

impl<S> Stream for OptionalStream<S>
where
    S: Stream + Send + Unpin + 'static,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(stream) = self.stream.as_mut() else {
            self.waker.replace(cx.waker().clone());
            return Poll::Pending;
        };

        match Pin::new(stream).poll_next(cx) {
            Poll::Ready(Some(output)) => Poll::Ready(Some(output)),
            Poll::Ready(None) => {
                self.stream.take();
                Poll::Ready(None)
            }
            Poll::Pending => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.stream.as_ref() {
            Some(st) => st.size_hint(),
            None => (0, Some(0)),
        }
    }
}

impl<S> FusedStream for OptionalStream<S>
where
    S: Stream + Send + Unpin + 'static,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_none()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::StreamExt;

    #[test]
    fn test_optional_stream() {
        let mut stream = OptionalStream::new(futures::stream::once(async { 0 }).boxed());
        assert!(stream.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(Some(0)));
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(None));
        assert!(stream.is_none());
    }

    #[test]
    fn reusable_optional_stream() {
        let mut stream = OptionalStream::new(futures::stream::once(async { 0 }).boxed());
        assert!(stream.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(Some(0)));
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(None));
        assert!(stream.is_none());

        stream.replace(futures::stream::once(async { 1 }).boxed());
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(Some(1)));
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(None));
        assert!(stream.is_none());
    }

    #[test]
    fn convert_stream_to_optional_stream() {
        let st = futures::stream::once(async { 0 }).boxed();

        let mut stream = OptionalStream::from(st);

        assert!(stream.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(Some(0)));
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(None));
        assert!(stream.is_none());
    }
}
