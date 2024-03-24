pub mod notification;
pub mod request;
pub mod response;

pub use notification::DaemonNotification;
pub use request::{DaemonCommand, DaemonRequest, DaemonStartOptions};
pub use response::{DaemonResponse, DaemonResponseData, ServerConfig, ServerInfo};
use serde::{Deserialize, Serialize};

/// The `Message` object contains either a `DaemonRequest` or a `DaemonResponse` to be serialized / deserialized
/// for our IPC communication. Our IPC protocol is based on jsonrpc (https://www.jsonrpc.org/specification#overview),
/// but deviates from it in a few ways:
/// - We differentiate Notifications from Requests explicitly.
/// - We have a "type" field to differentiate between a request, a response, and a notification.
/// - The params field may receive any json value(such as a string), not just an object or an array.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonMessage {
    Request(DaemonRequest),
    Response(DaemonResponse),
    Notification(DaemonNotification),
}

impl From<DaemonRequest> for DaemonMessage {
    fn from(request: DaemonRequest) -> Self {
        DaemonMessage::Request(request)
    }
}

impl From<DaemonResponse> for DaemonMessage {
    fn from(response: DaemonResponse) -> Self {
        DaemonMessage::Response(response)
    }
}

impl From<DaemonNotification> for DaemonMessage {
    fn from(notification: DaemonNotification) -> Self {
        DaemonMessage::Notification(notification)
    }
}
