"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

type Props =
  | {
      intent: "subscribe";
      tier: "pro" | "business" | "scale";
      tierName: string;
    }
  | {
      intent: "portal";
    };

type State =
  | { kind: "loading" }
  | { kind: "signed-out" }
  | { kind: "signed-in"; email: string }
  | { kind: "redirecting" }
  | { kind: "error"; message: string };

export function BillingCta(props: Props) {
  const router = useRouter();
  const [state, setState] = useState<State>({ kind: "loading" });

  const nextPath =
    props.intent === "subscribe"
      ? `/checkout?tier=${props.tier}`
      : "/manage";
  const signinHref = `/signin?next=${encodeURIComponent(nextPath)}`;

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const resp = await fetch(`${API_URL}/v1/auth/me`, {
          credentials: "include",
        });
        if (!alive) return;
        if (resp.status === 401) {
          setState({ kind: "signed-out" });
          return;
        }
        if (!resp.ok) {
          setState({
            kind: "error",
            message: "Couldn't check your session. Try refreshing.",
          });
          return;
        }
        const me = await resp.json();
        if (!alive) return;
        setState({ kind: "signed-in", email: me?.user?.email ?? "" });
      } catch {
        if (!alive) return;
        setState({
          kind: "error",
          message: "Network error. Check your connection and retry.",
        });
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  async function go() {
    setState({ kind: "redirecting" });
    const endpoint =
      props.intent === "subscribe"
        ? `${API_URL}/v1/billing/stripe/checkout?tier=${props.tier}`
        : `${API_URL}/v1/billing/stripe/portal`;
    try {
      const resp = await fetch(endpoint, {
        method: "POST",
        credentials: "include",
      });
      if (resp.status === 401) {
        // Session expired between mount and click — restart at signin.
        router.replace(signinHref);
        return;
      }
      if (!resp.ok) {
        const data = await resp.json().catch(() => ({}));
        setState({
          kind: "error",
          message: data?.error ?? "Something went wrong. Try again.",
        });
        return;
      }
      const data = await resp.json();
      const url: string | undefined =
        props.intent === "subscribe" ? data?.checkout_url : data?.portal_url;
      if (!url) {
        setState({
          kind: "error",
          message: "Server didn't return a redirect URL. Try again.",
        });
        return;
      }
      window.location.assign(url);
    } catch {
      setState({
        kind: "error",
        message: "Network error. Check your connection and retry.",
      });
    }
  }

  if (state.kind === "loading") {
    return (
      <p className="text-[13px] text-[#52525b]">Checking your session…</p>
    );
  }

  if (state.kind === "signed-out") {
    return (
      <div className="space-y-3">
        <a
          href={signinHref}
          className="block text-center w-full h-11 leading-[44px] rounded-lg bg-[#2dd4bf] text-[#042f2e] text-[14px] font-semibold hover:bg-[#5eead4] transition-colors"
        >
          {props.intent === "subscribe"
            ? `Sign in to start ${props.tierName}`
            : "Sign in to manage billing"}
        </a>
        <p className="text-[12px] text-[#52525b] leading-relaxed">
          You&rsquo;ll come right back here after signing in.
        </p>
      </div>
    );
  }

  const buttonLabel =
    state.kind === "redirecting"
      ? "Redirecting…"
      : props.intent === "subscribe"
        ? "Continue to Stripe"
        : "Open billing portal";

  return (
    <div className="space-y-3">
      {state.kind === "signed-in" && state.email && (
        <p className="text-[12px] font-mono text-[#52525b] uppercase tracking-widest">
          Signed in as <span className="text-[#a1a1aa]">{state.email}</span>
        </p>
      )}
      <button
        type="button"
        onClick={go}
        disabled={state.kind === "redirecting"}
        className="w-full h-11 rounded-lg bg-[#2dd4bf] text-[#042f2e] text-[14px] font-semibold hover:bg-[#5eead4] transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
      >
        {buttonLabel}
      </button>
      {state.kind === "error" && (
        <p className="text-[13px] text-red-400">{state.message}</p>
      )}
    </div>
  );
}
