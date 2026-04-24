//! MagicLinksService — request + redeem logic for billing magic links.
//!
//! Keeps the business rules (IP + per-email rate limits, email-enumeration
//! defense, intent/tier validation, Stripe dispatch on redeem) out of the
//! HTTP layer. Route handlers are thin wrappers that map
//! `MagicLinkError`/`RedeemOutcome` variants onto HTTP responses.
//!
//! Why it's here and not in `repos/magic_links.rs`: the repo is a pure data
//! store. This service orchestrates the repo with email sending, Stripe
//! sessions, and tenant lookups — classic service-layer territory.

use std::sync::Arc;

use crate::core::rate_limit::RateLimiter;
use crate::services::auth::tenants::repo::{PlanTier, TenantsRepository};
use crate::services::billing::email as billing_email;
use crate::services::billing::repos::magic_links::{
    MagicLinkIntent, MagicLinkTier, MagicLinksRepository,
};
use crate::services::billing::stripe_client::{
    create_checkout_session_for_magic_link, create_portal_session, MagicLinkCheckoutOpts,
    StripeConfig,
};

/// Errors that bubble out to the HTTP layer. The service deliberately does
/// NOT surface "no such email" or "email send failed" — those are swallowed
/// internally to preserve the anti-enumeration contract (the request endpoint
/// always returns 200 to the caller).
#[derive(Debug)]
pub enum MagicLinkError {
    /// Caller-supplied input was malformed. `code` is the machine-readable
    /// slug, `message` is the human-readable explanation.
    Invalid { code: &'static str, message: String },
    /// Too many requests from this IP within the sustained window.
    RateLimited,
}

/// Successful redeem outcome. The route converts each variant into a 302.
#[derive(Debug)]
pub enum RedeemOutcome {
    /// 302 to the Stripe Checkout session URL.
    CheckoutUrl(String),
    /// 302 to the Stripe Billing Portal session URL.
    PortalUrl(String),
    /// Token was valid and intent=portal, but the email doesn't resolve to a
    /// tenant with a Stripe customer. Route redirects to /manage with an
    /// error banner.
    NoSubscription,
    /// Token unknown, expired, or already consumed. Route redirects to
    /// /pricing with an error banner.
    Expired,
}

/// Config slice passed to the service at construction. Keeps the service
/// decoupled from `AppState`; only the fields it actually needs are here.
#[derive(Clone)]
pub struct MagicLinksConfig {
    pub resend_api_key: String,
    pub resend_from_email: String,
    pub public_url: String,
    pub stripe: StripeConfig,
}

pub struct MagicLinksService {
    magic_links_repo: Arc<dyn MagicLinksRepository>,
    tenants_repo: Arc<dyn TenantsRepository>,
    config: MagicLinksConfig,
    /// Per-IP token bucket. Owned by the service so tests can construct a
    /// fresh bucket per case; production wires a single instance via
    /// `AppState`.
    ip_limiter: RateLimiter,
}

impl MagicLinksService {
    /// Magic link lifetime. Kept short because these tokens land in email
    /// bodies, access logs, and browser history — shorter window = smaller
    /// exposure.
    const TOKEN_TTL_SECS: i64 = 15 * 60;

    /// Per-email rate cap: at most 3 links per hour regardless of which IPs
    /// they come from. Stops someone from cycling proxies to spam one inbox.
    const PER_EMAIL_WINDOW_SECS: i64 = 3600;
    const PER_EMAIL_MAX: u64 = 3;

    pub fn new(
        magic_links_repo: Arc<dyn MagicLinksRepository>,
        tenants_repo: Arc<dyn TenantsRepository>,
        config: MagicLinksConfig,
    ) -> Self {
        // Per-IP budget: 5 requests per minute sustained, burst 5. Effectively
        // "a user can fire ~5 retries before being asked to wait a bit" —
        // tighter than per-email so abusers can't just swap emails.
        let ip_limiter = RateLimiter::new(5, 5);
        Self {
            magic_links_repo,
            tenants_repo,
            config,
            ip_limiter,
        }
    }

    /// Issue a magic link for `email` with `intent` (and `tier` if Subscribe).
    /// Always returns Ok when inputs validate and the IP isn't rate-limited —
    /// downstream failures (email not found, email send failed) are logged
    /// and swallowed so callers can't distinguish them from success.
    ///
    /// `client_ip` is the caller's IP as extracted by the transport
    /// (X-Forwarded-For, etc.) — the service doesn't care where it came from,
    /// just that it's stable per client.
    pub async fn request(
        &self,
        email_raw: &str,
        intent_raw: &str,
        tier_raw: Option<&str>,
        client_ip: &str,
    ) -> Result<(), MagicLinkError> {
        // IP rate limit first — cheapest check.
        if !self.ip_limiter.check(client_ip) {
            return Err(MagicLinkError::RateLimited);
        }

        let email = email_raw.trim().to_lowercase();
        if !email.contains('@') || email.len() < 5 {
            return Err(MagicLinkError::Invalid {
                code: "invalid_email",
                message: "Invalid email".to_string(),
            });
        }

        let intent = parse_intent(intent_raw)?;
        let tier = parse_subscribe_tier(intent, tier_raw)?;

        // Per-email rate limit. Always return Ok (anti-enumeration) but skip
        // sending when we're over the cap.
        let recent = self
            .magic_links_repo
            .count_recent_for_email(&email, Self::PER_EMAIL_WINDOW_SECS)
            .await
            .unwrap_or(0);
        if recent >= Self::PER_EMAIL_MAX {
            tracing::info!(email = %email, "magic_link_email_rate_limited");
            return Ok(());
        }

        match self
            .magic_links_repo
            .create(&email, intent, tier, Self::TOKEN_TTL_SECS)
            .await
        {
            Ok((raw_token, _doc)) => {
                let link_url = format!(
                    "{}/v1/billing/go?token={}",
                    self.config.public_url, raw_token
                );
                let send_result = match intent {
                    MagicLinkIntent::Subscribe => {
                        billing_email::send_magic_link_subscribe(
                            &self.config.resend_api_key,
                            &self.config.resend_from_email,
                            &email,
                            &link_url,
                            tier.expect("parse_subscribe_tier guaranteed Some for Subscribe"),
                        )
                        .await
                    }
                    MagicLinkIntent::Portal => {
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
                    // Don't leak the failure to the caller — log and return Ok.
                    tracing::error!(error = %e, email = %email, "magic_link_email_send_failed");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "magic_link_create_failed");
            }
        }

        Ok(())
    }

    /// Consume a raw token and resolve it to a Stripe destination.
    pub async fn redeem(&self, raw_token: &str) -> RedeemOutcome {
        let doc = match self.magic_links_repo.consume(raw_token).await {
            Ok(Some(d)) => d,
            Ok(None) => return RedeemOutcome::Expired,
            Err(e) => {
                tracing::error!(error = %e, "magic_link_consume_failed");
                return RedeemOutcome::Expired;
            }
        };

        let tenant = match self.tenants_repo.find_by_owner_email(&doc.email).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, "magic_link_tenant_lookup_failed");
                return RedeemOutcome::Expired;
            }
        };

        match doc.intent {
            MagicLinkIntent::Subscribe => {
                let Some(ml_tier) = doc.tier else {
                    return RedeemOutcome::Expired;
                };
                let plan_tier = ml_tier_to_plan_tier(ml_tier);
                let tenant_id_hex = tenant.as_ref().and_then(|t| t.id).map(|oid| oid.to_hex());
                let customer_id = tenant.as_ref().and_then(|t| t.stripe_customer_id.clone());

                let success_url = format!("{}/welcome", self.config.public_url);
                let cancel_url = format!("{}/pricing?error=cancelled", self.config.public_url);

                let opts = MagicLinkCheckoutOpts {
                    tier: plan_tier,
                    customer_id: customer_id.as_deref(),
                    customer_email: if customer_id.is_none() {
                        Some(doc.email.as_str())
                    } else {
                        None
                    },
                    pending_email: if tenant.is_none() {
                        Some(doc.email.as_str())
                    } else {
                        None
                    },
                    tenant_id_hex: tenant_id_hex.as_deref(),
                    success_url: &success_url,
                    cancel_url: &cancel_url,
                };

                match create_checkout_session_for_magic_link(&self.config.stripe, opts).await {
                    Ok(session) => RedeemOutcome::CheckoutUrl(session.url),
                    Err(e) => {
                        tracing::error!(error = %e, "magic_link_checkout_failed");
                        RedeemOutcome::Expired
                    }
                }
            }
            MagicLinkIntent::Portal => {
                let Some(tenant) = tenant else {
                    return RedeemOutcome::NoSubscription;
                };
                let Some(customer_id) = tenant.stripe_customer_id else {
                    return RedeemOutcome::NoSubscription;
                };
                let return_url = format!("{}/manage?done=1", self.config.public_url);
                match create_portal_session(
                    &self.config.stripe.secret_key,
                    &customer_id,
                    &return_url,
                )
                .await
                {
                    Ok(session) => RedeemOutcome::PortalUrl(session.url),
                    Err(e) => {
                        // Swallow Stripe errors into Expired — the user sees a
                        // generic "link expired" banner rather than internal
                        // config names.
                        tracing::error!(error = %e, "magic_link_portal_failed");
                        RedeemOutcome::Expired
                    }
                }
            }
        }
    }
}

fn parse_intent(raw: &str) -> Result<MagicLinkIntent, MagicLinkError> {
    match raw {
        "subscribe" => Ok(MagicLinkIntent::Subscribe),
        "portal" => Ok(MagicLinkIntent::Portal),
        _ => Err(MagicLinkError::Invalid {
            code: "invalid_intent",
            message: "intent must be 'subscribe' or 'portal'".to_string(),
        }),
    }
}

fn parse_subscribe_tier(
    intent: MagicLinkIntent,
    raw: Option<&str>,
) -> Result<Option<MagicLinkTier>, MagicLinkError> {
    if intent != MagicLinkIntent::Subscribe {
        return Ok(None);
    }
    let Some(s) = raw else {
        return Err(MagicLinkError::Invalid {
            code: "missing_tier",
            message: "tier is required when intent=subscribe".to_string(),
        });
    };
    MagicLinkTier::parse(s)
        .map(Some)
        .ok_or(MagicLinkError::Invalid {
            code: "invalid_tier",
            message: "tier must be one of pro, business, scale".to_string(),
        })
}

fn ml_tier_to_plan_tier(t: MagicLinkTier) -> PlanTier {
    match t {
        MagicLinkTier::Pro => PlanTier::Pro,
        MagicLinkTier::Business => PlanTier::Business,
        MagicLinkTier::Scale => PlanTier::Scale,
    }
}
