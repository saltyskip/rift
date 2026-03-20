"use client";

import { useEffect, useRef, useState } from "react";
import { codeToHtml } from "shiki";

interface DocsCodeBlockProps {
  children: string;
  lang?: string;
}

export function DocsCodeBlock({ children, lang = "bash" }: DocsCodeBlockProps) {
  const [html, setHtml] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    let cancelled = false;
    codeToHtml(children.trim(), {
      lang,
      theme: "vesper",
    }).then((result) => {
      if (!cancelled) setHtml(result);
    });
    return () => { cancelled = true; };
  }, [children, lang]);

  function handleCopy() {
    navigator.clipboard.writeText(children.trim());
    setCopied(true);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div className="relative group rounded-lg border border-[#1e1e22] overflow-hidden bg-[#0c0c0e]">
      {/* Copy button */}
      <button
        onClick={handleCopy}
        className="absolute top-2.5 right-2.5 z-10 flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] font-medium bg-[#1e1e22] text-[#71717a] hover:text-[#fafafa] opacity-0 group-hover:opacity-100 transition-opacity"
        aria-label="Copy code"
      >
        {copied ? (
          <>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="20 6 9 17 4 12" />
            </svg>
            Copied
          </>
        ) : (
          <>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect width="14" height="14" x="8" y="8" rx="2" />
              <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
            </svg>
            Copy
          </>
        )}
      </button>

      {/* Code */}
      <div className="p-4 overflow-x-auto text-[13px] leading-relaxed [&_pre]:!bg-transparent [&_code]:!bg-transparent [&_pre]:!m-0 [&_pre]:!p-0">
        {html ? (
          <div dangerouslySetInnerHTML={{ __html: html }} />
        ) : (
          <pre className="font-mono text-[#a1a1aa]">
            <code>{children.trim()}</code>
          </pre>
        )}
      </div>
    </div>
  );
}
