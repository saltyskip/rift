"use client";

import { cn } from "@/lib/utils";

interface CodeBlockProps {
  code: string;
  language?: string;
  className?: string;
  filename?: string;
}

export function CodeBlock({ code, className, filename }: CodeBlockProps) {
  const highlighted = code
    .replace(
      /"([^"]+)":/g,
      '<span class="code-key">"$1"</span>:'
    )
    .replace(
      /: "([^"]+)"/g,
      ': <span class="code-string">"$1"</span>'
    )
    .replace(
      /: (true|false|null)/g,
      ': <span class="code-keyword">$1</span>'
    )
    .replace(
      /: (\d+\.?\d*)/g,
      ': <span class="code-number">$1</span>'
    );

  return (
    <div
      className={cn(
        "relative rounded-xl border border-border overflow-hidden bg-foreground/[0.03]",
        className
      )}
    >
      {/* Title bar */}
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-border bg-foreground/[0.02]">
        <div className="flex gap-1.5">
          <div className="size-2 rounded-full bg-foreground/10" />
          <div className="size-2 rounded-full bg-foreground/10" />
          <div className="size-2 rounded-full bg-foreground/10" />
        </div>
        {filename && (
          <span className="text-[11px] text-muted-foreground/60 ml-2 font-mono">
            {filename}
          </span>
        )}
      </div>
      {/* Code */}
      <div className="p-5 overflow-x-auto">
        <pre className="text-[12.5px] leading-[1.8]">
          <code
            className="font-mono text-foreground/70"
            dangerouslySetInnerHTML={{ __html: highlighted }}
          />
        </pre>
      </div>
      <style jsx>{`
        :global(.code-key) {
          color: oklch(0.40 0.14 160);
        }
        :global(.code-string) {
          color: oklch(0.45 0.10 55);
        }
        :global(.code-keyword) {
          color: oklch(0.45 0.14 270);
        }
        :global(.code-number) {
          color: oklch(0.50 0.14 30);
        }
      `}</style>
    </div>
  );
}
