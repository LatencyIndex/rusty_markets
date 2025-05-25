use async_stream::stream;
use futures_util::{Stream, StreamExt};
use std::pin::pin;
use tokio::sync::mpsc;
use tokio::time::error::Elapsed;
use tokio::time::{timeout, Duration};

/// Redirect the stream output into the mpsc channel.
fn splice_stream<T, S>(stream: S, tx: mpsc::Sender<T>)
where
    T: Send + 'static,
    S: Stream<Item = T> + Send + 'static,
{
    tokio::spawn(async move {
        let mut stream = pin!(stream);
        while let Some(msg) = stream.next().await {
            if tx.send(msg).await.is_err() {
                break;
            };
        }
    });
}

/// Create a mpsc channel that yields the merged output of the streams.
pub fn splice<T, S, I>(streams: I) -> mpsc::Receiver<T>
where
    T: Send + 'static,
    S: Stream<Item = T> + Send + 'static,
    I: IntoIterator<Item = S>,
{
    let (tx, rx) = mpsc::channel(8);
    for stream in streams {
        splice_stream(stream, tx.clone());
    }
    rx
}

/// If no message is received for more than 'lifetime', sends an Err<Elapsed>
/// This notifies the recipient that the validity of the last message has expired.
/// Sends at most one expiry notification per message.
/// I.e. never sends consecutive expiry notifications.
pub fn expire<T>(
    lifetime: Duration,
    stream: impl Stream<Item = T>,
) -> impl Stream<Item = Result<T, Elapsed>> {
    stream! {
        let mut stream = pin!(stream);
        // Transmit first message normally, since there is nothing to expire yet.
        if let Some(msg) = stream.next().await {
            yield Ok(msg);
        }
        loop {
            match timeout(lifetime, stream.next()).await {
                Ok(Some(msg)) => yield Ok(msg),
                Ok(None) => break,
                Err(elapsed) => {
                    yield Err(elapsed);
                    // Previous message was already expired, so we await next one without timeout,
                    // since we don't have to send any more expiry events.
                    match stream.next().await {
                        Some(msg) => yield Ok(msg),
                        None => break,
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_stream::stream;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn splice_test() {
        fn into_stream(r: Vec<i32>) -> impl Stream<Item = i32> {
            stream! {
                for i in r {
                    sleep(Duration::from_millis(30)).await;
                    yield i;
                }
            }
        }
        let s1 = into_stream(vec![1, 3, 5]);
        let s2 = into_stream(vec![2, 4, 6]);
        let mut s = splice([s1, s2].into_iter());
        let mut v = Vec::new();
        while let Some(i) = s.recv().await {
            v.push(i);
        }
        // Check that the streams were not chained, but somewhat intermixed.
        assert_ne!(v, [1, 3, 5, 2, 4, 6]);
        assert_ne!(v, [2, 4, 6, 1, 3, 5]);
        // Check that v contains the elements of each stream, in order.
        let (even, odd): (Vec<i32>, Vec<i32>) = v.iter().copied().partition(|i| i % 2 == 0);
        assert_eq!(odd, [1, 3, 5]);
        assert_eq!(even, [2, 4, 6]);
        // Check that v got all and only the elements of both streams:
        v.sort();
        assert_eq!(v, [1, 2, 3, 4, 5, 6]);
    }
}
