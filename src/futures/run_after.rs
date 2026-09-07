use crate::optional::timeout::TimeoutOptional;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;

#[pin_project::pin_project]
pub struct RunAfter<F> {
    #[pin]
    f: F,
    timer: TimeState,
}

enum TimeState {
    Init {
        duration: Duration,
    },
    Timer {
        timer: TimeoutOptional<futures::future::Pending<()>>,
    },
    Done,
}

impl TimeState {
    pub fn is_done(&self) -> bool {
        matches!(self, TimeState::Done)
    }
}

impl core::future::Future for TimeState {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        loop {
            match this {
                TimeState::Init { ref duration } => {
                    *this = TimeState::Timer {
                        timer: TimeoutOptional::new_with_future(
                            *duration,
                            futures::future::pending(),
                        ),
                    };
                }
                TimeState::Timer { ref mut timer } => {
                    let _ = core::task::ready!(Pin::new(timer).poll(cx));
                    *this = TimeState::Done;
                }
                TimeState::Done => return Poll::Ready(()),
            }
        }
    }
}

impl<F> RunAfter<F> {
    pub fn new(duration: Duration, f: F) -> Self {
        Self {
            timer: TimeState::Init { duration },
            f,
        }
    }
}

impl<F> core::future::Future for RunAfter<F>
where
    F: core::future::Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        if !this.timer.is_done() {
            core::task::ready!(Pin::new(&mut this.timer).poll(cx));
        }

        this.f.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::future::ready;

    #[test]
    fn test_run_after() {
        futures::executor::block_on(async move {
            let val = RunAfter::new(Duration::from_millis(50), ready(42)).await;
            assert_eq!(val, 42)
        });
    }

    #[test]
    fn test_run_after_timer_doesnt_start_until_poll() {
        let delay = Duration::from_millis(50);
        let future = RunAfter::new(delay, ready(42));

        std::thread::sleep(delay * 2);

        let start = std::time::Instant::now();
        assert_eq!(futures::executor::block_on(future), 42);
        assert!(start.elapsed() >= delay);
    }
}
