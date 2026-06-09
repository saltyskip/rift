# Rift Agent Layer — Product & Technical Spec

**Status:** Draft / for discussion
**Author:** drei
**Last updated:** 2026-06-05
**Working name:** Rift Agent Layer (RAL)

> One line: **Rift Agent Layer is the measurement + controlled-handoff layer for MCP servers** — a drop-in SDK that auto-instruments every tool an agent calls, and provides a deferred-deeplink rail for the moments a human takes over, so a brand can answer *"which agent drove this conversion?"* across ChatGPT, Perplexity, Claude, and Gemini — including the web→app handoff no MMP can see.
>
> Think **"Branch for MCP-server developers."**

---

## 1. Summary

As AI assistants become a real acquisition channel, brands face a measurement crisis: AI-referred traffic is their **highest-converting** channel and their **least-trackable** one. Existing tools either measure the inbound *citation* (web-only, referrer-dependent) or instrument the *pipe* (ops/billing), but nothing connects an **agent's action** to a **downstream conversion** — especially across the web→app install boundary, where attribution structurally breaks today.

Rift already owns the hard half of that boundary: deep links, deferred deep linking, attribution chains, conversions, and affiliate credit. The Agent Layer **re-points that engine from "app-install channels" to "agent-action surfaces."**

The product is a **drop-in SDK** (Rust first, for dogfooding through Rift's own MCP; FastMCP/Python second, for the market) that:

1. **Auto-instruments** every `tools/call` on the operator's MCP server — zero per-tool code.
2. **Emits an agent-action event** to Rift (async, fail-open).
3. **Rewrites URLs** in tool responses into Rift deferred-deeplinks that carry a journey token + agent context (the handoff rail).
4. Feeds a **cross-agent conversion funnel** no platform will unify for the brand.

---

## 2. Problem (today, not 2027)

AI-assistant traffic is simultaneously the best and the most invisible channel in 2026:

- **It converts 5–10× better.** Claude ~16.8%, ChatGPT ~14–16%, Perplexity ~10.5% vs. Google organic ~2–3%. ([ALM Corp](https://almcorp.com/blog/chatgpt-vs-organic-search-conversion-rate/))
- **It's mostly dark.** ~70% of AI traffic arrives with no referrer and lands in "Direct"; 35–70% per Statcounter. ([Loamly](https://www.loamly.ai/blog/ai-traffic-attribution-crisis))
- **The web→app handoff zeroes it out.** Named the "MMP blind spot": *AI citation → brand awareness → search → App Store → install*, where the MMP logs a "natural install" and AI's contribution disappears. ([Global-Gravity](https://www.global-gravity.com/en/blog/mmp-blind-spot-ai-rewrites-acquisition))
- **GA4's May-2026 `ai-assistant` channel** is web-only, referrer-dependent, and dies at the App Store — it names the *medium*, not the *dollar*, and can't cross to the app.

The **web→app handoff is exactly what deferred deep linking exists to solve** — and it's the seam where every incumbent loses the AI source. Rift already spans it.

### The honest constraint we design around
We do **not** out-detect referrer stripping on the web. We do not control what an agent *cites* (it links `brand.com`, never a Rift link). Our value is at the **verb**, not the **citation**: instrument the actions an operator's own agent surface exposes, and preserve context across the handoff.

---

## 3. Positioning & strategy

### Own the verb, not the citation
An agent can display `brand.com/x` forever — that's just the address. The moment it needs to *do* something (drive an install, deep-link to a screen, complete an action), it **calls a tool**. Whatever the tool returns is what the agent uses. If the operator's tool is Rift-wrapped, the agent gets an attributable, deferred deep link — even while still showing the human "brand.com." **Display stays canonical; the verb routes through Rift.**

### Two modes of agent commerce — we span both
- **Mode 1 — autonomous completion.** "Best shoe" → search → "buy it" → headless tokenized checkout in-chat. **No link, not our game.** Owned by OpenAI Instant Checkout / ACP / Shopify. We *measure* it (the action happened) but we don't try to insert a link.
- **Mode 2 — handoff to human.** The conversion lives where the agent can't reach: an app, an authed session, a regulated flow, a rich UI, or a brand that *refuses to be a faceless SKU*. The agent returns a link; the human continues. **This is where the deep-link rail is load-bearing.**

Spanning both is the de-risk: we don't have to bet which mode wins. In Mode 1 we're analytics; in Mode 2 we're analytics + the rail. The more uncertain the future, the more valuable a neutral layer that measures both.

### Customer = the MCP-server operator's growth/dev team
Exactly Branch's customer (the app dev), one layer up:

| | Branch (2014) | Rift Agent Layer (2026) |
|---|---|---|
| Customer | App developer | MCP-server / agent-surface operator |
| Pain | "Can't attribute installs across web↔app & channels" | "Agents call my tools and things convert downstream — I have no idea which agent or what" |
| Integration | Branch SDK + links | Rift SDK on their MCP server + handoff links |
| In-path because | Dev put Branch in their links/app | Operator put Rift in the actions their server returns |
| Measures | install → open → conversion, by channel | tool call → handoff → install/purchase, **by agent** |

### Target the conversions where we're the *only* answer
Lead with operators whose conversion is an **app install / in-app activation** (apps, fintech, gaming, streaming, subscriptions). There, deferred deep linking is structural and we beat everyone. Web-checkout-of-a-SKU is Shopify's; don't fight there.

---

## 4. Competitive landscape

Six camps. Each owns one adjacent piece; none owns the combination. **The closest competitor is MCPcat (row A0) — it has already nailed the exact drop-in DX we designed, so we must NOT lead with observability.**

| Camp | Examples | Owns | Doesn't | Threat |
|---|---|---|---|---|
| **A0. MCP-native product analytics** ⭐ | **[MCPcat](https://mcpcat.io/)** | Session replay + tool-call analytics + intent/traffic-by-client; one-line `track()` drop-in in TS/Python/Go; free tier + OSS-free; real logos | **No** conversion attribution, revenue/outcome, deep linking, web→app handoff, or cross-agent *business* attribution | **Highest on the wedge** — they own observability. We must differentiate on the moat, not capture. See §4.1. |
| **A. AI-visibility / AEO** | Profound, Scrunch, Evertune | Inbound: "did citations convert" (GA4/web) | Web-only, referrer-bound, no app, no handoff, no own-surface instrumentation | **Med** — closest on "attribution," but a content/AEO muscle, not deep-link infra |
| **B. MCP gateways / observability** | Moesif, Kong, NGINX, Composio | The pipe: ops metrics + per-call billing | Zero downstream conversion / revenue / handoff / cross-agent attribution | **Low-Med** — could add attribution, but different buyer (sec/ops) |
| **C. LLM/agent observability** | Langfuse, LangSmith, Helicone, Braintrust | Builder debugging (traces, evals) | Not business attribution; treats agents as LLM-call sequences | **None** — different buyer |
| **D. Agentic-commerce platforms** | **Shopify Agentic Storefronts**, ACP, AP2 | AI order attribution *inside Shopify, web checkout* | Non-Shopify, app/deep conversions, cross-platform, the rail | **High** — so don't fight web checkout |
| **E. Human-in-the-loop / trust** | Amex ACE, AP2 mandates, Agno `requires_confirmation` | "Prove a human authorized" (payments/fraud) | Measurement, deep-linking, the funnel | **Adjacent** — sit alongside, not against |

**The gap (unclaimed):** instrument the brand's **own agent surface** + tie actions to **downstream app/deep conversions** + provide the **deferred-deeplink handoff rail** + **cross-platform** + with **auto-vs-confirm autonomy control.**

### 4.1 MCPcat — the closest competitor (read carefully)
MCPcat is the most similar product in market and the one to position against explicitly.

- **What it is:** session replay + product analytics for MCP servers — "know where agents get stuck." One-line `mcpcat.track(server, id)` in **TS/Python/Go**, capturing tool calls, agent intent, errors, traffic-by-client. Free up to 1k sessions/mo, free for OSS, with traction (Webflow, Cortex, GibsonAI).
- **It has already built our v0 wedge — better.** The drop-in DX, the languages, the buyer (MCP server owners) are exactly what we sketched. **Leading with "we measure your tool calls" makes us a worse, later, single-language MCPcat.** Do not enter on observability.
- **It does none of the moat.** No conversion attribution, revenue/outcome, deep linking, web→app handoff, or cross-agent *business* attribution. It's a tool for the **MCP engineer** (improve the agent UX), not for **growth** (attribute revenue).
- **The framing:** `MCPcat : Rift Agent Layer :: Amplitude/LogRocket : Branch.` Session-replay/product-analytics of MCP vs. deep-linking/attribution of MCP. **Different buyer, different category — could coexist (a brand runs both).**
- **The defensibility argument:** our moat is the deferred-deeplink + web→app + conversion engine — hard infra (years of Rift's existing product) MCPcat would build from scratch. The *capture* half is not defensible; assume MCPcat (or anyone) can replicate it, and that it trends to free.
- **Watch:** MCPcat moving down-funnel. They already have intent + client + custom user attributes; bolting on "did it convert" is a smaller leap for them than building deferred-install deep-linking is for us to be copied. Our durable edge is the link engine, not the sensor. (See risk #9.)
- **Open: partner or compete?** They own capture; we own attribution + handoff. A capture→attribution integration is plausible. Note, don't decide yet.

**Timing:** AI sessions are still <0.2% of ecommerce traffic (growing 7–11×); the industry pegs attribution maturity 18–24 months out ([digitalapplied](https://www.digitalapplied.com/blog/ai-agent-commerce-revenue-attribution-guide-2026), [commercetools](https://commercetools.com/blog/agentic-commerce-stats-enterprise-guide)). We're building the measurement layer just ahead of the volume — the Branch timing. The risk is *too early*, not *wrong*.

---

## 5. Product

### 5.1 What it is
A drop-in SDK + a backend. The SDK is the **sensor** (auto-capture + URL-rewrite); the backend is the **brain** (event store, journey-token mint, attribution chain, cross-agent funnel).

⚠️ **Positioning correction (post-MCPcat, see §4.1):** observability/tool-call analytics is **already owned** by MCPcat and trending to free. **Do not lead with it.** The wedge *and* the moat must both be **attribution-to-outcome + the deferred-deeplink handoff** — the thing MCPcat structurally isn't. Capture is table stakes we ship because we need the data, not a value prop we sell.

### 5.2 The integration (operator's view)
One line. The SDK hooks the single `tools/call` chokepoint — every tool, present and future, captured with no per-tool code.

```rust
// Rust (rmcp)
let service = MyMcpServer::new(deps).instrument(RiftConfig::from_env());
service.serve(transport).await?;
```
```python
# Python (FastMCP) — the market port
mcp.add_middleware(RiftMiddleware(api_key=RIFT_KEY))
```

### 5.3 The autonomy / "confirm before YOLO" control
The hero feature, not a fallback. As agents gain the power to spend, **users and brands both want a trust brake.** Per-action policy `AgentAutonomy { Auto, Confirm }`:
- `Auto` → action completes; Rift logs it.
- `Confirm` → the tool returns a Rift handoff link ("Review & buy in the app →"); the human validates in the brand's surface; Rift attributes on completion.

This is the natural bridge between Mode 1 and Mode 2 and the seam where the deep-link rail earns its keep.

### 5.4 Worked example (the demo that matters)
**Plume**, a budgeting app; conversion = "user links a bank account." They add `riftl-mcp` to their MCP server.

1. User in ChatGPT: *"I keep overspending — what helps me budget automatically?"*
2. ChatGPT calls Plume's tool `recommend_plan({intent:"automatic budgeting"})`.
3. SDK records `actor=agent, platform=chatgpt, tool=recommend_plan, intent=…, T0`; rewrites the returned URL into a Rift deep link with a journey token.
4. User taps → installs → first open claims the journey token → install bound to the T0 agent touch (existing deferred-install machinery).
5. +2 days: bank-link conversion fires → attribution chain walked.
6. **Dashboard row:** `Activation · bank linked · first touch: ChatGPT (recommend_plan, "automatic budgeting") · +2d`.

Their MMP calls that "organic." GA4 calls it "direct." **Only Rift names it** — because Rift sat at the verb.

### 5.5 The dashboard (the thing no one else can show)
Unified funnel across modes and agents:
`1,000 intents → 600 auto-completed · 400 handed off → 350 confirmed`, sliced by **agent platform / tool / intent**, all the way to activation, with first/last-touch and affiliate credit.

---

## 6. Scope & non-goals

**In scope:** instrumenting operator-hosted MCP surfaces; agent-action capture; deferred-deeplink handoff; cross-agent conversion attribution; auto-vs-confirm policy; the unified funnel.

**Explicit non-goals:**
- **We do not build a payment rail.** Sit on x402/ACP/AP2; never rebuild them.
- **We do not do headless Mode-1 web checkout.** Shopify/ACP own it. We measure it at most.
- **We cannot instrument surfaces we don't host** (e.g. Shopify's MCP). Target own-surface operators.
- **We do not out-detect referrer stripping** on the open web. Value is at the verb.
- **No user-scoped analytics** (cohorts/funnels/retention) — consistent with the existing conversions hard-line; link/agent-scoped questions only.

---

## 7. Architecture

### 7.1 Layer model (where we sit)
```
MCP (the spec)           → defines tools/call exists. Universal anchor.        [not ours]
  └ SDKs (rmcp/TS/Python) → where we hook in, one integration per language.    [our SDK]
      └ middleware/wrapper → the nicest hook; we wrap tools/call.              [our SDK]
          └ Rift Agent Layer = a Sentry-shaped plugin for MCP servers.        [our SDK]
Rift backend             → ingest + event store + journey token + funnel.      [our server]
```
We instrument the **server** (operator-controlled), never the **client** (inside Claude/ChatGPT — untouchable).

### 7.2 End-to-end data flow
```
Agent calls operator's Rift-wrapped tool
        │
        ▼
[NEW] record AgentActionEvent          ← agent_platform, tool, intent, session
        │
        ├── Mode 1 (auto): operator completes → reports conversion to Rift
        │        correlation key = agent_action_id   (server-side, no device)
        │
        └── Mode 2 (confirm/handoff):
                 [NEW] mint journey_token (rj_…), URL-rewritten into the response
                       │
                       ▼
                 EXISTING do_resolve → record_click      [EXTEND: actor=agent]
                       │
                       ▼
                 install → EXISTING POST /v1/lifecycle/attribute
                       [EXTEND: accept journey_token → bind install to agent action]
                       correlation key = journey_token   (through the device)
        │
        ▼
EXISTING conversion (POST /w/{token}) → credited_links_for_user
        [EXTEND: carry first/last_touch_actor + agent_platform]
        │
        ▼
[NEW] unified funnel: agent_action_events ⋈ attribution_events ⋈ conversion_events
        grouped by agent_platform / tool / intent
```

**Two correlation keys, one per mode** — this is what lets one product span both:
- **Mode 1:** `agent_action_id`, reported server-side by the operator at conversion (no install).
- **Mode 2:** `journey_token`, carried in the link and claimed at first-open (reuses today's deferred-install binding).

### 7.3 Reuse vs. net-new
~70% is extension of the existing engine.

| Piece | New/Extend | Location |
|---|---|---|
| `AgentActionEvent` + time-series collection | **New** | new `services/agents/{models,repo,service}.rs` |
| `actor` + `agent_platform` on touches | **Extend** | `AttributionEventMeta` in `services/links/models.rs`; thread through `record_attribute_event` (service + repo) |
| `journey_token` (`Id<JourneyTokenMarker>`, prefix `rj_`) + claim | **New** | marker in `core/public_id/mod.rs` + `models.rs`; claim field on `AttributeRequest` in `api/lifecycle` |
| `AgentAutonomy { Auto, Confirm }` policy | **New (small)** | `services/agents/models.rs`; v1 = tenant/link flag, not a policy engine |
| Agent fields on conversion payload | **Extend** | `CreditedLinks` (`services/links/models.rs`) + `ConversionEventPayload` (`core/models.rs`); populate in `credited_links_for_user` (repo) |
| Ingest endpoint | **New** | `POST /v1/agents/actions` (rl_live_ authed, same `auth_context` pattern as `mcp/server.rs`) |
| Unified funnel read API | **New** | `services/analytics` aggregations + `api/analytics` routes |

Inherited for free: time-series + `retention_bucket` TTL, `Resource::TrackEvent` quota, fire-and-forget webhook dispatch, `rl_live_ → TenantId` auth, the `credited_links_for_user` chain walk, identify-backfill. The journey-token claim is structurally identical to today's `install_id`→touch binding — a second binding key, not a new system.

---

## 8. Data model (sketch)

```rust
// services/agents/models.rs  (NEW domain)
pub struct AgentActionEvent {
    pub id: Option<AgentActionId>,      // cev-style public id, prefix e.g. "aae_"
    pub timestamp: DateTime,
    pub meta: AgentActionMeta,
    pub agent_platform: Option<String>, // "chatgpt" | "claude" | "perplexity" | ... (self-reported)
    pub tool: String,                   // e.g. "recommend_plan"
    pub intent: Option<serde_json::Value>, // tool arguments, redacted/capped
    pub status: String,                 // "ok" | "error"
    pub latency_ms: u32,
    pub journey_token: Option<JourneyToken>, // set if a handoff link was minted
}
pub struct AgentActionMeta {
    pub tenant_id: TenantId,
    pub retention_bucket: String,
}

pub enum AgentAutonomy { Auto, Confirm }

// services/links/models.rs  (EXTEND)
pub struct AttributionEventMeta {
    // ...existing...
    pub actor: Option<String>,          // "device" | "agent"  (default "device")
    pub agent_platform: Option<String>,
}

// core/models.rs  (EXTEND ConversionEventPayload)
pub struct ConversionEventPayload {
    // ...existing first/last_touch_*...
    pub first_touch_actor: Option<String>,
    pub first_touch_agent_platform: Option<String>,
    pub last_touch_actor: Option<String>,
    pub last_touch_agent_platform: Option<String>,
}
```

**Open modeling decisions:** new `services/agents/` domain vs. fold into `links` (lean: new domain — cleaner boundaries, architecture-test friendly); whether agent actions consume `TrackEvent` quota or a new meter (a pricing decision in disguise).

---

## 9. The SDK

### 9.1 Design rules
- **Separate crate/package**, never server-internal code. Depends only on the MCP SDK + an HTTP client + the ingest contract — *never* on Rift's server internals. This is what makes dogfood == product.
- **One-line drop-in** via the idiomatic hook per language.
- **Async + fail-open.** Capture happens off the hot path; if Rift is unreachable, the tool still returns. We are **never** a hard availability dependency of the operator's commerce flow. (This is why we ship middleware, not a proxy.)
- **Auto-handoff via URL-rewrite.** Any URL already in a tool response is transparently wrapped into a Rift deferred link (journey token + agent context). Net-new handoff destinations need a one-time declarative map, not per-call code.

### 9.2 Rust (rmcp) — first, for dogfooding
`ServerHandler::call_tool` is the chokepoint. Decorator + blanket ext-trait:

```rust
pub struct Instrumented<S> { inner: S, client: RiftClient }

impl<S: ServerHandler> ServerHandler for Instrumented<S> {
    async fn call_tool(&self, req: CallToolRequestParam, ctx: RequestContext<RoleServer>)
        -> Result<CallToolResult, McpError>
    {
        let tool = req.name.clone();
        let started = Instant::now();
        let result = self.inner.call_tool(req, ctx).await;
        self.client.emit(AgentAction { tool, ms: started.elapsed(), agent: ctx.peer_client_info() });
        result.map(rewrite_urls)
    }
    // delegate the rest to self.inner (one-time boilerplate; macro_rules! to taste)
}

pub trait RiftInstrumentExt: ServerHandler + Sized {
    fn instrument(self, cfg: RiftConfig) -> Instrumented<Self> { Instrumented::new(self, cfg) }
}
impl<S: ServerHandler> RiftInstrumentExt for S {}
```
*Exact rmcp signatures + the cleanest non-`call_tool` delegation need confirming against the pinned rmcp version.*

### 9.3 Python (FastMCP) — second, the market opener
Native middleware exists (`on_call_tool`) — supported, stable, proven (Scout APM ships on it). Same body: emit async/fail-open, URL-rewrite. **This port is what opens the market** (Rust MCP servers are the smallest ecosystem).

### 9.4 TypeScript — third, deliberate burden
No official `server.use()` yet ([#1238](https://github.com/modelcontextprotocol/typescript-sdk/issues/1238)). Wrap the `tools/call` handler via the low-level `Server` / `setRequestHandler` chokepoint, hidden behind the same `Rift.instrument(server)` facade. Relies on SDK internals → pin + CI-test against versions; swap to official middleware when #1238 lands without changing the operator's one line.

### 9.5 Agent identity caveat (recurring) — confirmed at the code level
`agent_platform` comes from MCP `initialize` `clientInfo` (`Implementation { name, version }`) — **self-reported and transport-dependent.** Confirmed in rmcp 1.2.0: it lives on `context.peer.peer_info()`, set via a `OnceCell` during `initialize`. **But Rift runs streamable-HTTP in *stateless* mode (`NeverSessionManager`), so `peer_info()` is frequently `None` inside `call_tool`** — `initialize` and `tools/call` can be independent requests with no shared peer. **Therefore read identity from the HTTP layer**, not `peer_info`: the axum `Parts` extension Rift already threads into tools (`Extension<Parts>`), request headers, or `context.meta` (`_meta` on the call). Good enough for v0; harden later with signed agent tokens / Know-Your-Agent credentials slotted into the same field.

---

## 10. API surface (backend)

`POST /v1/agents/actions` — rl_live_ authed (same resolution as `RiftMcp::auth_context`).
```jsonc
// request
{
  "tool": "recommend_plan",
  "agent_platform": "chatgpt",          // self-reported, nullable
  "intent": { "goal": "automatic budgeting" },
  "status": "ok",
  "latency_ms": 142,
  "mint_journey_token": true            // if the SDK URL-rewrote a handoff link
}
// response
{ "agent_action_id": "aae_…", "journey_token": "rj_…" }
```
`POST /v1/lifecycle/attribute` — **EXTEND** with optional `journey_token` to bind an install to a prior agent action.
`ConversionEventPayload` — **EXTEND** with the `*_actor` / `*_agent_platform` fields.
Funnel read endpoints under `api/analytics` (v2).

---

## 11. Phasing

Each phase independently demoable; each de-risks the next.

### v0 — Engine + Rust SDK + dogfood (the build that proves the stack)
- `services/agents/` domain: `AgentActionEvent` store, `journey_token` marker.
- `POST /v1/agents/actions` ingest.
- `riftl-mcp` **Rust crate**: `Instrumented<S>` + `.instrument()`, async/fail-open emit, URL-rewrite.
- Wire Rift's own `RiftMcp` through the crate, pointed at its own endpoint.
- One synthetic **handoff tool** so the funnel isn't empty (else you only prove capture).
- **Proves:** data model, capture hook (Rust reference impl), end-to-end through the real surface. **Demo:** add one line to an rmcp server → every tool call lands, attributed to the calling agent; one action flows to a (test) conversion.
- **Does NOT prove:** adoption at scale, the conversion moat on real consumer flows.

### v1 — Handoff rail + conversion attribution
- `actor`/`agent_platform` on touches; `journey_token` claim through `/lifecycle/attribute`; agent fields on `ConversionEventPayload`.
- **Demo:** the full Plume story end-to-end — agent action → install → conversion credited to the agent.

### v2 — Autonomy + funnel + FastMCP port
- `AgentAutonomy { Auto, Confirm }` (the trust brake); the cross-agent funnel dashboard; **`riftl-mcp` FastMCP package** (market opener).
- **Demo:** the unified funnel screen no competitor can show.

---

## 12. Risks & open questions

| # | Risk / question | Take |
|---|---|---|
| 1 | **Timing — too early?** AI sessions <0.2% of traffic. | Real risk. v0 is cheap precisely to find a design partner before heavy build. |
| 2 | **Shopify disintermediation.** They tag AI orders natively. | Don't fight web checkout. Win non-Shopify, app/deep, cross-platform neutrality — the slice no single platform offers. |
| 3 | **Adoption / DX.** Will operators wire an SDK or wait for platforms? | The whole reason for one-line auto-instrument. Validate with a real operator, not on spec. |
| 4 | **Agent identity is self-reported.** | Fine for v0; harden with signed agent tokens later. Don't market rock-solid identity. |
| 5 | ~~rmcp hook seam unconfirmed~~ **RESOLVED.** | **Viable** (rmcp 1.2.0). Wrap a delegating `ServerHandler` newtype `Instrumented<H>(H)`, override `call_tool` (RPITIT → `async move { … inner.call_tool().await }`), delegate `get_tool`/`list_tools`/`get_info`/etc. The `#[tool_handler]` macro generates `call_tool` as a normal inner method, so wrapping is transparent and routes into per-tool dispatch. Wire via `StreamableHttpService::new(\|\| Ok(Instrumented(RiftMcp::new(...))), …)`. |
| 6 | **Rust SDK market is thin.** | Its value is dogfood + reference + clean boundaries. Market opens with the FastMCP port — don't conflate. |
| 7 | **Pricing / quota.** Do agent actions burn `TrackEvent` quota or a new meter? | Decide before v0 ships the ingest; it's a pricing decision dressed as a schema field. |
| 8 | **TS internal fragility** (until #1238). | Pin + CI-test; facade absorbs the churn. |
| 9 | **MCPcat owns the observability wedge** and could move down-funnel into attribution (§4.1). | Don't compete on capture (commoditizing to free). Win on the deferred-deeplink + web→app + conversion engine — hard infra they'd build from scratch. Reassess partner-vs-compete. |

---

## 13. What each phase proves (success criteria)

- **v0:** one line instruments any rmcp server; agent actions land attributed; one end-to-end conversion row exists. *The engine and hook pattern are real.*
- **v1:** a real agent-originated install → conversion is correctly credited to the agent across the web→app seam. *The moat is real.*
- **v2:** a design partner reads the unified cross-agent funnel and says "no one else shows me this." *The product is real.*

---

## Appendix — key existing code touchpoints

| Purpose | Path |
|---|---|
| MCP server / auth | `server/src/mcp/server.rs` (`RiftMcp`, `auth_context`, `call_tool` dispatch) |
| Link resolve (public) | `server/src/api/links/routes.rs` (`resolve_link`, `do_resolve`, `record_click`) |
| Attribution model | `server/src/services/links/models.rs` (`AttributionEvent`, `AttributionEventMeta`, `CreditedLinks`) |
| Attribution storage / chain | `server/src/services/links/repo.rs` (`record_attribute_event`, `credited_links_for_user`) |
| Conversions pipeline | `server/src/services/conversions/service.rs` (`ingest`, `ingest_parsed`) |
| Conversion payload | `server/src/core/models.rs` (`ConversionEventPayload`) |
| Lifecycle (install binding) | `server/src/api/lifecycle/routes.rs` (`lifecycle_attribute`, `lifecycle_identify`) |
| Public IDs | `server/src/core/public_id/` (markers + `Id<P>`) |
