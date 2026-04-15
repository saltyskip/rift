"use client";

import { useEffect, useRef } from "react";

const API_SPEC_URL =
  process.env.NEXT_PUBLIC_API_URL
    ? `${process.env.NEXT_PUBLIC_API_URL}/openapi.json`
    : "https://api.riftl.ink/openapi.json";

export function ScalarDocs() {
  const containerRef = useRef<HTMLDivElement>(null);
  const initialized = useRef(false);

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    const script = document.createElement("script");
    script.src = "https://cdn.jsdelivr.net/npm/@scalar/api-reference";
    script.onload = () => {
      const scalar = (window as unknown as { Scalar?: { createApiReference: (el: HTMLDivElement, cfg: Record<string, unknown>) => void } }).Scalar;
      if (scalar && containerRef.current) {
        scalar.createApiReference(containerRef.current, {
          url: API_SPEC_URL,
          theme: "none",
          darkMode: true,
          hideDarkModeToggle: true,
          customCss: `
            :root {
              --scalar-custom-header-height: 0px;
            }
            .scalar-app,
            .dark-mode .scalar-app {
              --scalar-background-1: #09090b;
              --scalar-background-2: #111113;
              --scalar-background-3: #18181b;
              --scalar-color-1: #fafafa;
              --scalar-color-2: #a1a1aa;
              --scalar-color-3: #71717a;
              --scalar-color-accent: #2dd4bf;
              --scalar-color-green: #34d399;
              --scalar-color-blue: #60a5fa;
              --scalar-color-orange: #f59e0b;
              --scalar-color-red: #ef4444;
              --scalar-border-color: #222225;
              --scalar-button-1: #2dd4bf;
              --scalar-button-1-hover: #5eead4;
              --scalar-button-1-color: #042f2e;
              --scalar-radius: 8px;
              --scalar-radius-lg: 10px;
              --scalar-font: 'Inter', system-ui, sans-serif;
              --scalar-font-code: 'JetBrains Mono', ui-monospace, monospace;
              --scalar-sidebar-background-1: #0c0c0e;
              --scalar-sidebar-border-color: #1e1e22;
              --scalar-sidebar-color-1: #fafafa;
              --scalar-sidebar-color-2: #71717a;
              --scalar-sidebar-color-active: #2dd4bf;
              --scalar-sidebar-search-background: #111113;
              --scalar-sidebar-search-border-color: #222225;
              --scalar-sidebar-search-color: #a1a1aa;
            }
          `,
        });
      }
    };
    document.body.appendChild(script);

    return () => {
      script.remove();
    };
  }, []);

  return (
    <>
      <style>{`
        nav, footer { display: none !important; }
        main { padding: 0 !important; margin: 0 !important; }
      `}</style>
      <div ref={containerRef} style={{ minHeight: "100vh" }} />
    </>
  );
}
