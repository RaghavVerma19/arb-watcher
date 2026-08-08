use std::future::Future;
use std::time::Duration;

use rand::Rng;

pub async fn with_backoff<F, Fut, T, E>(
    mut op: F,
    max_retries: u32,
    base_delay: Duration,
    is_retryable: impl Fn(&E) -> bool,
) -> Result<T, E>
where
    F: FnMut() -> Fut + Send + Sync,
    Fut: Future<Output = Result<T, E>> + Send,
    T: Send,
    E: Send + std::fmt::Debug,
{
    let mut attempt = 0;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(err) if attempt >= max_retries || !is_retryable(&err) => return Err(err),
            Err(err) => {
                attempt += 1;
                let base_ms = base_delay.as_millis() as u64;
                let jitter_ms = rand::thread_rng().gen_range(0..base_ms.max(1));
                let delay = Duration::from_millis(base_ms * attempt as u64 + jitter_ms);
                eprintln!(
                    "retry {attempt}/{max_retries} after {:?}: {:?}",
                    delay, err
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn retries_then_succeeds() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let res: Result<&'static str, String> = with_backoff(
            move || {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        Err(format!("fail {n}"))
                    } else {
                        Ok("ok")
                    }
                }
            },
            5,
            Duration::from_millis(1),
            |_| true,
        )
        .await;
        assert_eq!(res.unwrap(), "ok");
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn gives_up_after_max_retries() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let res: Result<(), String> = with_backoff(
            move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err("always fail".to_string())
                }
            },
            2,
            Duration::from_millis(1),
            |_| true,
        )
        .await;
        assert!(res.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 1 initial + 2 retries
    }

    #[tokio::test]
    async fn non_retryable_error_returns_immediately() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let res: Result<(), String> = with_backoff(
            move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err("fatal".to_string())
                }
            },
            5,
            Duration::from_millis(1),
            |_| false,
        )
        .await;
        assert!(res.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
