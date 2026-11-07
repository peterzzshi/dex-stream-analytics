use std::fmt;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retriability {
    Retriable,
    Fatal,
}

/// Retry an operation with exponential backoff, using a classifier to decide retriability.
///
/// The classifier receives the error by reference and returns whether to retry. This
/// preserves the typed error for matching (unlike `anyhow::Error` erasure) while staying
/// generic over error types.
pub async fn retry_with_classifier<F, Fut, T, E, C>(
    max_retries: u32,
    initial_backoff: Duration,
    mut classifier: C,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: fmt::Display,
    C: FnMut(&E) -> Retriability,
{
    let mut attempt = 0;
    let mut backoff = initial_backoff;

    loop {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("Operation succeeded after {} retries", attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                if classifier(&e) == Retriability::Fatal || attempt >= max_retries {
                    return Err(e);
                }

                warn!(
                    "Operation failed (attempt {}/{}): {}. Retrying in {}ms",
                    attempt + 1,
                    max_retries,
                    e,
                    backoff.as_millis()
                );

                sleep(backoff).await;
                attempt += 1;
                backoff = (backoff * 2).min(Duration::from_secs(10));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[derive(Debug)]
    struct TestError {
        retriable: bool,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "test error")
        }
    }

    impl std::error::Error for TestError {}

    #[tokio::test(start_paused = true)]
    async fn retries_retriable_errors_until_success() {
        let calls = Cell::new(0);
        let result: Result<u32, TestError> = retry_with_classifier(
            3,
            Duration::from_millis(100),
            |e: &TestError| {
                if e.retriable {
                    Retriability::Retriable
                } else {
                    Retriability::Fatal
                }
            },
            || {
                let count = calls.get();
                calls.set(count + 1);
                async move {
                    if count < 2 {
                        Err(TestError { retriable: true })
                    } else {
                        Ok(42)
                    }
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.get(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_after_max_retries() {
        let calls = Cell::new(0);
        let result: Result<(), TestError> = retry_with_classifier(
            2,
            Duration::from_millis(100),
            |_| Retriability::Retriable,
            || {
                calls.set(calls.get() + 1);
                async { Err(TestError { retriable: true }) }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(calls.get(), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn fatal_errors_are_not_retried() {
        let calls = Cell::new(0);
        let result: Result<(), TestError> = retry_with_classifier(
            3,
            Duration::from_millis(100),
            |_| Retriability::Fatal,
            || {
                calls.set(calls.get() + 1);
                async { Err(TestError { retriable: false }) }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(calls.get(), 1);
    }
}
