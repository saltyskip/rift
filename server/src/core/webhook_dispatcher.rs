pub use super::models::{
    AttributionEventPayload, ClickEventPayload, ConversionEventPayload, WebhookPayload,
};

pub trait WebhookDispatcher: Send + Sync {
    fn dispatch_click(&self, payload: ClickEventPayload);
    fn dispatch_attribution(&self, payload: AttributionEventPayload);
    fn dispatch_conversion(&self, payload: ConversionEventPayload);
}
