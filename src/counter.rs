//! The example click counter's calls and synced objects.

use serde_json::Value;

use crate::live::{Error, Service, Synced, Updates};

/// The example click counter, synced as `counter` and incremented by the
/// `counter.increment` call.
pub struct Counter {
    /// The current count.
    count: Synced<u64>,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            count: Synced::new(0),
        }
    }
}

impl Service for Counter {
    fn call(
        &self,
        method: &str,
        _params: &Value,
    ) -> impl Future<Output = Result<Value, Error>> + Send {
        let result = if method == "counter.increment" {
            self.count.modify(|count| *count += 1);
            Ok(Value::Null)
        } else {
            Err(Error::new(
                "unknown_method",
                format!("no method named {method}"),
            ))
        };
        std::future::ready(result)
    }

    fn subscribe(&self, key: &str) -> Option<Updates> {
        (key == "counter").then(|| self.count.subscribe())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use serde_json::json;

    #[tokio::test]
    async fn increment_synced_count() {
        let counter = Counter::default();
        let mut updates = counter.subscribe("counter").unwrap();
        let snapshot = updates.next().await.unwrap();
        assert_eq!((snapshot.version, snapshot.value), (0, json!(0)));
        let result = counter.call("counter.increment", &Value::Null).await;
        assert_eq!(result, Ok(Value::Null));
        let snapshot = updates.next().await.unwrap();
        assert_eq!((snapshot.version, snapshot.value), (1, json!(1)));
    }

    #[tokio::test]
    async fn reject_unknown_method() {
        let counter = Counter::default();
        let result = counter.call("nope", &Value::Null).await;
        let error = Error::new("unknown_method", "no method named nope");
        assert_eq!(result, Err(error));
    }

    #[test]
    fn have_no_other_synced_objects() {
        let counter = Counter::default();
        assert!(counter.subscribe("nope").is_none());
    }
}
