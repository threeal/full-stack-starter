//! The example click counter's calls and synced objects.

use serde_json::Value;

use crate::socket::{Error, Service, Synced, Updates};

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
