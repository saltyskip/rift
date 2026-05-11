pub use super::models::{
    AttributionEventPayload, ClickEventPayload, ConversionEventPayload, IdentifyEventPayload,
    WebhookPayload,
};

pub trait WebhookDispatcher: Send + Sync {
    fn dispatch_click(&self, payload: ClickEventPayload);
    fn dispatch_attribution(&self, payload: AttributionEventPayload);
    fn dispatch_conversion(&self, payload: ConversionEventPayload);
    fn dispatch_identify(&self, payload: IdentifyEventPayload);
}
