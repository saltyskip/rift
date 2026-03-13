"use client";

import { useState, useEffect } from "react";
import { motion } from "motion/react";

type Line = { type: string; text?: string; k?: string; v?: string };

const HUMAN_FLOW: Line[] = [
  { type: "comment", text: "# A human clicks the link" },
  { type: "cmd", text: "GET /r/summer-launch" },
  { type: "blank" },
  { type: "status", text: "302 Found" },
  { type: "header", text: "Location: myapp://promo/summer" },
  { type: "blank" },
  { type: "comment", text: "# → Redirected to the app" },
];

const AGENT_FLOW: Line[] = [
  { type: "comment", text: "# An agent resolves the same link" },
  { type: "cmd", text: "GET /r/summer-launch" },
  { type: "header", text: "Accept: application/json" },
  { type: "blank" },
  { type: "status", text: "200 OK" },
  { type: "json-open", text: "{" },
  { type: "json-key", k: "link_id", v: '"summer-launch"' },
  { type: "json-key", k: "destination", v: '"myapp://promo/summer"' },
  { type: "json-nested-open", k: "metadata", text: "{" },
  { type: "json-nested-key", k: "campaign", v: '"summer-2026"' },
  { type: "json-nested-key", k: "source", v: '"agent-outreach"' },
  { type: "json-nested-close" },
  { type: "json-close", text: "}" },
];

function TerminalLine({ line, visible }: { line: Line; visible: boolean }) {
  if (!visible) return null;

  if (line.type === "blank") return <div className="h-3" />;
  if (line.type === "comment") return <div className="syn-comment text-[13px] font-mono">{line.text}</div>;
  if (line.type === "cmd") return <div className="font-mono text-[13px]"><span className="syn-method">{line.text?.split(" ")[0]}</span> <span className="text-[#fafafa]">{line.text?.split(" ").slice(1).join(" ")}</span></div>;
  if (line.type === "header") return <div className="font-mono text-[13px] syn-url">{line.text}</div>;
  if (line.type === "status") return <div className="font-mono text-[13px] syn-kw">{line.text}</div>;
  if (line.type === "json-open" || line.type === "json-close") return <div className="font-mono text-[13px] text-[#a1a1aa]">{line.text}</div>;
  if (line.type === "json-key") return (
    <div className="font-mono text-[13px] pl-4">
      <span className="syn-key">&quot;{line.k}&quot;</span>
      <span className="text-[#52525b]">: </span>
      <span className="syn-str">{line.v}</span>
    </div>
  );
  if (line.type === "json-nested-open") return (
    <div className="font-mono text-[13px] pl-4">
      <span className="syn-key">&quot;{line.k}&quot;</span>
      <span className="text-[#52525b]">: </span>
      <span className="text-[#a1a1aa]">{"{"}</span>
    </div>
  );
  if (line.type === "json-nested-key") return (
    <div className="font-mono text-[13px] pl-8">
      <span className="syn-key">&quot;{line.k}&quot;</span>
      <span className="text-[#52525b]">: </span>
      <span className="syn-str">{line.v}</span>
    </div>
  );
  if (line.type === "json-nested-close") return <div className="font-mono text-[13px] pl-4 text-[#a1a1aa]">{"}"}</div>;

  return null;
}

function Terminal({ title, lines, delay = 0 }: { title: string; lines: Line[]; delay?: number }) {
  const [visibleCount, setVisibleCount] = useState(0);

  useEffect(() => {
    const timeout = setTimeout(() => {
      const interval = setInterval(() => {
        setVisibleCount((c) => {
          if (c >= lines.length) {
            clearInterval(interval);
            return c;
          }
          return c + 1;
        });
      }, 120);
      return () => clearInterval(interval);
    }, delay);
    return () => clearTimeout(timeout);
  }, [lines.length, delay]);

  return (
    <div className="window flex-1">
      <div className="window-bar">
        <div className="window-dot" />
        <div className="window-dot" />
        <div className="window-dot" />
        <span className="ml-2 text-[11px] text-[#52525b] font-mono">{title}</span>
      </div>
      <div className="p-5 min-h-[260px] space-y-1">
        {lines.map((line, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0, y: 4 }}
            animate={i < visibleCount ? { opacity: 1, y: 0 } : {}}
            transition={{ duration: 0.15 }}
          >
            <TerminalLine line={line} visible={i < visibleCount} />
          </motion.div>
        ))}
        {visibleCount < lines.length && (
          <span className="inline-block w-2 h-4 bg-[#2dd4bf] cursor-blink ml-0.5 -mb-0.5" />
        )}
      </div>
    </div>
  );
}

export function TerminalDemo() {
  return (
    <div className="flex flex-col lg:flex-row gap-4">
      <Terminal title="human — browser" lines={HUMAN_FLOW} delay={500} />
      <Terminal title="agent — api client" lines={AGENT_FLOW} delay={2000} />
    </div>
  );
}
