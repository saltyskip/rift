"use client";

// Server-driven OAuth signin buttons — these are plain anchors that hit
// `/v1/auth/oauth/{provider}/start`. The server redirects to the provider,
// the provider redirects back to `/v1/auth/oauth/{provider}/callback`, which
// mints a session cookie and 303s to `/account`. No JS state, no fetch,
// browser handles the navigation natively.

const API_URL = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

function startUrl(provider: "github" | "google", next?: string): string {
  const u = new URL(`${API_URL}/v1/auth/oauth/${provider}/start`);
  if (next) u.searchParams.set("next", next);
  return u.toString();
}

export function OauthButtons({ next }: { next?: string }) {
  return (
    <div className="space-y-2.5">
      <a
        href={startUrl("github", next)}
        className="flex items-center justify-center gap-2.5 w-full h-11 rounded-lg border border-[#222225] bg-[#0a0a0b] text-[14px] font-medium text-[#fafafa] hover:border-[#2dd4bf]/30 transition-colors"
      >
        <GithubLogo />
        Continue with GitHub
      </a>
      <a
        href={startUrl("google", next)}
        className="flex items-center justify-center gap-2.5 w-full h-11 rounded-lg border border-[#222225] bg-[#0a0a0b] text-[14px] font-medium text-[#fafafa] hover:border-[#2dd4bf]/30 transition-colors"
      >
        <GoogleLogo />
        Continue with Google
      </a>

      <div className="flex items-center gap-3 py-3">
        <div className="flex-1 h-px bg-[#222225]" />
        <span className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest">
          or
        </span>
        <div className="flex-1 h-px bg-[#222225]" />
      </div>
    </div>
  );
}

function GithubLogo() {
  return (
    <svg
      aria-hidden
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="currentColor"
    >
      <path d="M12 .5C5.6.5.5 5.7.5 12.1c0 5.1 3.3 9.4 7.9 10.9.6.1.8-.2.8-.6v-2c-3.2.7-3.9-1.5-3.9-1.5-.5-1.4-1.3-1.7-1.3-1.7-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.7 1.3 3.4 1 .1-.8.4-1.3.8-1.6-2.6-.3-5.3-1.3-5.3-5.9 0-1.3.5-2.4 1.2-3.2-.1-.3-.5-1.5.1-3.1 0 0 1-.3 3.3 1.2.9-.3 2-.4 3-.4s2.1.1 3 .4c2.3-1.6 3.3-1.2 3.3-1.2.7 1.6.2 2.8.1 3.1.8.8 1.2 1.9 1.2 3.2 0 4.6-2.8 5.6-5.4 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6 4.6-1.5 7.9-5.8 7.9-10.9C23.5 5.7 18.4.5 12 .5z" />
    </svg>
  );
}

function GoogleLogo() {
  return (
    <svg aria-hidden width="18" height="18" viewBox="0 0 18 18">
      <path
        fill="#4285F4"
        d="M17.64 9.2c0-.637-.057-1.251-.164-1.84H9v3.481h4.844a4.14 4.14 0 0 1-1.796 2.717v2.258h2.908c1.702-1.567 2.684-3.874 2.684-6.615z"
      />
      <path
        fill="#34A853"
        d="M9 18c2.43 0 4.467-.806 5.956-2.184l-2.908-2.258c-.806.54-1.837.86-3.048.86-2.344 0-4.328-1.584-5.036-3.711H.957v2.332A8.997 8.997 0 0 0 9 18z"
      />
      <path
        fill="#FBBC05"
        d="M3.964 10.707A5.41 5.41 0 0 1 3.682 9c0-.593.102-1.17.282-1.707V4.961H.957A8.996 8.996 0 0 0 0 9c0 1.452.348 2.827.957 4.039l3.007-2.332z"
      />
      <path
        fill="#EA4335"
        d="M9 3.58c1.321 0 2.508.454 3.44 1.345l2.582-2.58C13.463.891 11.426 0 9 0A8.997 8.997 0 0 0 .957 4.961L3.964 7.293C4.672 5.166 6.656 3.58 9 3.58z"
      />
    </svg>
  );
}
