const DEFAULT_BASE = "https://api.riftl.ink";

// Retained for API compatibility and potential future authenticated calls. The
// web SDK currently makes no authenticated requests (clipboard is local;
// getLink hits the public resolve endpoint), so this is stored but unused.
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
            // Stamp the clipboard for deferred attribution (we have the gesture)
            // and let the browser navigate. The web SDK never records the click
            // itself — the navigation hits the resolver, which is the single
            // source of truth for web clicks. (#194)
            copyLinkToClipboard(linkId, _domain!);
          }
        }
      },
      true,
    );
  }
}

/**
 * Stamp the link URL onto the clipboard so a freshly-installed app can read it
 * for deferred attribution. Must run inside a user gesture. No-op when the
 * Clipboard API is unavailable.
 */
function copyLinkToClipboard(linkId: string, domain?: string | null): void {
  if (typeof navigator === "undefined" || !navigator.clipboard) return;
  const d =
    domain || _domain || (typeof location !== "undefined" ? location.hostname : null);
  if (d) {
    navigator.clipboard.writeText("https://" + d + "/" + linkId).catch(() => {});
  }
}

/**
 * Stamp the deferred-attribution clipboard URL for a link. Use on a button or
 * link the user taps to go install the app.
 *
 * The web SDK intentionally does **not** record the click: when the user
 * navigates to the link it resolves through the server, which is the single
 * counter for web clicks. (To record a click that never passes through the
 * resolver — e.g. server-side — call `POST /v1/lifecycle/click` directly.)
 */
export function click(linkId: string, opts?: RiftClickOptions): void {
  copyLinkToClipboard(linkId, opts?.domain);
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
