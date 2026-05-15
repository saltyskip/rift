pub use super::models::{
    AttributeEventPayload, ClickEventPayload, ConversionEventPayload, IdentifyEventPayload,
    WebhookPayload,
};

pub trait WebhookDispatcher: Send + Sync {
    fn dispatch_click(&self, payload: ClickEventPayload);
    fn dispatch_attribute(&self, payload: AttributeEventPayload);
    fn dispatch_conversion(&self, payload: ConversionEventPayload);
    fn dispatch_identify(&self, payload: IdentifyEventPayload);
}
