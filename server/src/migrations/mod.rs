pub mod m001_auth_split;

use async_trait::async_trait;
use mongodb::Database;

#[async_trait]
pub trait Migration: Send + Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn run(&self, db: &Database, dry_run: bool) -> Result<(), String>;
}

pub fn all() -> Vec<Box<dyn Migration>> {
    vec![Box::new(m001_auth_split::M001AuthSplit)]
}

pub fn get_by_name(name: &str) -> Option<Box<dyn Migration>> {
    all().into_iter().find(|m| m.id() == name)
}
