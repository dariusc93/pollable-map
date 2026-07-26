#[cfg(feature = "timeout")]
#[derive(Debug, PartialEq, Eq)]
pub struct TimedError;

#[cfg(feature = "timeout")]
impl core::fmt::Display for TimedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Operation timed out")
    }
}

#[cfg(feature = "timeout")]
impl core::error::Error for TimedError {}
