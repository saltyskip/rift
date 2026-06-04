"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

interface Me {
  user: {
    id: string;
    email: string;
    verified: boolean;
    is_owner: boolean;
  };
  tenant: { id: string };
}

interface SecretKey {
  id: string;
  key_prefix: string;
  created_by: string;
  created_at: string;
}

type LoadState =
  | { kind: "loading" }
  | { kind: "ready"; me: Me; keys: SecretKey[] }
  | { kind: "error"; message: string };

type ModalState =
  | { kind: "closed" }
  | { kind: "creating" }
  | { kind: "revealed"; key: string }
  | { kind: "error"; message: string };

export function AccountDashboard() {
  const router = useRouter();
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [modal, setModal] = useState<ModalState>({ kind: "closed" });
  // Bumped after each key create/revoke; useEffect below re-fetches on change.
  const [refreshTick, setRefreshTick] = useState(0);

  useEffect(() => {
    let alive = true;
    (async () => {
      const meResp = await fetch(`${API_URL}/v1/auth/me`, {
        credentials: "include",
      });
      if (!alive) return;
      if (meResp.status === 401) {
        router.replace("/signin?next=/account");
        return;
      }
      if (!meResp.ok) {
        setState({
          kind: "error",
          message: "Failed to load your account. Try refreshing.",
        });
        return;
      }
      const me: Me = await meResp.json();
      if (!alive) return;

      const keysResp = await fetch(`${API_URL}/v1/auth/secret-keys`, {
        credentials: "include",
      });
      if (!alive) return;
      if (!keysResp.ok) {
        setState({
          kind: "error",
          message:
            "Loaded your account but couldn't fetch keys. Try refreshing.",
        });
        return;
      }
      const keysBody: { keys: SecretKey[] } = await keysResp.json();
      if (!alive) return;

      setState({ kind: "ready", me, keys: keysBody.keys });
    })();
    return () => {
      alive = false;
    };
  }, [router, refreshTick]);

  function triggerRefresh() {
    setRefreshTick((n) => n + 1);
  }

  async function onSignOut() {
    await fetch(`${API_URL}/v1/auth/signout`, {
      method: "POST",
      credentials: "include",
    });
    router.push("/");
  }

  async function onCreateKey() {
    setModal({ kind: "creating" });
    try {
      const resp = await fetch(`${API_URL}/v1/auth/secret-keys/issue`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        credentials: "include",
        body: JSON.stringify({}),
      });
      if (resp.status === 409) {
        const data = await resp.json().catch(() => ({}));
        setModal({
          kind: "error",
          message:
            data.error ??
            "Key limit reached (max 5 per tenant). Revoke an unused one first.",
        });
        return;
      }
      if (!resp.ok) {
        const data = await resp.json().catch(() => ({}));
        setModal({
          kind: "error",
          message: data.error ?? "Failed to create key. Try again.",
        });
        return;
      }
      const body: { key: string } = await resp.json();
      setModal({ kind: "revealed", key: body.key });
      // Refresh keys list in the background so the prefix shows up after close.
      triggerRefresh();
    } catch {
      setModal({
        kind: "error",
        message: "Network error. Check your connection and retry.",
      });
    }
  }

  async function onRevokeKey(keyId: string) {
    const ok = window.confirm(
      "Revoke this key? Any service using it will start failing immediately.",
    );
    if (!ok) return;
    const resp = await fetch(`${API_URL}/v1/auth/secret-keys/${keyId}`, {
      method: "DELETE",
      credentials: "include",
    });
    if (!resp.ok) {
      const data = await resp.json().catch(() => ({}));
      window.alert(data.error ?? "Failed to revoke key.");
      return;
    }
    triggerRefresh();
  }

  if (state.kind === "loading") {
    return (
      <main className="pt-24 pb-20 px-6 min-h-[70vh]">
        <div className="mx-auto max-w-2xl">
          <p className="text-[14px] text-[#71717a]">Loading…</p>
        </div>
      </main>
    );
  }

  if (state.kind === "error") {
    return (
      <main className="pt-24 pb-20 px-6 min-h-[70vh]">
        <div className="mx-auto max-w-2xl">
          <div className="rounded-xl border border-red-500/30 bg-red-500/[0.05] p-5">
            <p className="text-[12px] font-mono text-red-400 uppercase tracking-widest mb-2">
              Error
            </p>
            <p className="text-[14px] text-[#fafafa]">{state.message}</p>
          </div>
        </div>
      </main>
    );
  }

  return (
    <main className="pt-24 pb-20 px-6 min-h-[70vh]">
      <div className="mx-auto max-w-2xl">
        <div className="flex items-start justify-between mb-12">
          <div>
            <p className="text-[12px] font-mono text-[#52525b] uppercase tracking-widest mb-2">
              Account
            </p>
            <p className="text-[20px] font-medium text-[#fafafa]">
              {state.me.user.email}
            </p>
          </div>
          <button
            onClick={onSignOut}
            className="text-[13px] text-[#71717a] hover:text-[#fafafa] border border-[#222225] hover:border-[#2dd4bf]/30 rounded-lg px-4 py-2 transition-colors"
          >
            Sign out
          </button>
        </div>

        <section className="mb-12">
          <div className="flex items-center justify-between mb-4">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest">
              API keys
            </p>
            <button
              onClick={onCreateKey}
              className="text-[13px] font-medium bg-[#2dd4bf] text-[#042f2e] px-3.5 py-1.5 rounded-lg hover:bg-[#5eead4] transition-colors"
            >
              + Create API key
            </button>
          </div>

          {state.keys.length === 0 ? (
            <div className="rounded-xl border border-[#222225] bg-[#111113] p-8 text-center">
              <p className="text-[14px] text-[#71717a]">
                You don&rsquo;t have any API keys yet.
              </p>
              <p className="text-[13px] text-[#52525b] mt-1">
                Create one to start using Rift from the CLI, your backend, or MCP.
              </p>
            </div>
          ) : (
            <ul className="rounded-xl border border-[#222225] bg-[#111113] divide-y divide-[#222225]">
              {state.keys.map((k) => (
                <li
                  key={k.id}
                  className="flex items-center justify-between px-5 py-4"
                >
                  <div>
                    <p className="text-[13px] font-mono text-[#fafafa]">
                      {k.key_prefix}
                    </p>
                    <p className="text-[12px] text-[#52525b] mt-1">
                      Created {formatDate(k.created_at)}
                    </p>
                  </div>
                  <button
                    onClick={() => onRevokeKey(k.id)}
                    className="text-[13px] text-[#71717a] hover:text-red-400 transition-colors"
                  >
                    Revoke
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-5">
          <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-2">
            Quick start
          </p>
          <pre className="text-[12px] font-mono text-[#a1a1aa] overflow-x-auto">
            <code>{`# Install the CLI\ncurl -fsSL https://raw.githubusercontent.com/saltyskip/rift/main/client/cli/install.sh | sh\n\n# Sign in (opens your browser)\nrift login\n\n# Create your first link\nrift links create --web-url https://yourapp.com`}</code>
          </pre>
        </section>
      </div>

      {modal.kind !== "closed" && (
        <KeyCreatedModal modal={modal} onClose={() => setModal({ kind: "closed" })} />
      )}
    </main>
  );
}

function KeyCreatedModal({
  modal,
  onClose,
}: {
  modal: ModalState;
  onClose: () => void;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center px-6"
      style={{ background: "rgba(0,0,0,0.7)" }}
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-xl border border-[#222225] bg-[#111113] p-6"
        onClick={(e) => e.stopPropagation()}
      >
        {modal.kind === "creating" && (
          <p className="text-[14px] text-[#71717a]">Creating key…</p>
        )}
        {modal.kind === "error" && (
          <>
            <p className="text-[12px] font-mono text-red-400 uppercase tracking-widest mb-2">
              Error
            </p>
            <p className="text-[14px] text-[#fafafa] mb-5">{modal.message}</p>
            <button
              onClick={onClose}
              className="w-full h-10 rounded-lg border border-[#222225] text-[14px] text-[#fafafa] hover:border-[#2dd4bf]/30 transition-colors"
            >
              Close
            </button>
          </>
        )}
        {modal.kind === "revealed" && (
          <>
            <p className="text-[12px] font-mono text-[#2dd4bf] uppercase tracking-widest mb-2">
              Key created
            </p>
            <p className="text-[13px] text-[#71717a] mb-4">
              Save this now — we won&rsquo;t show it again. If you lose it,
              revoke it and create a new one.
            </p>
            <CopyableKey value={modal.key} />
            <button
              onClick={onClose}
              className="mt-5 w-full h-10 rounded-lg bg-[#2dd4bf] text-[#042f2e] text-[14px] font-semibold hover:bg-[#5eead4] transition-colors"
            >
              Done
            </button>
          </>
        )}
      </div>
    </div>
  );
}

function CopyableKey({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="rounded-lg border border-[#222225] bg-[#0a0a0b] p-3 flex items-center gap-3">
      <code className="text-[12px] font-mono text-[#fafafa] break-all flex-1">
        {value}
      </code>
      <button
        onClick={async () => {
          try {
            await navigator.clipboard.writeText(value);
            setCopied(true);
            setTimeout(() => setCopied(false), 1500);
          } catch {
            // Clipboard write can fail in some browsers; user can copy manually.
          }
        }}
        className="text-[12px] text-[#2dd4bf] hover:text-[#5eead4] transition-colors shrink-0"
      >
        {copied ? "Copied" : "Copy"}
      </button>
    </div>
  );
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return iso;
  }
}
