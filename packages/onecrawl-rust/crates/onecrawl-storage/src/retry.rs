//! Retry logic for storage operations.

use std::time::Duration;
use std::thread;

/// Retry a fallible operation with exponential backoff.
pub fn retry_with_backoff<T, E, F>(
    max_retries: u32,
    initial_delay_ms: u64,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut delay = Duration::from_millis(initial_delay_ms);

    for attempt in 0..max_retries {
        match operation() {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempt == max_retries - 1 {
                    return Err(e);
                }
                thread::sleep(delay);
                delay *= 2;
            }
        }
    }

    // Should never reach here, but satisfy compiler
    operation()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn retry_succeeds_first_try() {
        let result = retry_with_backoff(3, 10, || Ok::<_, &str>("success"));
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn retry_succeeds_after_failures() {
        let attempts = AtomicU32::new(0);
        let result = retry_with_backoff(3, 10, || {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            if n < 2 { Err("not yet") } else { Ok("finally") }
        });
        assert_eq!(result.unwrap(), "finally");
    }

    #[test]
    fn retry_exhausted() {
        let result = retry_with_backoff(2, 10, || Err::<(), _>("always fails"));
        assert!(result.is_err());
    }
}
