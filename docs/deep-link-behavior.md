# Deep Link Behavior

How Rift links resolve across platforms, depending on where the link is clicked and whether the app is installed.

## Core principles

1. **App installed → the OS opens the app before Rift is involved.** Every path on your verified domain is a Universal Link (iOS/macOS) / App Link (Android) — Rift serves the `apple-app-site-association` and `assetlinks.json` automatically. A tap from anywhere external (Messages, email, social, another site) opens the app directly; the resolver never runs.
2. **App not installed → the request reaches the Rift resolver,** which decides per platform: a zero-flash redirect, or the landing page.
3. **`redirect_mode` controls the bounce** (see below). Crawlers and clients with no user-activation signal always get the landing page, so link unfurls/previews are never broken.

## Recommended link configuration

| Field | Value | Purpose |
|-------|-------|---------|
| `ios_store_url` | App Store URL | iOS fallback (no app) |
| `android_store_url` | Play Store URL | Android fallback (no app) |
| `macos_store_url` | Mac App Store URL | macOS fallback (no app) |
| `windows_store_url` | Microsoft Store URL | Windows fallback (no app) |
| `web_url` | Your site / store page | Desktop & unknown fallback |
| `redirect_mode` | `auto` (default) or `off` | Auto-bounce eligible visitors vs. always show the landing page |
| App registered | `POST /v1/apps` (bundle/team id, package/sha256) | Enables Universal Links / App Links |
| Custom domain | Verified via `POST /v1/domains` | AASA + assetlinks.json served automatically |

## `redirect_mode`

- **`auto`** (new links default to this) — eligible visitors are bounced straight to their destination; everyone else lands. Tenant-wide default is `default_redirect_mode`; a per-link `redirect_mode` overrides it. Existing links with no value behave as `off`.
- **`off`** — always show the landing page (visitor taps to continue).

The auto-bounce (a `307`) only fires for a **human navigation** — detected by the `Sec-Fetch-User: ?1` request header. Crawlers/bots and clients that don't send it fall through to the landing page (which carries the OpenGraph/JSON-LD for unfurls). No bot allowlist is involved.

---

## Matrix — external click (the common case)

Link clicked somewhere that isn't your domain: a message, email, social post, another website.

### App **installed**

| Platform | What happens | Click + attribution |
|----------|-------------|---------------------|
| **iOS** | Universal Link → **app opens directly** (Rift not hit) | The app reads the URL → `link_id` → `POST /v1/lifecycle/attribute`, and records the touch via `POST /v1/lifecycle/click`. Deterministic. |
| **Android** | App Link → **app opens directly** | same |
| **macOS** | Opens the Mac app *only if* the Mac app's ID is in the AASA; otherwise reaches Rift → Mac App Store | clipboard (best-effort) |
| **Windows** | No web→app association → reaches Rift → `307` Microsoft Store | MS Store `cid` |

### App **not installed** (`redirect_mode = auto`, human navigation)

| Platform | What happens | Attribution (credited on later install) |
|----------|-------------|------------------------------------------|
| **iOS** | **Lands** → tap "Get" → App Store | **Clipboard** — stamped on the tap; the app reads the pasteboard on first launch |
| **Android** | **`307` → Play Store** (`?referrer=rift_link`) — no landing page | **Play install referrer** (survives the redirect) |
| **macOS** | **Lands** → tap → Mac App Store | **Clipboard** (best-effort) |
| **Windows** | **`307` → Microsoft Store** (`?cid`) | **MS Store `cid`** |
| **Linux / other** | **`307` → `web_url`** (`?rift_link`) | web query |

**Why iOS/macOS land but Android/Windows bounce:** Apple stores carry no install referrer, so deferred attribution depends on the **clipboard**, and the browser only allows a clipboard write inside a **user-gesture tap** — hence the landing button. Android's Play **referrer** and the Microsoft Store **`cid`** ride the store URL and survive a plain `307`, so no tap (and no landing) is needed. macOS also lands so the page can correct an **iPad** (which reports as a Mac via its User-Agent) to the iOS App Store.

---

## On your own domain

A button on `https://go.yourcompany.com/...` with `Rift.click()` on the handler. Same-domain taps don't fire Universal/App Links, so the click reaches Rift. Add `?redirect=1` to the `<a href>` to skip the landing page and go straight to the destination — `Rift.click()` has already stamped the clipboard at the tap, and on iOS the App Store URL opens the app if installed. (`?redirect=1` is the explicit, gesture-backed path; `redirect_mode=auto` is the implicit one for everywhere else.)

The web SDK does **not** record the click — the navigation hits the resolver, which is the single counter (see below).

---

## Click tracking & attribution

- **The resolver records every click, once**, at resolve time — before any redirect/landing branch. It is the single source of truth for web clicks.
- **The web SDK never records clicks.** Auto-track only stamps the clipboard; `Rift.click()` is clipboard-only. (For a click that never passes through the resolver — e.g. server-side — call `POST /v1/lifecycle/click` directly; the mobile SDK uses it when the app is opened directly by a Universal/App Link.)
- **Attribution channels:** deterministic via the URL the OS hands an installed app; **Android** Play referrer; **Windows** MS Store `cid`; **web** `rift_link`; **iOS/macOS** clipboard (deferred, gesture-required).

---

## Edge cases

- **URL typed/pasted in the address bar** — Universal/App Links don't fire; the landing page loads (no auto-bounce without `Sec-Fetch-User`).
- **In-app browsers** (Instagram, Facebook, TikTok, X) — Universal/App Links are unreliable; the user usually reaches the landing page. Many of these webviews also omit `Sec-Fetch-User`, so they land rather than bounce — which gives an installed user a second chance to open the app via the landing button.
- **iPad in desktop mode** — reports a Mac User-Agent. macOS requests with an iOS target land so the page can detect touch and route the iPad to the **iOS** App Store; a real Mac gets the Mac App Store. (If a tenant has an alternate domain configured, a *not-installed* iPad can fall through to `web_url` — a known limitation of header-only detection.)
- **macOS / Windows don't auto-launch an installed desktop app** — there's no desktop deep-link/trampoline in v1 (macOS Universal Links + a `windows_deep_link` are deferred); installed desktop users get the store listing's "Open" button.
