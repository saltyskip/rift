"use client";

import { useState, type FormEvent } from "react";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

type Intent = "subscribe" | "portal";

interface Props {
  intent: Intent;
  tier?: "pro" | "business" | "scale";
  label?: string;
  submitLabel?: string;
  placeholder?: string;
  note?: string;
}

type State =
  | { kind: "idle" }
  | { kind: "submitting" }
  | { kind: "sent"; email: string }
  | { kind: "error"; message: string };

export function MagicLinkForm({
  intent,
  tier,
  label = "Email address",
  submitLabel = "Continue",
  placeholder = "you@company.com",
  note,
}: Props) {
  const [email, setEmail] = useState("");
  const [state, setState] = useState<State>({ kind: "idle" });

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const trimmed = email.trim();
    if (!trimmed || !trimmed.includes("@")) {
      setState({ kind: "error", message: "Enter a valid email address." });
      return;
    }
    setState({ kind: "submitting" });
    try {
      const body: Record<string, unknown> = { email: trimmed, intent };
      if (tier) body.tier = tier;
      const resp = await fetch(`${API_URL}/v1/billing/magic-link`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      if (resp.status === 429) {
        setState({
          kind: "error",
          message: "Too many requests. Try again in a bit.",
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
          We sent a secure link to <span className="font-medium">{state.email}</span>.
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
          {label}
        </span>
        <input
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder={placeholder}
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
        {state.kind === "submitting" ? "Sending link…" : submitLabel}
      </button>
      {state.kind === "error" && (
        <p className="text-[13px] text-red-400">{state.message}</p>
      )}
      {note && state.kind !== "error" && (
        <p className="text-[12px] text-[#52525b] leading-relaxed">{note}</p>
      )}
    </form>
  );
}
