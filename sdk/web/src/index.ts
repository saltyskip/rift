const DEFAULT_BASE = "https://api.riftl.ink";

let _publishableKey: string | null = null;
let _domain: string | null = null;
let _baseUrl: string = DEFAULT_BASE;
let _bound = false;

export interface RiftInitOptions {
  /** Your custom link domain (e.g. "go.yourcompany.com"). Enables auto-tracking. */
  domain?: string;
  /** API base URL. Default: https://api.riftl.ink */
  baseUrl?: string;
}

export interface RiftClickOptions {
  /** Custom domain for the clipboard URL. Defaults to the init domain. */
  domain?: string;
}

export interface RiftGetLinkOptions {
  /** API base URL override. */
  baseUrl?: string;
}

/**
 * Initialize the Rift SDK with your publishable key.
 * Pass `domain` to auto-track clicks on links matching your custom domain.
 */
export function init(publishableKey: string, opts?: RiftInitOptions): void {
  _publishableKey = publishableKey;
  if (opts?.baseUrl) _baseUrl = opts.baseUrl;
  if (opts?.domain) _domain = opts.domain;

  // Auto-track clicks on links matching the custom domain.
  if (!_bound && _domain && typeof document !== "undefined") {
    _bound = true;
    document.addEventListener(
      "click",
      (e: MouseEvent) => {
        const a = (e.target as Element)?.closest?.("a[href]") as HTMLAnchorElement | null;
        if (!a) return;
        const prefix = "https://" + _domain + "/";
        const href = a.href;
        if (href.indexOf(prefix) === 0) {
          const linkId = href.slice(prefix.length).split("?")[0].split("#")[0];
          if (linkId) {
            click(linkId, { domain: _domain! });
          }
        }
      },
      true,
    );
  }
}

/**
 * Record a click event. Fire-and-forget — does not block navigation.
 * Called automatically for links matching the custom domain.
 * Can also be called manually for programmatic use cases.
 */
export function click(linkId: string, opts?: RiftClickOptions): void {
  // Clipboard write — must happen while we have the user gesture.
  if (typeof navigator !== "undefined" && navigator.clipboard) {
    const domain = opts?.domain || _domain || (typeof location !== "undefined" ? location.hostname : null);
    if (domain) {
      navigator.clipboard.writeText("https://" + domain + "/" + linkId).catch(() => {});
    }
  }

  if (!_publishableKey) {
    console.warn("Rift: call Rift.init('pk_live_...') before Rift.click()");
    return;
  }

  const url = _baseUrl + "/v1/attribution/click?key=" + encodeURIComponent(_publishableKey);
  const body = JSON.stringify({ link_id: linkId });

  if (navigator.sendBeacon) {
    navigator.sendBeacon(url, new Blob([body], { type: "application/json" }));
  } else {
    fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body,
      keepalive: true,
    }).catch(() => {});
  }
}

/**
 * Fetch link data without navigating. Returns the link metadata,
 * destinations, and agent context.
 */
export function getLink(linkId: string, opts?: RiftGetLinkOptions): Promise<unknown> {
  const base = opts?.baseUrl || _baseUrl;
  return fetch(base + "/r/" + encodeURIComponent(linkId), {
    headers: { Accept: "application/json" },
  }).then((r) => r.json());
}

export const Rift = { init, click, getLink };
export default Rift;
