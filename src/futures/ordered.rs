use futures::Stream;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// An unbounded queue of futures imposed a FIFO order while polling one future at a time
/// and returning the output to stream before popping the next future in queue to be polled.
pub struct OrderedFutureSet<F> {
    queue: VecDeque<F>,
    current_future: Option<F>,
    waker: Option<Waker>,
}

impl<F> OrderedFutureSet<F>
where
    F: Future + Send + Unpin + 'static,
{
    /// Constructs a new, empty [`OrderedFutureSet`]
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current_future: None,
            waker: None,
        }
    }

    /// Furshes a future to the back of the queue
    pub fn push(&mut self, fut: F) {
        self.queue.push_back(fut);
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

impl<F> Stream for OrderedFutureSet<F>
where
    F: Future + Send + Unpin + 'static,
{
    type Item = F::Output;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        loop {
            if this.current_future.is_none() {
                let Some(fut) = this.queue.pop_front() else {
                    break;
                };
                this.current_future.replace(fut);
            }

            match this.current_future.as_mut() {
                Some(fut) => {
                    let output = futures::ready!(Pin::new(fut).poll(cx));
                    this.current_future.take();
                    cx.waker().wake_by_ref();
                    return Poll::Ready(Some(output));
                }
                None => {
                    this.waker.replace(cx.waker().clone());
                }
            }
        }

        this.waker.replace(cx.waker().clone());
        Poll::Pending
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.queue.len(), None)
    }
}

#[cfg(test)]
mod tests {
    use crate::futures::ordered::OrderedFutureSet;
    use futures::{FutureExt, StreamExt};

    #[test]
    fn fifo_futures() {
        futures::executor::block_on(async move {
            let mut fifo = OrderedFutureSet::new();
            fifo.push(futures::future::ready(1));
            fifo.push(futures::future::ready(2));
            fifo.push(futures::future::ready(4));
            fifo.push(futures::future::ready(3));

            let items = fifo.take(4).collect::<Vec<u8>>().now_or_never().unwrap();

            assert_eq!(items, vec![1, 2, 4, 3]);
        });
    }
}
