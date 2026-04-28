import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Riftl.ink MCP Server — Rift Docs",
  description:
    "Connect Claude, ChatGPT, and other MCP hosts to Riftl.ink over Streamable HTTP.",
  alternates: { canonical: "/docs/mcp" },
};

export default function McpPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">
          Agents
        </p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">
          Riftl.ink MCP Server
        </h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Rift exposes a Model Context Protocol server at{" "}
          <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
            https://api.riftl.ink/mcp
          </code>{" "}
          using the Streamable HTTP transport. AI agents can use it to create, list, inspect,
          update, and delete deep links without calling the REST API directly.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">When to use MCP</h2>
          <ul className="list-disc pl-5 space-y-2 text-[15px] text-[#a1a1aa]">
            <li>Use MCP when the host already supports tool calling over the Model Context Protocol.</li>
            <li>Use MCP when an agent needs to create or manage links interactively during a conversation.</li>
            <li>Use the REST API when you want direct HTTP integration, generated clients, or raw OpenAPI tooling.</li>
          </ul>
          <Callout type="info">
            The MCP server and REST API operate on the same Rift data model. MCP is a transport
            for agent tools, not a separate product surface.
          </Callout>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Connection details</h2>

          <Step n={1} title="Use the manifest or server metadata">
            <p>
              Machine-readable server metadata is available at{" "}
              <a href="/.well-known/mcp.json" className="text-[#2dd4bf] hover:underline">
                /.well-known/mcp.json
              </a>{" "}
              and{" "}
              <a href="/mcp/server.json" className="text-[#2dd4bf] hover:underline">
                /mcp/server.json
              </a>
              .
            </p>
          </Step>

          <Step n={2} title="Configure the transport">
            <CodeBlock lang="json">{`{
  "mcpServers": {
    "rift": {
      "url": "https://api.riftl.ink/mcp",
      "headers": {
        "x-api-key": "rl_live_YOUR_KEY"
      }
    }
  }
}`}</CodeBlock>
          </Step>

          <Step n={3} title="Authenticate with a Rift secret key">
            <p>
              The MCP endpoint uses the same server-side secret key as the REST API. Use an{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
                rl_live_
              </code>{" "}
              key and send it in the <code>x-api-key</code> header.
            </p>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Available capabilities</h2>
          <ul className="list-disc pl-5 space-y-2 text-[15px] text-[#a1a1aa]">
            <li>Create deep links with per-platform destinations and optional agent context.</li>
            <li>List existing links for a tenant.</li>
            <li>Get a single link by ID.</li>
            <li>Update link fields without switching to raw HTTP.</li>
            <li>Delete links that are no longer needed.</li>
          </ul>
        </section>

        <div className="gradient-line" />

        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Related resources</h2>
          <ul className="list-disc pl-5 space-y-2 text-[15px] text-[#a1a1aa]">
            <li>
              <a href="/api-reference" className="text-[#2dd4bf] hover:underline">
                Riftl.ink API Reference
              </a>
            </li>
            <li>
              <a href="/openapi.json" className="text-[#2dd4bf] hover:underline">
                OpenAPI JSON
              </a>
            </li>
            <li>
              <a href="/llms.txt" className="text-[#2dd4bf] hover:underline">
                llms.txt
              </a>
            </li>
          </ul>
        </section>
      </div>
    </div>
  );
}
