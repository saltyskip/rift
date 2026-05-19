"use client";

import { useEffect, useState, type FormEvent } from "react";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

// `?error=<code>` codes the OAuth callback redirects back with. Mapped to
// user-facing toasts so the magic-link form can surface OAuth failures
// without a separate component.
const OAUTH_ERROR_MESSAGES: Record<string, string> = {
  oauth_state_invalid:
    "Your sign-in link expired or was already used. Try again.",
  oauth_email_unverified:
    "Your email isn't verified with that provider. Verify it or use email sign-in.",
  oauth_no_email:
    "We couldn't get an email from that provider. Use email sign-in instead.",
  oauth_provider_error:
    "Sign-in with that provider failed. Try again or use email.",
  oauth_not_configured:
    "That provider isn't configured. Use email sign-in.",
  oauth_provider_unknown: "Unknown sign-in provider.",
  oauth_internal: "Something went wrong. Try again.",
  rate_limited: "Too many sign-in requests. Try again in a bit.",
  link_expired: "Your sign-in link expired. Try again.",
};

type State =
  | { kind: "idle" }
  | { kind: "submitting" }
  | { kind: "sent"; email: string }
  | { kind: "error"; message: string };

function initialStateFromUrl(): State {
  if (typeof window === "undefined") return { kind: "idle" };
  const code = new URLSearchParams(window.location.search).get("error");
  if (!code) return { kind: "idle" };
  const message = OAUTH_ERROR_MESSAGES[code] ?? "Sign-in failed. Try again.";
  return { kind: "error", message };
}

export function SignInForm({ next }: { next?: string } = {}) {
  const [email, setEmail] = useState("");
  // Initialize from `?error=<code>` so an OAuth callback redirect surfaces a
  // toast on the magic-link form without a separate component. Lazy
  // initializer (not useEffect+setState) keeps the render synchronous and
  // avoids the "cascading renders" lint rule.
  const [state, setState] = useState<State>(initialStateFromUrl);

  // Strip `?error=<code>` from the URL once after mount so reloading the
  // page doesn't re-show the toast. This is a side effect (external system =
  // browser history), not a setState — safe to do in useEffect.
  useEffect(() => {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams(window.location.search);
    if (!params.has("error")) return;
    params.delete("error");
    const next = params.toString();
    const url =
      window.location.pathname + (next ? `?${next}` : "") + window.location.hash;
    window.history.replaceState(null, "", url);
  }, []);

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const trimmed = email.trim();
    if (!trimmed || !trimmed.includes("@")) {
      setState({ kind: "error", message: "Enter a valid email address." });
      return;
    }
    setState({ kind: "submitting" });
    try {
      const body: Record<string, unknown> = { email: trimmed };
      if (next) body.next = next;
      const resp = await fetch(`${API_URL}/v1/auth/signin`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        credentials: "include",
        body: JSON.stringify(body),
      });
      if (resp.status === 429) {
        setState({
          kind: "error",
          message: "Too many sign-in requests. Try again in a bit.",
        });
        return;
      }
      if (!resp.ok) {
        const data = await resp.json().catch(() => ({}));
        setState({
          kind: "error",
          message: data.error ?? "Something went wrong. Try again.",
        });
        return;
      }
      setState({ kind: "sent", email: trimmed });
    } catch {
      setState({
        kind: "error",
        message: "Network error. Check your connection and retry.",
      });
    }
  }

  if (state.kind === "sent") {
    return (
      <div className="rounded-xl border border-[#2dd4bf]/30 bg-[#2dd4bf]/[0.05] p-6">
        <p className="text-[12px] font-mono text-[#2dd4bf] uppercase tracking-widest mb-2">
          Check your inbox
        </p>
        <p className="text-[15px] text-[#fafafa] mb-1">
          We sent a sign-in link to{" "}
          <span className="font-medium">{state.email}</span>.
        </p>
        <p className="text-[13px] text-[#71717a]">
          The link expires in 15 minutes and can only be used once.
        </p>
      </div>
    );
  }

  return (
    <form onSubmit={onSubmit} className="space-y-3">
      <label className="block">
        <span className="block text-[12px] font-mono text-[#52525b] uppercase tracking-widest mb-2">
          Email address
        </span>
        <input
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="you@company.com"
          required
          disabled={state.kind === "submitting"}
          className="w-full h-11 rounded-lg border border-[#222225] bg-[#111113] px-4 text-[14px] text-[#fafafa] placeholder:text-[#52525b] outline-none focus:border-[#2dd4bf]/50 transition-colors disabled:opacity-60"
        />
      </label>
      <button
        type="submit"
        disabled={state.kind === "submitting"}
        className="w-full h-11 rounded-lg bg-[#2dd4bf] text-[#042f2e] text-[14px] font-semibold hover:bg-[#5eead4] transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
      >
        {state.kind === "submitting" ? "Sending link…" : "Email me a sign-in link"}
      </button>
      {state.kind === "error" && (
        <p className="text-[13px] text-red-400">{state.message}</p>
      )}
    </form>
  );
}
