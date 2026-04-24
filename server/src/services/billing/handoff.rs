//! BillingHandoffService — one-shot email → Stripe handoff.
//!
//! Post-refactor this service does two things:
//! 1. Issue a short-lived token via `TokenService`, email a link carrying it.
//! 2. On redeem, consume the token and dispatch to Stripe Checkout (new
//!    subscription / upgrade) or Stripe Billing Portal (manage / cancel)
//!    based on the token's purpose.
//!
//! Token storage used to live here as `MagicLinksRepo`. That moved to
//! `services/tokens` — this service is now the handoff orchestrator.
//! The HTTP route handlers in `api/billing/routes.rs` stay thin.

use std::sync::Arc;

use mongodb::bson::doc;
use serde::Deserialize;

use crate::core::rate_limit::RateLimiter;
use crate::services::auth::tenants::repo::{PlanTier, TenantsRepository};
use crate::services::billing::email as billing_email;
use crate::services::billing::stripe_client::{
    create_checkout_session_for_handoff, create_portal_session, HandoffCheckoutOpts, StripeConfig,
};
use crate::services::tokens::{ConsumeOutcome, TokenKind, TokenPurpose, TokenService, TokenSpec};

// ── HTTP boundary types ──
//
// These are the request-body shapes the handoff endpoint accepts. They used
// to double as storage discriminators (`MagicLinkIntent`/`Tier` enums); post-
// refactor storage lives on `TokenPurpose`, so these are purely "what the
// client asked for."

/// What flow the caller wants the magic link to gate. Maps onto `TokenPurpose`
/// at issue time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingIntent {
    Subscribe,
    Portal,
}

/// Paid tier selector. `PlanTier` minus Free (Free doesn't need Stripe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingTier {
    Pro,
    Business,
    Scale,
}

impl BillingTier {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pro" => Some(Self::Pro),
            "business" => Some(Self::Business),
            "scale" => Some(Self::Scale),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pro => "pro",
            Self::Business => "business",
            Self::Scale => "scale",
        }
    }

    pub fn to_plan_tier(self) -> PlanTier {
        match self {
            Self::Pro => PlanTier::Pro,
            Self::Business => PlanTier::Business,
            Self::Scale => PlanTier::Scale,
        }
    }
}

// ── Errors + outcomes ──

#[derive(Debug)]
pub enum BillingHandoffError {
    Invalid { code: &'static str, message: String },
    RateLimited,
}

/// Successful redeem outcome. Routes convert each variant to a 302.
#[derive(Debug)]
pub enum HandoffOutcome {
    CheckoutUrl(String),
    PortalUrl(String),
    /// Portal intent but the email doesn't resolve to a Stripe customer.
    NoSubscription,
    /// Token not found, expired, already consumed, or of an unexpected purpose.
    Expired,
}

// ── Config + service ──

/// Config slice passed at construction. Kept separate from `AppState` so the
/// service is easy to test with arbitrary Stripe/Resend stubs.
#[derive(Clone)]
pub struct BillingHandoffConfig {
    pub resend_api_key: String,
    pub resend_from_email: String,
    /// API domain (e.g. `https://api.riftl.ink`). Used when building the
    /// magic-link URL that embeds in the email — must hit an API route.
    pub public_url: String,
    /// Marketing site (e.g. `https://riftl.ink`). Used for every redirect
    /// target that lands on a marketing page: Stripe success/cancel, portal
    /// return, link-expired banner, no-subscription banner.
    pub marketing_url: String,
    pub stripe: StripeConfig,
}

pub struct BillingHandoffService {
    tokens: Arc<TokenService>,
    tenants_repo: Arc<dyn TenantsRepository>,
    config: BillingHandoffConfig,
    ip_limiter: RateLimiter,
}

impl BillingHandoffService {
    /// 15min TTL — short because the link lives in email bodies, access logs,
    /// and browser history.
    const TOKEN_TTL_SECS: i64 = 15 * 60;

    /// Per-email: 3 handoff emails per hour. Stops someone from spamming an
    /// inbox by cycling proxies.
    const PER_EMAIL_WINDOW_SECS: i64 = 3600;
    const PER_EMAIL_MAX: u64 = 3;

    pub fn new(
        tokens: Arc<TokenService>,
        tenants_repo: Arc<dyn TenantsRepository>,
        config: BillingHandoffConfig,
    ) -> Self {
        // Per-IP: 5 req/min sustained, burst 5. Tighter than per-email so
        // abusers can't bypass with email rotation.
        let ip_limiter = RateLimiter::new(5, 5);
        Self {
            tokens,
            tenants_repo,
            config,
            ip_limiter,
        }
    }

    /// Issue a handoff link for `email` + intent (+ tier when Subscribe).
    /// Returns Ok when inputs are valid and the IP isn't rate-limited;
    /// downstream failures (email send, etc.) are logged and swallowed to
    /// preserve the enumeration-defense "always 200" contract.
    pub async fn request(
        &self,
        email_raw: &str,
        intent_raw: &str,
        tier_raw: Option<&str>,
        client_ip: &str,
    ) -> Result<(), BillingHandoffError> {
        if !self.ip_limiter.check(client_ip) {
            return Err(BillingHandoffError::RateLimited);
        }

        let email = email_raw.trim().to_lowercase();
        if !email.contains('@') || email.len() < 5 {
            return Err(BillingHandoffError::Invalid {
                code: "invalid_email",
                message: "Invalid email".to_string(),
            });
        }

        let intent = parse_intent(intent_raw)?;
        let tier = parse_subscribe_tier(intent, tier_raw)?;

        // Per-email rate limit. Silently skip sending when over cap; still
        // return 200 so callers can't tell the difference.
        let purpose = match intent {
            BillingIntent::Subscribe => TokenPurpose::BillingSubscribe,
            BillingIntent::Portal => TokenPurpose::BillingPortal,
        };
        let recent = self
            .tokens
            .count_recent(purpose, &email, Self::PER_EMAIL_WINDOW_SECS)
            .await
            .unwrap_or(0);
        if recent >= Self::PER_EMAIL_MAX {
            tracing::info!(email = %email, "handoff_email_rate_limited");
            return Ok(());
        }

        let metadata = match tier {
            Some(t) => doc! { "tier": t.as_str() },
            None => doc! {},
        };

        let raw_token = match self
            .tokens
            .issue(TokenSpec {
                purpose,
                kind: TokenKind::HashKeyed,
                ttl_secs: Self::TOKEN_TTL_SECS,
                email: email.clone(),
                metadata,
            })
            .await
        {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, "handoff_token_issue_failed");
                return Ok(());
            }
        };

        let link_url = format!(
            "{}/v1/billing/go?token={}",
            self.config.public_url, raw_token
        );
        let send_result = match intent {
            BillingIntent::Subscribe => {
                billing_email::send_magic_link_subscribe(
                    &self.config.resend_api_key,
                    &self.config.resend_from_email,
                    &email,
                    &link_url,
                    tier.expect("validated above"),
                )
                .await
            }
            BillingIntent::Portal => {
                billing_email::send_magic_link_portal(
                    &self.config.resend_api_key,
                    &self.config.resend_from_email,
                    &email,
                    &link_url,
                )
                .await
            }
        };
        if let Err(e) = send_result {
            tracing::error!(error = %e, email = %email, "handoff_email_send_failed");
        }

        Ok(())
    }

    /// Consume a raw token and resolve it to a Stripe destination. The
    /// token's purpose (stored at issue time) decides Checkout vs Portal —
    /// the route doesn't need to know up front.
    pub async fn redeem(&self, raw_token: &str) -> HandoffOutcome {
        let outcome = match self.tokens.consume_hash(raw_token).await {
            Ok(o) => o,
            Err(e) => {
                tracing::error!(error = %e, "handoff_consume_failed");
                return HandoffOutcome::Expired;
            }
        };

        let (purpose, email, metadata) = match outcome {
            ConsumeOutcome::Ok {
                purpose,
                email,
                metadata,
            } => (purpose, email, metadata),
            ConsumeOutcome::NotFound | ConsumeOutcome::AttemptsExhausted => {
                return HandoffOutcome::Expired;
            }
        };

        let tenant = match self.tenants_repo.find_by_owner_email(&email).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, "handoff_tenant_lookup_failed");
                return HandoffOutcome::Expired;
            }
        };

        match purpose {
            TokenPurpose::BillingSubscribe => {
                // Extract tier from metadata.
                let tier_str = match metadata.get_str("tier") {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(error = %e, "handoff_missing_tier_metadata");
                        return HandoffOutcome::Expired;
                    }
                };
                let Some(tier) = BillingTier::parse(tier_str) else {
                    tracing::error!(tier = %tier_str, "handoff_invalid_tier_metadata");
                    return HandoffOutcome::Expired;
                };
                let plan_tier = tier.to_plan_tier();

                let tenant_id_hex = tenant.as_ref().and_then(|t| t.id).map(|oid| oid.to_hex());
                let customer_id = tenant.as_ref().and_then(|t| t.stripe_customer_id.clone());

                let success_url = format!("{}/welcome", self.config.marketing_url);
                let cancel_url =
                    format!("{}/?error=cancelled#pricing", self.config.marketing_url);

                let opts = HandoffCheckoutOpts {
                    tier: plan_tier,
                    customer_id: customer_id.as_deref(),
                    customer_email: if customer_id.is_none() {
                        Some(email.as_str())
                    } else {
                        None
                    },
                    pending_email: if tenant.is_none() {
                        Some(email.as_str())
                    } else {
                        None
                    },
                    tenant_id_hex: tenant_id_hex.as_deref(),
                    success_url: &success_url,
                    cancel_url: &cancel_url,
                };

                match create_checkout_session_for_handoff(&self.config.stripe, opts).await {
                    Ok(session) => HandoffOutcome::CheckoutUrl(session.url),
                    Err(e) => {
                        tracing::error!(error = %e, "handoff_checkout_failed");
                        HandoffOutcome::Expired
                    }
                }
            }
            TokenPurpose::BillingPortal => {
                let Some(tenant) = tenant else {
                    return HandoffOutcome::NoSubscription;
                };
                let Some(customer_id) = tenant.stripe_customer_id else {
                    return HandoffOutcome::NoSubscription;
                };
                let return_url = format!("{}/manage?done=1", self.config.marketing_url);
                match create_portal_session(
                    &self.config.stripe.secret_key,
                    &customer_id,
                    &return_url,
                )
                .await
                {
                    Ok(session) => HandoffOutcome::PortalUrl(session.url),
                    Err(e) => {
                        tracing::error!(error = %e, "handoff_portal_failed");
                        HandoffOutcome::Expired
                    }
                }
            }
            // Token valid but for a different flow (email verify, key
            // rotation). Refuse — the user clicked a link from the wrong
            // context.
            _ => HandoffOutcome::Expired,
        }
    }
}

fn parse_intent(raw: &str) -> Result<BillingIntent, BillingHandoffError> {
    match raw {
        "subscribe" => Ok(BillingIntent::Subscribe),
        "portal" => Ok(BillingIntent::Portal),
        _ => Err(BillingHandoffError::Invalid {
            code: "invalid_intent",
            message: "intent must be 'subscribe' or 'portal'".to_string(),
        }),
    }
}

fn parse_subscribe_tier(
    intent: BillingIntent,
    raw: Option<&str>,
) -> Result<Option<BillingTier>, BillingHandoffError> {
    if intent != BillingIntent::Subscribe {
        return Ok(None);
    }
    let Some(s) = raw else {
        return Err(BillingHandoffError::Invalid {
            code: "missing_tier",
            message: "tier is required when intent=subscribe".to_string(),
        });
    };
    BillingTier::parse(s)
        .map(Some)
        .ok_or(BillingHandoffError::Invalid {
            code: "invalid_tier",
            message: "tier must be one of pro, business, scale".to_string(),
        })
}
