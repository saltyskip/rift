//! Unit tests for the pure invite-classification and member-status logic.
//! The full `invite` flow needs a live TenantsService/TokenService and is
//! covered by integration tests; here we lock down the branch decisions.

use super::*;
use crate::core::public_id::{TenantId, UserId};
use crate::services::auth::users::models::MemberStatus;
use mongodb::bson::DateTime;

fn user(verified: bool, id: Option<UserId>) -> UserDoc {
    UserDoc {
        id,
        tenant_id: TenantId::new(),
        email: "alice@example.com".into(),
        verified,
        is_owner: false,
        created_at: DateTime::now(),
        invite_expires_at: None,
    }
}

#[test]
fn classify_no_row_creates_new() {
    assert_eq!(classify_invite(None), InviteAction::CreateNew);
}

#[test]
fn classify_verified_member_is_rejected() {
    assert_eq!(
        classify_invite(Some(user(true, Some(UserId::new())))),
        InviteAction::AlreadyMember
    );
}

#[test]
fn classify_pending_or_expired_resends_with_its_id() {
    let id = UserId::new();
    // An unverified row — whether its token is pending or long expired —
    // resolves to Resend so a fresh link can go out.
    assert_eq!(
        classify_invite(Some(user(false, Some(id)))),
        InviteAction::Resend(id)
    );
}

#[test]
fn status_verified_is_active_regardless_of_expiry() {
    let past = DateTime::from_millis(0);
    let now = DateTime::now();
    assert_eq!(
        MemberStatus::derive(true, Some(past), now),
        MemberStatus::Active
    );
}

#[test]
fn status_unverified_with_future_expiry_is_pending() {
    let now = DateTime::now();
    let future = DateTime::from_millis(now.timestamp_millis() + 60_000);
    assert_eq!(
        MemberStatus::derive(false, Some(future), now),
        MemberStatus::Pending
    );
}

#[test]
fn status_unverified_past_or_missing_expiry_is_expired() {
    let now = DateTime::now();
    let past = DateTime::from_millis(now.timestamp_millis() - 60_000);
    assert_eq!(
        MemberStatus::derive(false, Some(past), now),
        MemberStatus::Expired
    );
    assert_eq!(
        MemberStatus::derive(false, None, now),
        MemberStatus::Expired
    );
}
