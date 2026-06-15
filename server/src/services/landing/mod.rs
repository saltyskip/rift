//! Landing-page branding config. Owns the [`models::LandingTheme`] type — the
//! single brand input the landing-page renderer consumes. Phase 1 ships the
//! type with a `Default` impl (Rift defaults); Phase 2 persists it on the
//! tenant and adds a cascade (`Link override → LandingTheme → Rift default`).

pub mod models;
