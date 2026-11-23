//! The calls and synced objects that an app serves.

use serde_json::Value;

use super::synced::Updates;

/// The calls and synced objects that an app serves to its pages.
pub trait Service: Send + Sync + 'static {
    /// Handles a call to `method` with `params`, returning its result.
    fn call(
        &self,
        method: &str,
        params: &Value,
    ) -> impl Future<Output = Result<Value, Error>> + Send;

    /// Returns the updates of the synced object at `key`, or `None` if there's
    /// no such object.
    fn subscribe(&self, key: &str) -> Option<Updates>;
}

/// An error that a call replies with.
#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    /// A machine-readable code identifying the kind of error.
    pub(super) code: &'static str,
    /// A human-readable description of the error.
    pub(super) message: String,
}

impl Error {
    /// Creates an error with a machine-readable `code` and a human-readable
    /// `message`.
    #[must_use]
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
