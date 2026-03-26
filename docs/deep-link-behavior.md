# Deep Link Behavior

How Rift links behave depending on where the link is hosted, the user's platform, and whether the app is installed.

## Recommended Link Configuration

| Field | Value | Purpose |
|-------|-------|---------|
| `ios_store_url` | App Store URL | Fallback for iOS users without the app |
| `android_store_url` | Play Store URL | Fallback for Android users without the app |
| `web_url` | Your website or store page | Desktop fallback |
| App registered | `POST /v1/apps` with bundle_id + team_id (iOS), package_name + sha256 (Android) | Enables Universal Links / App Links |
| Custom domain | Verified via `POST /v1/domains` | AASA and assetlinks.json served automatically |
| Publishable key | Created via `POST /v1/auth/publishable-keys` | Required for `Rift.click()` on your website |

---

## 1. Link is on your domain

The download button on your website uses `<a href="https://go.yourcompany.com/link-id">` with `Rift.click()` on the onClick handler. The `<a>` tag should include `?redirect=1` so the landing page skips its UI and goes straight to the store.

### iOS

| App Installed | What Happens | Click Tracked |
|---------------|-------------|---------------|
| Yes | Universal Link fires. App opens directly. Landing page never loads. | ✅ sendBeacon from your website |
| No | Landing page loads. Copies link URL to clipboard. Immediately redirects to App Store. | ✅ sendBeacon + server-side |

### Android

| App Installed | What Happens | Click Tracked |
|---------------|-------------|---------------|
| Yes | App Link fires (if assetlinks.json configured). App opens directly. | ✅ sendBeacon from your website |
| No | Landing page loads. Appends install referrer to Play Store URL. Immediately redirects to Play Store. | ✅ sendBeacon + server-side |

### Desktop

| What Happens | Click Tracked |
|-------------|---------------|
| Landing page loads. Immediately redirects to `web_url`. | ✅ sendBeacon + server-side |

---

## 2. Link is NOT on your domain

The link is shared externally — in an email, text message, social media post, chat, etc. There is no `Rift.click()` because rift.js is not loaded on external platforms. No `?redirect=1` — the full landing page is shown.

### iOS

| App Installed | What Happens | Click Tracked |
|---------------|-------------|---------------|
| Yes | Universal Link fires. App opens directly. Landing page never loads. | ❌ Not tracked (no JS, no server request) |
| No | Landing page loads. Shows branded page with App Store button. Copies link URL to clipboard for deferred deep linking. | ✅ Server-side |

### Android

| App Installed | What Happens | Click Tracked |
|---------------|-------------|---------------|
| Yes | App Link fires (if configured). App opens directly. | ❌ Not tracked |
| No | Landing page loads. Shows branded page with Play Store button. Play Store URL includes install referrer for deferred deep linking. | ✅ Server-side |

### Desktop

| What Happens | Click Tracked |
|-------------|---------------|
| Landing page loads. Shows branded page with download button linking to `web_url`. | ✅ Server-side |

---

## Edge Cases

### User types or pastes the URL in the address bar

Universal Links and App Links do NOT fire from the address bar on any platform. The landing page always loads and shows the full branded page. Click is tracked server-side.

### In-app browsers (Twitter, Instagram, LinkedIn)

Universal Links are unreliable in in-app browsers. The landing page will usually load. On iOS, a Smart App Banner (`<meta name="apple-itunes-app">`) can prompt the user to open in the real app. Click is tracked server-side.

### External link with app installed — click not tracked

When the app opens via Universal Link from an external source, neither sendBeacon nor the server records the click. To close this gap, the mobile SDK should send a retroactive click when the app opens via a Universal Link (i.e., the app receives a URL via `application(_:continue:restorationHandler:)` on iOS or the intent handler on Android).
