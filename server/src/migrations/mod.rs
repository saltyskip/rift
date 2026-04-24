pub mod m001_auth_split;
pub mod m002_billing_foundation;
pub mod m003_unify_tokens;

use async_trait::async_trait;
use mongodb::Database;

#[async_trait]
pub trait Migration: Send + Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn run(&self, db: &Database, dry_run: bool) -> Result<(), String>;
}

pub fn all() -> Vec<Box<dyn Migration>> {
    vec![
        Box::new(m001_auth_split::M001AuthSplit),
        Box::new(m002_billing_foundation::M002BillingFoundation),
        Box::new(m003_unify_tokens::M003UnifyTokens),
    ]
}

pub fn get_by_name(name: &str) -> Option<Box<dyn Migration>> {
    all().into_iter().find(|m| m.id() == name)
}
