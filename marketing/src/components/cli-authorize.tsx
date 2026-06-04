"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

// The CLI opens `${API}/v1/auth/cli/start?redirect_uri=…&state=…`, which bounces
// the browser here. This page reuses the normal dashboard session (shared
// cookie): if the user isn't signed in we send them through `/signin?next=…`
// and they land back here. On approval we POST to the API to mint a fresh
// `rift-cli` session, then navigate the browser to the CLI's loopback listener
// carrying the token.

interface Me {
  user: { id: string; email: string; verified: boolean; is_owner: boolean };
  tenant: { id: string };
}

type State =
  | { kind: "loading" }
  | { kind: "ready"; email: string }
  | { kind: "approving" }
  | { kind: "done" }
  | { kind: "error"; message: string };

interface Params {
  redirectUri: string;
  state?: string;
}

// Mirror the server's loopback guard: only an http loopback host may receive
// the token.
function isLoopback(uri: string): boolean {
  try {
    const u = new URL(uri);
    return (
      u.protocol === "http:" &&
      (u.hostname === "127.0.0.1" ||
        u.hostname === "localhost" ||
        u.hostname === "[::1]" ||
        u.hostname === "::1")
    );
  } catch {
    return false;
  }
}

function readParams(): Params | null {
  if (typeof window === "undefined") return null;
  const q = new URLSearchParams(window.location.search);
  const redirectUri = q.get("redirect_uri");
  if (!redirectUri) return null;
  return { redirectUri, state: q.get("state") ?? undefined };
}

export function CliAuthorize() {
  const router = useRouter();
  const [state, setState] = useState<State>({ kind: "loading" });
  const [params, setParams] = useState<Params | null>(null);

  useEffect(() => {
    let alive = true;
    // All state updates live inside the async body so none run synchronously in
    // the effect (avoids the cascading-render lint).
    (async () => {
      const p = readParams();
      if (!p) {
        if (alive) {
          setState({
            kind: "error",
            message: "Missing redirect_uri. Re-run `rift login` from your terminal.",
          });
        }
        return;
      }
      if (!isLoopback(p.redirectUri)) {
        if (alive) {
          setState({
            kind: "error",
            message: "This sign-in request has an invalid destination and was blocked.",
          });
        }
        return;
      }
      if (alive) setParams(p);

      const meResp = await fetch(`${API_URL}/v1/auth/me`, {
        credentials: "include",
      });
      if (!alive) return;
      if (meResp.status === 401) {
        const next = `/cli/authorize${window.location.search}`;
        router.replace(`/signin?next=${encodeURIComponent(next)}`);
        return;
      }
      if (!meResp.ok) {
        setState({
          kind: "error",
          message: "Couldn't verify your session. Try refreshing.",
        });
        return;
      }
      const me: Me = await meResp.json();
      if (!alive) return;
      setState({ kind: "ready", email: me.user.email });
    })();

    return () => {
      alive = false;
    };
  }, [router]);

  async function onApprove() {
    if (!params) return;
    setState({ kind: "approving" });
    try {
      const resp = await fetch(`${API_URL}/v1/auth/cli/authorize`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ redirect_uri: params.redirectUri }),
      });
      if (!resp.ok) {
        const data = await resp.json().catch(() => ({}));
        setState({
          kind: "error",
          message: data.error ?? "Failed to authorize the CLI. Try again.",
        });
        return;
      }
      const body: { token: string } = await resp.json();
      const dest = new URL(params.redirectUri);
      dest.searchParams.set("token", body.token);
      if (params.state) dest.searchParams.set("state", params.state);
      setState({ kind: "done" });
      // Hand the token to the CLI's loopback listener via a top-level
      // navigation (loopback http is exempt from mixed-content blocking).
      window.location.href = dest.toString();
    } catch {
      setState({
        kind: "error",
        message: "Network error. Check your connection and retry.",
      });
    }
  }

  return (
    <main className="pt-24 pb-20 px-6 min-h-[70vh]">
      <div className="mx-auto max-w-md">
        <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
          Command line
        </p>
        <h1 className="text-3xl font-semibold tracking-[-0.03em] mb-4">
          Authorize the Rift CLI.
        </h1>

        {state.kind === "loading" && (
          <p className="text-[14px] text-[#71717a]">Checking your session…</p>
        )}

        {(state.kind === "ready" || state.kind === "approving") && (
          <div className="rounded-xl border border-[#222225] bg-[#111113] p-6">
            <p className="text-[14px] text-[#a1a1aa] mb-1 leading-relaxed">
              The Rift command-line tool on this device is requesting to sign in
              as:
            </p>
            <p className="text-[15px] font-medium text-[#fafafa] mb-6">
              {state.kind === "ready" ? state.email : "…"}
            </p>
            <button
              onClick={onApprove}
              disabled={state.kind === "approving"}
              className="w-full h-11 rounded-lg bg-[#2dd4bf] text-[#042f2e] text-[14px] font-semibold hover:bg-[#5eead4] transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
            >
              {state.kind === "approving" ? "Authorizing…" : "Authorize this device"}
            </button>
            <p className="text-[12px] text-[#52525b] mt-3 leading-relaxed">
              Only approve this if you just started <code>rift login</code> in
              your terminal. The CLI gets its own session that you can revoke any
              time.
            </p>
          </div>
        )}

        {state.kind === "done" && (
          <div className="rounded-xl border border-[#2dd4bf]/30 bg-[#2dd4bf]/[0.05] p-6">
            <p className="text-[12px] font-mono text-[#2dd4bf] uppercase tracking-widest mb-2">
              Authorized
            </p>
            <p className="text-[14px] text-[#fafafa]">
              You can return to your terminal — the CLI is now signed in.
            </p>
          </div>
        )}

        {state.kind === "error" && (
          <div className="rounded-xl border border-red-500/30 bg-red-500/[0.05] p-5">
            <p className="text-[12px] font-mono text-red-400 uppercase tracking-widest mb-2">
              Error
            </p>
            <p className="text-[14px] text-[#fafafa]">{state.message}</p>
          </div>
        )}
      </div>
    </main>
  );
}
