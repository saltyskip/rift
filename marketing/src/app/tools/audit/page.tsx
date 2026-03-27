"use client";

import { useState, useEffect, useCallback } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
} from "@/components/ui/card";

const API_BASE = "https://api.riftl.ink";
const SAMPLE_LINK = "landing-home-apple";

interface LinkData {
  link_id: string;
  ios_deep_link: string | null;
  android_deep_link: string | null;
  web_url: string | null;
  ios_store_url: string | null;
  android_store_url: string | null;
  metadata: {
    title?: string;
    description?: string;
    image?: string;
  } | null;
  agent_context: {
    action?: string;
    cta?: string;
    description?: string;
  } | null;
  _rift_meta: {
    context: string;
    source: string;
    status: string;
    tenant_domain: string | null;
    tenant_verified: boolean;
  };
}

interface CheckItem {
  field: string;
  label: string;
  configured: boolean;
  value: string | null;
  hint: string;
  patchField: string;
  placeholder: string;
}

// ── Tiers ──

interface Tier {
  name: string;
  description: string;
  items: CheckItem[];
}

function computeTiers(data: LinkData): Tier[] {
  return [
    {
      name: "Routing",
      description: "Where users land on each platform",
      items: [
        {
          field: "web_url",
          label: "Web URL",
          configured: !!data.web_url,
          value: data.web_url,
          hint: "Desktop users won't have a destination",
          patchField: "web_url",
          placeholder: "https://yourcompany.com",
        },
        {
          field: "ios_store_url",
          label: "iOS App Store",
          configured: !!data.ios_store_url,
          value: data.ios_store_url,
          hint: "iOS users without the app can't download it",
          patchField: "ios_store_url",
          placeholder: "https://apps.apple.com/app/id123456789",
        },
        {
          field: "android_store_url",
          label: "Android Play Store",
          configured: !!data.android_store_url,
          value: data.android_store_url,
          hint: "Android users without the app can't download it",
          patchField: "android_store_url",
          placeholder:
            "https://play.google.com/store/apps/details?id=com.example",
        },
      ],
    },
    {
      name: "Deep Linking",
      description: "Which screen opens in the app",
      items: [
        {
          field: "ios_deep_link",
          label: "iOS Deep Link",
          configured: !!data.ios_deep_link,
          value: data.ios_deep_link,
          hint: "App opens to the home screen instead of specific content",
          patchField: "ios_deep_link",
          placeholder: "myapp://product/123",
        },
        {
          field: "android_deep_link",
          label: "Android Deep Link",
          configured: !!data.android_deep_link,
          value: data.android_deep_link,
          hint: "App opens to the home screen instead of specific content",
          patchField: "android_deep_link",
          placeholder: "myapp://product/123",
        },
      ],
    },
    {
      name: "Presentation",
      description: "How the link looks when shared",
      items: [
        {
          field: "metadata.title",
          label: "Social Title",
          configured: !!data.metadata?.title,
          value: data.metadata?.title || null,
          hint: "Social previews show the raw URL instead of a title",
          patchField: "metadata",
          placeholder: '{"title": "Your Title", "description": "..."}',
        },
        {
          field: "metadata.description",
          label: "Social Description",
          configured: !!data.metadata?.description,
          value: data.metadata?.description || null,
          hint: "Social previews won't have a description",
          patchField: "metadata",
          placeholder: '{"title": "...", "description": "Your description"}',
        },
        {
          field: "metadata.image",
          label: "Social Image",
          configured: !!data.metadata?.image,
          value: data.metadata?.image || null,
          hint: "Social previews won't have an image",
          patchField: "metadata",
          placeholder: '{"image": "https://example.com/og.jpg"}',
        },
        {
          field: "agent_context",
          label: "AI Agent Context",
          configured: !!(
            data.agent_context?.action ||
            data.agent_context?.cta ||
            data.agent_context?.description
          ),
          value: data.agent_context?.action || null,
          hint: "AI agents won't understand what this link does",
          patchField: "agent_context",
          placeholder:
            '{"action": "download", "cta": "Get the App", "description": "..."}',
        },
      ],
    },
  ];
}

function qualitativeLabel(
  configured: number,
  total: number
): { label: string; color: string } {
  const pct = total > 0 ? configured / total : 0;
  if (pct >= 1) return { label: "Ready to ship", color: "text-green-400" };
  if (pct >= 0.7) return { label: "Almost there", color: "text-yellow-400" };
  if (pct >= 0.4) return { label: "Needs work", color: "text-orange-400" };
  return { label: "Getting started", color: "text-muted-foreground" };
}

function worstProblem(tiers: Tier[]): string | null {
  for (const tier of tiers) {
    const missing = tier.items.find((i) => !i.configured);
    if (missing) return missing.hint;
  }
  return null;
}

function extractLinkId(input: string): string {
  const trimmed = input.trim();
  try {
    const url = new URL(trimmed);
    const segments = url.pathname.split("/").filter(Boolean);
    return segments[segments.length - 1] || trimmed;
  } catch {
    return trimmed;
  }
}

type SurfaceTab = "ios" | "android" | "desktop" | "social" | "agent";

function getTabItems(tab: SurfaceTab, tiers: Tier[]): CheckItem[] {
  const allItems = tiers.flatMap((t) => t.items);
  switch (tab) {
    case "ios":
      return allItems.filter((i) =>
        ["ios_store_url", "ios_deep_link"].includes(i.field)
      );
    case "android":
      return allItems.filter((i) =>
        ["android_store_url", "android_deep_link"].includes(i.field)
      );
    case "desktop":
      return allItems.filter((i) => i.field === "web_url");
    case "social":
      return allItems.filter((i) =>
        ["metadata.title", "metadata.description", "metadata.image"].includes(
          i.field
        )
      );
    case "agent":
      return allItems.filter((i) => i.field === "agent_context");
  }
}

function getTabMeta(tab: SurfaceTab) {
  switch (tab) {
    case "ios":
      return {
        title: "iOS",
        description: "App Store link and deep link for iOS users",
      };
    case "android":
      return {
        title: "Android",
        description: "Play Store link and deep link for Android users",
      };
    case "desktop":
      return {
        title: "Desktop",
        description: "Fallback URL for desktop and non-mobile users",
      };
    case "social":
      return {
        title: "Social Preview",
        description: "How the link unfurls in Slack, Twitter, iMessage, etc.",
      };
    case "agent":
      return {
        title: "AI Agent",
        description: "Structured context for AI agents and LLMs",
      };
  }
}

// ── Preview components ──

function LandingPageIframe({ linkId }: { linkId: string }) {
  const iframeUrl = `${API_BASE}/r/${encodeURIComponent(linkId)}`;
  // Render iframe at 390px (iPhone viewport) and scale to fit the device frame.
  const iframeWidth = 390;
  const iframeHeight = 844;
  const frameWidth = 270;
  const scale = frameWidth / iframeWidth;
  const displayHeight = iframeHeight * scale;

  return (
    <div className="flex flex-col items-center">
      <div
        className="rounded-[36px] border-[3px] border-foreground/20 bg-black p-2 shadow-2xl"
        style={{ width: frameWidth + 16 }}
      >
        <div className="mx-auto w-24 h-5 bg-black rounded-b-2xl relative z-10" />
        <div
          className="rounded-[28px] -mt-2.5 overflow-hidden"
          style={{ width: frameWidth, height: displayHeight }}
        >
          <iframe
            src={iframeUrl}
            title="Landing page preview"
            className="border-0 bg-black"
            style={{
              width: iframeWidth,
              height: iframeHeight,
              transform: `scale(${scale})`,
              transformOrigin: "top left",
            }}
          />
        </div>
        <div className="mx-auto w-20 h-1 bg-foreground/20 rounded-full mt-2" />
      </div>
      <a
        href={iframeUrl}
        target="_blank"
        rel="noopener noreferrer"
        className="text-[10px] text-muted-foreground hover:text-primary font-mono mt-3"
      >
        Open in new tab
      </a>
    </div>
  );
}

function DesktopIframe({
  linkId,
  domain,
}: {
  linkId: string;
  domain: string | null;
}) {
  const iframeUrl = `${API_BASE}/r/${encodeURIComponent(linkId)}`;
  // Render iframe at 1280px wide and scale down to fit the container.
  const iframeWidth = 1280;
  const displayWidth = 720;
  const scale = displayWidth / iframeWidth;
  const iframeHeight = 800;
  const displayHeight = iframeHeight * scale;

  return (
    <div className="w-full" style={{ maxWidth: displayWidth }}>
      <div className="rounded-lg border border-foreground/20 bg-[#1a1a1a] shadow-2xl overflow-hidden">
        {/* Browser chrome */}
        <div className="flex items-center gap-2 px-3 py-2 bg-[#2a2a2a] border-b border-foreground/10">
          <div className="flex gap-1.5">
            <div className="w-2.5 h-2.5 rounded-full bg-[#ff5f57]" />
            <div className="w-2.5 h-2.5 rounded-full bg-[#febc2e]" />
            <div className="w-2.5 h-2.5 rounded-full bg-[#28c840]" />
          </div>
          <div className="flex-1 bg-[#1a1a1a] rounded px-3 py-1 text-[10px] text-white/40 font-mono truncate ml-2">
            {domain || "riftl.ink"}/{linkId}
          </div>
        </div>
        <div
          style={{
            width: displayWidth,
            height: displayHeight,
            overflow: "hidden",
          }}
        >
          <iframe
            src={iframeUrl}
            title="Landing page (desktop)"
            className="border-0 bg-black"
            style={{
              width: iframeWidth,
              height: iframeHeight,
              transform: `scale(${scale})`,
              transformOrigin: "top left",
            }}
          />
        </div>
      </div>
      <div className="text-center mt-2">
        <a
          href={iframeUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="text-[10px] text-muted-foreground hover:text-primary font-mono"
        >
          Open in new tab
        </a>
      </div>
    </div>
  );
}

function SlackMockup({ data }: { data: LinkData }) {
  return (
    <div className="w-full max-w-[480px]">
      <div className="bg-[#1a1d21] rounded-lg overflow-hidden shadow-2xl">
        {/* Channel header */}
        <div className="border-b border-white/5 px-4 py-2.5 flex items-center gap-2">
          <span className="text-white/40 text-sm font-bold">#</span>
          <span className="text-[13px] font-bold text-white/90">general</span>
          <span className="text-[11px] text-white/30 ml-1">
            Your team&apos;s main channel
          </span>
        </div>

        {/* Messages area */}
        <div className="px-4 py-4 space-y-4">
          <div className="flex gap-2.5">
            <div className="w-9 h-9 rounded-lg bg-emerald-700 flex-shrink-0 flex items-center justify-center">
              <span className="text-[13px] text-white font-bold">A</span>
            </div>
            <div className="flex-1">
              <div className="flex items-baseline gap-2">
                <span className="text-[13px] font-bold text-white">
                  Alice Chen
                </span>
                <span className="text-[10px] text-white/30">11:42 AM</span>
              </div>
              <p className="text-[13px] text-[#d1d2d3] mt-0.5">
                Check out this link{" "}
                <span className="text-[#1d9bd1]">
                  {data._rift_meta.tenant_domain || "riftl.ink"}/{data.link_id}
                </span>
              </p>

              {/* Unfurl card */}
              <div className="mt-2 border-l-[3px] border-[#4a9cc5] pl-3 py-1 max-w-md">
                <p className="text-[11px] text-[#8b8d90] mb-0.5 flex items-center gap-1">
                  {data._rift_meta.tenant_domain || "riftl.ink"}
                  {data._rift_meta.tenant_verified && (
                    <span className="text-primary text-[9px]">&#10003;</span>
                  )}
                </p>
                <p className="text-[13px] font-bold text-[#1d9bd1] mb-0.5">
                  {data.metadata?.title || (
                    <span className="text-[#8b8d90] italic font-normal">
                      No title configured
                    </span>
                  )}
                </p>
                <p className="text-[12px] text-[#d1d2d3] line-clamp-2 mb-2 leading-relaxed">
                  {data.metadata?.description || (
                    <span className="text-[#8b8d90] italic">
                      No description configured
                    </span>
                  )}
                </p>
                {data.metadata?.image ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={data.metadata.image}
                    alt=""
                    className="w-full max-h-[200px] object-cover rounded"
                  />
                ) : (
                  <div className="w-full h-24 bg-[#2c2d30] rounded flex items-center justify-center">
                    <span className="text-[11px] text-[#8b8d90]">
                      No image configured
                    </span>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Reaction */}
          <div className="ml-[46px] -mt-2">
            <div className="inline-flex items-center gap-1 bg-white/5 rounded-full px-2 py-0.5 border border-white/10">
              <span className="text-[12px]">&#128064;</span>
              <span className="text-[11px] text-[#1d9bd1]">2</span>
            </div>
          </div>
        </div>

        {/* Input bar */}
        <div className="px-4 pb-4">
          <div className="bg-[#22252a] rounded-lg border border-white/10 px-3 py-2.5 flex items-center">
            <span className="text-[12px] text-white/25">
              Message #general
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

function AIAgentMockup({ data }: { data: LinkData }) {
  return (
    <div className="w-full max-w-[480px]">
      <div className="bg-[#111113] rounded-lg border border-border overflow-hidden shadow-2xl flex flex-col min-h-[400px]">
        {/* Header */}
        <div className="border-b border-border px-4 py-3 flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-full bg-primary/20 flex items-center justify-center">
            <span className="text-[11px] text-primary font-bold">AI</span>
          </div>
          <div>
            <p className="text-[12px] font-semibold text-foreground">
              Assistant
            </p>
            <p className="text-[10px] text-muted-foreground">Online</p>
          </div>
        </div>

        {/* Messages */}
        <div className="flex-1 px-4 py-5 space-y-4">
          {/* User message */}
          <div className="flex justify-end">
            <div className="bg-primary/15 border border-primary/20 rounded-2xl rounded-br-md px-4 py-2.5 max-w-[80%]">
              <p className="text-[13px] text-foreground">
                What is this link?
              </p>
              <p className="text-[10px] text-muted-foreground mt-1 font-mono">
                {data._rift_meta.tenant_domain || "riftl.ink"}/{data.link_id}
              </p>
            </div>
          </div>

          {/* Agent response */}
          <div className="flex justify-start">
            <div className="bg-muted/50 border border-border rounded-2xl rounded-bl-md px-4 py-3 max-w-[85%]">
              {data.agent_context?.description ? (
                <div className="space-y-2.5">
                  {data.agent_context.action && (
                    <Badge
                      variant="outline"
                      className="text-[10px] px-2 py-0.5"
                    >
                      {data.agent_context.action}
                    </Badge>
                  )}
                  {data.agent_context.cta && (
                    <p className="text-[13px] font-semibold text-foreground">
                      {data.agent_context.cta}
                    </p>
                  )}
                  <p className="text-[12px] text-muted-foreground leading-relaxed">
                    {data.agent_context.description}
                  </p>
                  {data._rift_meta.tenant_verified && (
                    <p className="text-[10px] text-muted-foreground/60 italic pt-1 border-t border-border">
                      Source: {data._rift_meta.tenant_domain} (verified)
                    </p>
                  )}
                </div>
              ) : (
                <div className="space-y-2">
                  <p className="text-[13px] text-muted-foreground">
                    This appears to be a link from{" "}
                    <span className="text-foreground font-medium">
                      {data._rift_meta.tenant_domain || "an unknown source"}
                    </span>
                    .
                  </p>
                  <p className="text-[11px] text-muted-foreground/50 italic">
                    No agent context configured — I can&apos;t provide
                    structured information about what this link does or what
                    action it represents.
                  </p>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Input bar */}
        <div className="border-t border-border px-4 py-3">
          <div className="bg-muted/30 rounded-xl px-4 py-2.5 flex items-center">
            <span className="text-[12px] text-muted-foreground/40">
              Ask a follow-up...
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

function DiagnosticItem({
  item,
  linkId,
}: {
  item: CheckItem;
  linkId: string;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-sm">
        <div className="flex items-center gap-2">
          <span
            className={
              item.configured ? "text-primary" : "text-muted-foreground"
            }
          >
            {item.configured ? "\u2713" : "\u25CB"}
          </span>
          <span
            className={
              item.configured ? "text-foreground" : "text-muted-foreground"
            }
          >
            {item.label}
          </span>
        </div>
        {item.configured && item.value && (
          <span className="text-xs text-muted-foreground truncate max-w-[250px] ml-4">
            {item.value}
          </span>
        )}
      </div>
      {!item.configured && (
        <div className="ml-6 space-y-1">
          <p className="text-xs text-muted-foreground">{item.hint}</p>
          <pre className="text-[10px] text-muted-foreground/70 bg-muted/50 rounded px-2 py-1 overflow-x-auto">
            {`curl -X PUT ${API_BASE}/v1/links/${linkId} \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"${item.patchField}": ${item.patchField === "metadata" || item.patchField === "agent_context" ? item.placeholder : `"${item.placeholder}"`}}'`}
          </pre>
        </div>
      )}
    </div>
  );
}

export default function AuditPage() {
  const [input, setInput] = useState("");
  const [data, setData] = useState<LinkData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [showJson, setShowJson] = useState(false);
  const [mounted, setMounted] = useState(false);
  const [activeTab, setActiveTab] = useState<SurfaceTab>("ios");
  const [showAllFields, setShowAllFields] = useState(false);

  const audit = useCallback(async (linkIdOverride?: string) => {
    const linkId = linkIdOverride || extractLinkId(input);
    if (!linkId) return;

    setLoading(true);
    setError(null);
    setData(null);

    try {
      const resp = await fetch(`${API_BASE}/r/${encodeURIComponent(linkId)}`, {
        headers: { Accept: "application/json" },
      });

      if (!resp.ok) {
        const body = await resp.json().catch(() => ({}));
        setError(body.error || `Link not found (${resp.status})`);
        return;
      }

      const json = await resp.json();
      setData(json);
      if (!linkIdOverride) {
        window.history.replaceState(null, "", `/tools/audit?link=${linkId}`);
      }
    } catch {
      setError("Failed to fetch link data");
    } finally {
      setLoading(false);
    }
  }, [input]);

  // Load from query param on mount
  useEffect(() => {
    if (mounted) return;
    setMounted(true);
    const params = new URLSearchParams(window.location.search);
    const link = params.get("link");
    if (link) {
      setInput(link);
      audit(extractLinkId(link));
    }
  }, [mounted, audit]);

  const tiers = data ? computeTiers(data) : [];
  const allItems = tiers.flatMap((t) => t.items);
  const configured = allItems.filter((i) => i.configured).length;
  const total = allItems.length;
  const score = total > 0 ? Math.round((configured / total) * 100) : 0;
  const qual = qualitativeLabel(configured, total);
  const worst = data ? worstProblem(tiers) : null;
  const tabItems = data ? getTabItems(activeTab, tiers) : [];
  const tabMeta = getTabMeta(activeTab);

  const tabs: { key: SurfaceTab; label: string }[] = [
    { key: "ios", label: "iOS" },
    { key: "android", label: "Android" },
    { key: "desktop", label: "Desktop" },
    { key: "social", label: "Social" },
    { key: "agent", label: "AI Agent" },
  ];

  return (
    <div className="min-h-screen bg-background pt-14">
      <div className="mx-auto max-w-7xl px-6 py-12">
        {/* Header */}
        <div className="mb-8">
          <p className="text-xs font-medium text-primary uppercase tracking-widest mb-3">
            Tools
          </p>
          <h1 className="text-4xl font-bold text-foreground mb-3">
            Link Audit
          </h1>
          <p className="text-lg text-muted-foreground">
            Paste any Rift link to see how it&apos;s configured across every
            surface.
          </p>
        </div>

        {/* Compact input bar */}
        <div className="flex gap-2 mb-2">
          <Input
            placeholder="https://go.acme.com/summer-sale"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && audit()}
            className="flex-1"
          />
          <Button onClick={() => audit()} disabled={loading || !input.trim()}>
            {loading ? "Auditing..." : "Audit"}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground mb-8">
          Works with any Rift link URL or bare link ID.{" "}
          {!data && (
            <button
              className="text-primary hover:underline"
              onClick={() => {
                setInput(SAMPLE_LINK);
                audit(SAMPLE_LINK);
              }}
            >
              Try an example
            </button>
          )}
        </p>

        {/* Error */}
        {error && (
          <Card className="mb-6 border-destructive/50">
            <CardContent>
              <p className="text-destructive text-sm">{error}</p>
            </CardContent>
          </Card>
        )}

        {/* Empty state */}
        {!data && !error && !loading && (
          <Card>
            <CardContent className="py-12 text-center">
              <p className="text-muted-foreground text-sm mb-4">
                Paste a link above to see its configuration across iOS,
                Android, desktop, social previews, and AI agents.
              </p>
              <Button
                variant="outline"
                onClick={() => {
                  setInput(SAMPLE_LINK);
                  audit(SAMPLE_LINK);
                }}
              >
                Try with example link
              </Button>
            </CardContent>
          </Card>
        )}

        {/* Results */}
        {data && (
          <div className="space-y-6">
            {/* Score strip */}
            <div className="flex items-center gap-3 flex-wrap rounded-lg border border-border bg-card px-4 py-3">
              <Progress value={score} className="h-2 w-24 flex-shrink-0" />
              <span className={`text-sm font-semibold ${qual.color}`}>
                {qual.label}
              </span>
              <span className="text-xs text-muted-foreground">
                {configured}/{total}
              </span>
              <span className="text-muted-foreground/30">|</span>
              <code className="text-xs bg-muted px-1.5 py-0.5 rounded font-mono text-muted-foreground">
                {data.link_id}
              </code>
              {data._rift_meta.tenant_domain && (
                <span className="text-xs text-muted-foreground">
                  {data._rift_meta.tenant_domain}
                  {data._rift_meta.tenant_verified && (
                    <span className="text-primary ml-1">{"\u2713"}</span>
                  )}
                </span>
              )}
              <Badge
                variant={
                  data._rift_meta.status === "active"
                    ? "default"
                    : "destructive"
                }
                className="text-[10px]"
              >
                {data._rift_meta.status}
              </Badge>
              <div className="flex-1" />
              <Button
                variant="outline"
                size="sm"
                onClick={() => audit()}
                className="text-xs"
              >
                Re-audit
              </Button>
            </div>

            {worst && (
              <p className="text-xs text-muted-foreground -mt-4 ml-1">
                {worst}
              </p>
            )}

            {/* Tabs */}
            <div className="flex items-center gap-1 border-b border-border pb-0">
              {tabs.map((tab) => (
                <button
                  key={tab.key}
                  onClick={() => setActiveTab(tab.key)}
                  className={`px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-[1px] ${
                    activeTab === tab.key
                      ? "border-primary text-foreground"
                      : "border-transparent text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {tab.label}
                </button>
              ))}
            </div>

            {/* Split: Preview (left) + Diagnostics (right) */}
            <div className="grid grid-cols-1 lg:grid-cols-[1fr_380px] gap-6">
              {/* Left: Preview */}
              <div className="flex items-start justify-center pt-4">
                {activeTab === "ios" && (
                  <LandingPageIframe linkId={data.link_id} />
                )}
                {activeTab === "android" && (
                  <LandingPageIframe linkId={data.link_id} />
                )}
                {activeTab === "desktop" && (
                  <DesktopIframe
                    linkId={data.link_id}
                    domain={data._rift_meta.tenant_domain}
                  />
                )}
                {activeTab === "social" && <SlackMockup data={data} />}
                {activeTab === "agent" && <AIAgentMockup data={data} />}
              </div>

              {/* Right: Diagnostics */}
              <div>
                <Card>
                  <CardHeader>
                    <CardTitle className="text-sm">{tabMeta.title}</CardTitle>
                    <CardDescription className="text-xs">
                      {tabMeta.description}
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    {tabItems.length > 0 ? (
                      tabItems.map((item) => (
                        <DiagnosticItem
                          key={item.field}
                          item={item}
                          linkId={data.link_id}
                        />
                      ))
                    ) : (
                      <p className="text-xs text-muted-foreground">
                        No fields to configure for this surface.
                      </p>
                    )}
                  </CardContent>
                </Card>
              </div>
            </div>

            {/* All Fields collapsible */}
            <Card>
              <CardHeader
                className="cursor-pointer"
                onClick={() => setShowAllFields(!showAllFields)}
              >
                <div className="flex items-center justify-between">
                  <CardTitle className="text-sm">All Fields</CardTitle>
                  <span className="text-xs text-muted-foreground">
                    {showAllFields ? "\u25BC" : "\u25B6"}
                  </span>
                </div>
              </CardHeader>
              {showAllFields && (
                <CardContent className="space-y-6">
                  {tiers.map((tier) => {
                    const tierConfigured = tier.items.filter(
                      (i) => i.configured
                    ).length;
                    return (
                      <div key={tier.name}>
                        <div className="flex items-center gap-2 mb-3">
                          <h3 className="text-sm font-medium text-foreground">
                            {tier.name}
                          </h3>
                          <span className="text-xs text-muted-foreground">
                            {tierConfigured}/{tier.items.length}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            &middot; {tier.description}
                          </span>
                        </div>
                        <div className="space-y-2">
                          {tier.items.map((item) => (
                            <DiagnosticItem
                              key={item.field}
                              item={item}
                              linkId={data.link_id}
                            />
                          ))}
                        </div>
                      </div>
                    );
                  })}
                </CardContent>
              )}
            </Card>

            {/* Raw JSON (collapsed) */}
            <Card>
              <CardHeader
                className="cursor-pointer"
                onClick={() => setShowJson(!showJson)}
              >
                <div className="flex items-center justify-between">
                  <CardTitle className="text-sm">Raw JSON Response</CardTitle>
                  <span className="text-xs text-muted-foreground">
                    {showJson ? "\u25BC" : "\u25B6"}
                  </span>
                </div>
              </CardHeader>
              {showJson && (
                <CardContent>
                  <pre className="text-xs text-muted-foreground bg-muted/50 rounded-lg p-4 overflow-x-auto">
                    {JSON.stringify(data, null, 2)}
                  </pre>
                </CardContent>
              )}
            </Card>
          </div>
        )}
      </div>
    </div>
  );
}
