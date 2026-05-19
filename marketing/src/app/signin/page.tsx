import type { Metadata } from "next";
import Link from "next/link";
import { SignInForm } from "@/components/sign-in-form";

export const metadata: Metadata = {
  title: "Sign in to Rift",
  description:
    "Sign in to your Rift account with email. No passwords — we'll send you a one-time link. New accounts are created automatically on first sign-in.",
};

export default function SignInPage() {
  return (
    <main className="pt-24 pb-20 px-6 min-h-[70vh]">
      <div className="mx-auto max-w-md">
        <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
          Sign in
        </p>
        <h1 className="text-3xl font-semibold tracking-[-0.03em] mb-4">
          Sign in to Rift.
        </h1>
        <p className="text-[14px] text-[#71717a] mb-8 leading-relaxed">
          No password. We&rsquo;ll email you a link. New to Rift?
          Same form — your account is created on first sign-in.
        </p>

        <div className="rounded-xl border border-[#222225] bg-[#111113] p-6">
          <SignInForm />
        </div>

        <p className="mt-8 text-[13px] text-[#52525b] leading-relaxed">
          Prefer the CLI? Run{" "}
          <code className="text-[#a1a1aa]">rift init</code> after{" "}
          <Link
            href="/signup"
            className="text-[#2dd4bf] hover:text-[#5eead4] transition-colors"
          >
            installing
          </Link>
          .
        </p>
      </div>
    </main>
  );
}
