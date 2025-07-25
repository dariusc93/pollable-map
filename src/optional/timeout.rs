use crate::common::Timed;
use crate::optional::Optional;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use futures::Stream;

/// A reusable future or stream based on `Option` that will time out after a specific duration as elapse.
pub struct TimeoutOptional<T> {
    duration: Duration,
    task: Optional<Timed<T>>,
}

impl<T> Deref for TimeoutOptional<T> {
    type Target = Optional<Timed<T>>;
    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl<T> DerefMut for TimeoutOptional<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.task
    }
}

impl<T> TimeoutOptional<T> {
    /// Construct a new [`TimeoutOptional`].
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            task: Optional::default(),
        }
    }

    /// Construct a new [`TimeoutOptional`] with an existing [`Future`] or [`Stream`].
    pub fn new_with_task(duration: Duration, task: T) -> Self {
        Self {
            duration,
            task: Optional::new(Timed::new(task, duration)),
        }
    }

    /// Construct a new [`TimeoutOptional`] with an existing [`Future`].
    pub fn new_with_future(duration: Duration, task: T) -> Self
    where
        T: Future + Unpin,
    {
        Self {
            duration,
            task: Optional::with_future(Timed::new(task, duration)),
        }
    }

    /// Construct a new [`TimeoutOptional`] with an existing [`Stream`].
    pub fn new_with_stream(duration: Duration, task: T) -> Self
    where
        T: Stream + Unpin,
    {
        Self {
            duration,
            task: Optional::with_stream(Timed::new(task, duration)),
        }
    }

    /// Replaces the current the future or stream with a new one, returning the previous value if present.
    pub fn replace(&mut self, task: T) -> Option<T> {
        let prev = self.task.replace(Timed::new(task, self.duration));
        prev.map(|item| item.into_inner())
    }
}

impl<T: Future + Unpin> Future for TimeoutOptional<T> {
    type Output = std::io::Result<T::Output>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.task).poll(cx)
    }
}

impl<T: Stream + Unpin> Stream for TimeoutOptional<T> {
    type Item = std::io::Result<T::Item>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.task).poll_next(cx)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::future::ready;
    use std::future::pending;
    use std::time::Duration;

    #[test]
    fn test_timeout_optional_ready() {
        let mut task = TimeoutOptional::new_with_task(Duration::from_secs(1), ready(()));
        futures::executor::block_on(async move {
            let fut = Pin::new(&mut task);
            match fut.await {
                Ok(_) => assert!(task.is_none()),
                Err(e) => panic!("unexpected error: {e}"),
            }
        })
    }

    #[test]
    fn test_timeout_optional_timeout() {
        let mut task = TimeoutOptional::new_with_task(Duration::from_millis(10), pending::<()>());

        futures::executor::block_on(async move {
            let fut = Pin::new(&mut task);
            match fut.await {
                Ok(_) => unreachable!("should time out"),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    assert!(task.is_none());
                }
                Err(e) => panic!("unexpected error: {e}"),
            }
        })
    }
}
