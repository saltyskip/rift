//! Wire-contract test: `AgentAction` must serialize to exactly the JSON the
//! server's `RecordActionRequest` (`POST /v1/agents/actions`) expects. If a
//! field name drifts on either side, this test fails before the bytes ever hit
//! the network.

use riftl_mcp::AgentAction;

#[test]
fn serializes_to_the_ingest_contract() {
    let action = AgentAction {
        tool: "recommend_plan".to_string(),
        agent_platform: Some("chatgpt".to_string()),
        intent: Some(serde_json::json!({ "goal": "automatic budgeting" })),
        status: "ok".to_string(),
        latency_ms: 142,
        mint_journey_token: false,
    };

    let v = serde_json::to_value(&action).unwrap();
    assert_eq!(v["tool"], "recommend_plan");
    assert_eq!(v["agent_platform"], "chatgpt");
    assert_eq!(v["intent"]["goal"], "automatic budgeting");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["latency_ms"], 142);
    assert_eq!(v["mint_journey_token"], false);
}

#[test]
fn optional_fields_are_omitted_when_none() {
    let action = AgentAction {
        tool: "ping".to_string(),
        agent_platform: None,
        intent: None,
        status: "ok".to_string(),
        latency_ms: 0,
        mint_journey_token: false,
    };

    let obj = serde_json::to_value(&action)
        .unwrap()
        .as_object()
        .cloned()
        .unwrap();

    // Omitted (server applies serde defaults) ...
    assert!(!obj.contains_key("agent_platform"));
    assert!(!obj.contains_key("intent"));
    // ... required fields always present.
    for key in ["tool", "status", "latency_ms", "mint_journey_token"] {
        assert!(obj.contains_key(key), "missing required field `{key}`");
    }
}
