use futures_util::{Stream, StreamExt};
use std::pin::pin;
use tokio::sync::mpsc;

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
