"use client";

import { usePathname } from "next/navigation";
import { useState } from "react";

const NAV = [
  {
    group: "Getting Started",
    items: [
      { label: "Quick Start", href: "/docs" },
      { label: "Register Your App", href: "/docs/apps" },
      { label: "Custom Domains", href: "/docs/domains" },
    ],
  },
  {
    group: "Deep Linking",
    items: [
      { label: "Create Links", href: "/docs/links" },
      { label: "Universal Links", href: "/docs/universal-links" },
      { label: "Deferred Deep Linking", href: "/docs/deferred" },
    ],
  },
  {
    group: "SDKs",
    items: [
      { label: "Web (rift.js)", href: "/docs/web-sdk" },
      { label: "iOS (Swift)", href: "/docs/ios-sdk" },
      { label: "Android (Kotlin)", href: "/docs/android-sdk" },
    ],
  },
  {
    group: "Tracking",
    items: [
      { label: "Attribution", href: "/docs/attribution" },
      { label: "Webhooks", href: "/docs/webhooks" },
    ],
  },
];

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [open, setOpen] = useState(false);

  return (
    <div className="min-h-screen pt-14">
      {/* Mobile toggle */}
      <button
        onClick={() => setOpen(!open)}
        className="fixed top-16 left-4 z-40 md:hidden p-2 rounded-lg bg-[#111113] border border-[#1e1e22] text-[#71717a]"
        aria-label="Toggle docs navigation"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          {open ? (
            <path d="M6 18L18 6M6 6l12 12" />
          ) : (
            <path d="M3 12h18M3 6h18M3 18h18" />
          )}
        </svg>
      </button>

      <div className="mx-auto max-w-6xl flex">
        {/* Sidebar */}
        <aside
          className={`${
            open ? "translate-x-0" : "-translate-x-full"
          } md:translate-x-0 fixed md:sticky top-14 left-0 z-30 h-[calc(100vh-56px)] w-60 shrink-0 overflow-y-auto border-r border-[#1e1e22] bg-[#09090b] px-4 py-8 transition-transform md:transition-none`}
        >
          <nav className="space-y-6">
            {NAV.map((section) => (
              <div key={section.group}>
                <p className="text-[11px] font-medium text-[#52525b] uppercase tracking-widest mb-2">
                  {section.group}
                </p>
                <ul className="space-y-0.5">
                  {section.items.map((item) => {
                    const active = pathname === item.href;
                    return (
                      <li key={item.href}>
                        <a
                          href={item.href}
                          onClick={() => setOpen(false)}
                          className={`block rounded-md px-3 py-1.5 text-[13px] transition-colors ${
                            active
                              ? "bg-[#2dd4bf]/10 text-[#2dd4bf] font-medium"
                              : "text-[#71717a] hover:text-[#fafafa] hover:bg-[#111113]"
                          }`}
                        >
                          {item.label}
                        </a>
                      </li>
                    );
                  })}
                </ul>
              </div>
            ))}
          </nav>
        </aside>

        {/* Overlay on mobile */}
        {open && (
          <div
            className="fixed inset-0 z-20 bg-black/50 md:hidden"
            onClick={() => setOpen(false)}
          />
        )}

        {/* Content */}
        <div className="flex-1 min-w-0 px-6 md:px-12 py-10">
          {children}
        </div>
      </div>
    </div>
  );
}
