# @riftlinks/sdk

Rift web SDK — automatic deep link click tracking with zero configuration per link.

## Install

```bash
npm install @riftlinks/sdk
```

## Usage

```typescript
import { Rift } from '@riftlinks/sdk';

Rift.init("pk_live_YOUR_KEY", { domain: "go.yourcompany.com" });
```

That's it. Every `<a href="https://go.yourcompany.com/...">` on your page is automatically tracked. No attributes, no event handlers, no per-link setup.

### Script tag

```html
<script src="https://api.riftl.ink/sdk/rift.js"></script>
<script>
  Rift.init("pk_live_YOUR_KEY", { domain: "go.yourcompany.com" });
</script>
```

## API

### `Rift.init(publishableKey, opts?)`

Initialize the SDK. Pass `domain` to enable automatic click tracking.

| Param | Type | Description |
|-------|------|-------------|
| `publishableKey` | `string` | Your publishable key (`pk_live_` prefix) |
| `opts.domain` | `string` | Your custom link domain (e.g. `go.yourcompany.com`) |
| `opts.baseUrl` | `string` | API base URL (default: `https://api.riftl.ink`) |

### `Rift.click(linkId, opts?)`

Manually record a click. Not needed when using domain-based auto-tracking.

| Param | Type | Description |
|-------|------|-------------|
| `linkId` | `string` | The link ID to track |
| `opts.domain` | `string` | Custom domain for clipboard URL |

### `Rift.getLink(linkId, opts?)`

Fetch link data (metadata, destinations, agent context) without navigating.

```typescript
const link = await Rift.getLink("summer-sale");
console.log(link.agent_context);
```

## How it works

1. `Rift.init()` registers a click listener on `document`
2. When a user clicks any `<a>` tag whose `href` matches your domain, the SDK fires a `sendBeacon` to record the click
3. Navigation proceeds normally — Universal Links open the app if installed, otherwise the landing page loads
4. Click tracking is fire-and-forget and never blocks navigation

## Links

- [Documentation](https://www.riftl.ink/docs/web-sdk)
- [API Reference](https://www.riftl.ink/api-reference)
- [GitHub](https://github.com/saltyskip/rift)
