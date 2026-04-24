"use client";

import { useState, type FormEvent } from "react";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

type State =
  | { kind: "idle" }
  | { kind: "submitting" }
  | { kind: "sent"; email: string }
  | { kind: "error"; message: string };

export function FreeSignupForm() {
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
      const resp = await fetch(`${API_URL}/v1/auth/signup`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ email: trimmed }),
      });
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
          Verification sent
        </p>
        <p className="text-[15px] text-[#fafafa] mb-1">
          Check <span className="font-medium">{state.email}</span> to confirm your account and see your API key.
        </p>
        <p className="text-[13px] text-[#71717a]">
          The link expires in 24 hours.
        </p>
      </div>
    );
  }

  return (
    <form onSubmit={onSubmit} className="flex flex-col sm:flex-row gap-2">
      <input
        type="email"
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        placeholder="you@company.com"
        required
        disabled={state.kind === "submitting"}
        className="flex-1 h-11 rounded-lg border border-[#222225] bg-[#111113] px-4 text-[14px] text-[#fafafa] placeholder:text-[#52525b] outline-none focus:border-[#2dd4bf]/50 transition-colors disabled:opacity-60"
      />
      <button
        type="submit"
        disabled={state.kind === "submitting"}
        className="h-11 rounded-lg border border-[#222225] bg-[#111113] px-5 text-[14px] font-medium text-[#fafafa] hover:border-[#2dd4bf]/30 transition-colors disabled:opacity-60"
      >
        {state.kind === "submitting" ? "Sending…" : "Get free API key"}
      </button>
      {state.kind === "error" && (
        <p className="text-[13px] text-red-400 sm:w-full">{state.message}</p>
      )}
    </form>
  );
}
