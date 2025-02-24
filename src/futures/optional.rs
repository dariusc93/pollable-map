use futures::future::FusedFuture;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// A reusable future that is the equivalent to an `Option`.
///
/// By default, this future will be empty, which would return  [`Poll::Pending`] when polled,
/// but if a [`Future`] is supplied either upon construction via [`OptionalFuture::new`] or
/// is set via [`OptionalFuture::replace`], the future would then be polled once [`OptionalFuture`]
/// is polled. Once the future is polled to completion, the results will be returned, with
/// [`OptionalFuture`] being empty.
pub struct OptionalFuture<F> {
    future: Option<F>,
    waker: Option<Waker>,
}

impl<F: Unpin> Unpin for OptionalFuture<F> {}

impl<F> Default for OptionalFuture<F> {
    fn default() -> Self {
        Self {
            future: None,
            waker: None,
        }
    }
}

impl<F> From<Option<F>> for OptionalFuture<F> {
    fn from(fut: Option<F>) -> Self {
        Self {
            future: fut,
            waker: None,
        }
    }
}

impl<F: Future> From<F> for OptionalFuture<F> {
    fn from(fut: F) -> Self {
        Self {
            future: Some(fut),
            waker: None,
        }
    }
}

impl<F> OptionalFuture<F> {
    /// Construct a new `OptionalFuture` with an existing `Future`.
    pub fn new(future: F) -> Self {
        Self {
            future: Some(future),
            waker: None,
        }
    }

    /// Takes the future out, leaving the OptionalFuture empty.
    pub fn take(&mut self) -> Option<F> {
        let fut = self.future.take();
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        fut
    }

    /// Returns true if future still exist.
    pub fn is_some(&self) -> bool {
        self.future.is_some()
    }

    /// Returns false if future doesnt exist or has been completed.
    pub fn is_none(&self) -> bool {
        self.future.is_none()
    }

    /// Returns reference of the future.
    pub fn as_ref(&self) -> Option<&F> {
        self.future.as_ref()
    }

    /// Returns mutable reference of the future.
    pub fn as_mut(&mut self) -> Option<&mut F> {
        self.future.as_mut()
    }

    /// Replaces the current future with a new one, returning the previous future if present.
    pub fn replace(&mut self, future: F) -> Option<F> {
        let fut = self.future.replace(future);
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        fut
    }
}

impl<F> Future for OptionalFuture<F>
where
    F: Future + Send + Unpin + 'static,
{
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let Some(future) = self.future.as_mut() else {
            self.waker.replace(cx.waker().clone());
            return Poll::Pending;
        };

        match Pin::new(future).poll(cx) {
            Poll::Ready(output) => {
                self.future.take();
                Poll::Ready(output)
            }
            Poll::Pending => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl<F: Future> FusedFuture for OptionalFuture<F>
where
    F: Future + Send + Unpin + 'static,
{
    fn is_terminated(&self) -> bool {
        self.future.is_none()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_optional_future() {
        let mut future = OptionalFuture::new(futures::future::ready(0));
        assert!(future.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut future).poll(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(0));
        assert!(future.is_none());
    }

    #[test]
    fn reusable_optional_future() {
        let mut future = OptionalFuture::new(futures::future::ready(0));
        assert!(future.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut future).poll(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(0));
        assert!(future.is_none());

        future.replace(futures::future::ready(1));
        assert!(future.is_some());

        let val = Pin::new(&mut future).poll(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(1));
        assert!(future.is_none());
    }

    #[test]
    fn convert_future_to_optional_future() {
        let fut = futures::future::ready(0);

        let mut future = OptionalFuture::from(fut);
        assert!(future.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut future).poll(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(0));
        assert!(future.is_none());
    }
}
