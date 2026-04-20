import type { MDXComponents } from "mdx/types";
import { DocsCodeBlock } from "@/components/docs-code-block";
import { DocsCallout } from "@/components/docs-callout";
import { FAQ } from "./faq";

function extractCodeString(children: React.ReactNode): string {
  if (typeof children === "string") return children;
  if (Array.isArray(children)) return children.map(extractCodeString).join("");
  if (
    children &&
    typeof children === "object" &&
    "props" in children &&
    (children as { props: { children: React.ReactNode } }).props
  ) {
    return extractCodeString(
      (children as { props: { children: React.ReactNode } }).props.children,
    );
  }
  return "";
}

export const blogMdxComponents: MDXComponents = {
  h1: (props) => (
    <h1
      className="mt-14 mb-5 text-4xl font-bold tracking-tight text-[#fafafa]"
      {...props}
    />
  ),
  h2: (props) => (
    <h2
      className="mt-12 mb-4 text-2xl font-bold text-[#fafafa]"
      {...props}
    />
  ),
  h3: (props) => (
    <h3
      className="mt-8 mb-3 text-lg font-semibold text-[#fafafa]"
      {...props}
    />
  ),
  p: (props) => (
    <p
      className="my-5 text-[15px] leading-relaxed text-[#a1a1aa]"
      {...props}
    />
  ),
  a: (props) => (
    <a className="text-[#2dd4bf] underline-offset-2 hover:underline" {...props} />
  ),
  ul: (props) => (
    <ul
      className="my-5 list-disc space-y-2 pl-6 text-[15px] leading-relaxed text-[#a1a1aa] marker:text-[#52525b]"
      {...props}
    />
  ),
  ol: (props) => (
    <ol
      className="my-5 list-decimal space-y-2 pl-6 text-[15px] leading-relaxed text-[#a1a1aa] marker:text-[#52525b]"
      {...props}
    />
  ),
  li: (props) => <li className="pl-1" {...props} />,
  blockquote: (props) => (
    <blockquote
      className="my-6 border-l-2 border-[#2dd4bf]/40 pl-5 text-[15px] italic leading-relaxed text-[#d4d4d8]"
      {...props}
    />
  ),
  strong: (props) => (
    <strong className="font-semibold text-[#fafafa]" {...props} />
  ),
  hr: () => <div className="gradient-line my-10" />,
  table: (props) => (
    <div className="my-8 overflow-x-auto rounded-xl border border-[#1e1e22]">
      <table className="w-full text-left text-[14px]" {...props} />
    </div>
  ),
  th: (props) => (
    <th
      className="border-b border-[#1e1e22] bg-[#0c0c0e] px-4 py-3 font-semibold text-[#fafafa]"
      {...props}
    />
  ),
  td: (props) => (
    <td
      className="border-b border-[#1e1e22] px-4 py-3 text-[#a1a1aa] last:border-b-0"
      {...props}
    />
  ),
  code: (props) => (
    <code
      className="rounded bg-[#2dd4bf]/10 px-1.5 py-0.5 font-mono text-[13px] text-[#2dd4bf]"
      {...props}
    />
  ),
  pre: ({ children }) => {
    const childProps =
      children &&
      typeof children === "object" &&
      "props" in children
        ? (children as { props: { className?: string; children: React.ReactNode } })
            .props
        : undefined;
    const langMatch = childProps?.className?.match(/language-(\w+)/);
    const code = extractCodeString(childProps?.children);
    return <DocsCodeBlock lang={langMatch?.[1] ?? "text"}>{code}</DocsCodeBlock>;
  },
  Callout: DocsCallout,
  FAQ,
};
