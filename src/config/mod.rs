use std::time::Duration;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ViewTransferConfig {
    pub timeout_duration: Duration
}