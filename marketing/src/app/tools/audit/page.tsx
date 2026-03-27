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
          placeholder: "https://play.google.com/store/apps/details?id=com.example",
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

export default function AuditPage() {
  const [input, setInput] = useState("");
  const [data, setData] = useState<LinkData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [showJson, setShowJson] = useState(false);
  const [mounted, setMounted] = useState(false);

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

  return (
    <div className="min-h-screen bg-background pt-14">
      <div className="mx-auto max-w-3xl px-6 py-12">
        {/* Header */}
        <div className="mb-10">
          <p className="text-xs font-medium text-primary uppercase tracking-widest mb-3">
            Tools
          </p>
          <h1 className="text-4xl font-bold text-foreground mb-3">
            Link Audit
          </h1>
          <p className="text-lg text-muted-foreground">
            Paste any Rift link to see how it&apos;s configured across every surface.
          </p>
        </div>

        {/* Input */}
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
                Paste a link above to see its configuration across iOS, Android,
                desktop, social previews, and AI agents.
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
            {/* Score + Tiered Checklist (merged) */}
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle className="flex items-center gap-3">
                      <span className={qual.color}>{qual.label}</span>
                      <span className="text-sm font-normal text-muted-foreground">
                        {configured}/{total}
                      </span>
                    </CardTitle>
                    <CardDescription className="flex items-center gap-1.5">
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded font-mono">
                        {data.link_id}
                      </code>
                      {data._rift_meta.tenant_domain && (
                        <>
                          <span>&middot;</span>
                          <span>{data._rift_meta.tenant_domain}</span>
                          {data._rift_meta.tenant_verified && (
                            <span className="text-primary">✓</span>
                          )}
                        </>
                      )}
                    </CardDescription>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge
                      variant={
                        data._rift_meta.status === "active"
                          ? "default"
                          : "destructive"
                      }
                    >
                      {data._rift_meta.status}
                    </Badge>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => audit()}
                    >
                      Re-audit
                    </Button>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="space-y-6">
                <div>
                  <Progress value={score} className="h-2 mb-2" />
                  {worst && (
                    <p className="text-xs text-muted-foreground">{worst}</p>
                  )}
                </div>

                {/* Tiered checklist */}
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
                          <div key={item.field} className="space-y-1">
                            <div className="flex items-center justify-between text-sm">
                              <div className="flex items-center gap-2">
                                <span
                                  className={
                                    item.configured
                                      ? "text-primary"
                                      : "text-muted-foreground"
                                  }
                                >
                                  {item.configured ? "✓" : "○"}
                                </span>
                                <span
                                  className={
                                    item.configured
                                      ? "text-foreground"
                                      : "text-muted-foreground"
                                  }
                                >
                                  {item.label}
                                </span>
                              </div>
                              {item.configured && item.value && (
                                <span className="text-xs text-muted-foreground truncate max-w-[200px]">
                                  {item.value}
                                </span>
                              )}
                            </div>
                            {!item.configured && (
                              <div className="ml-6 space-y-1">
                                <p className="text-xs text-muted-foreground">
                                  {item.hint}
                                </p>
                                <pre className="text-[10px] text-muted-foreground/70 bg-muted/50 rounded px-2 py-1 overflow-x-auto">
                                  {`curl -X PUT ${API_BASE}/v1/links/${data.link_id} \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"${item.patchField}": ${item.patchField === "metadata" || item.patchField === "agent_context" ? item.placeholder : `"${item.placeholder}"`}}'`}
                                </pre>
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    </div>
                  );
                })}
              </CardContent>
            </Card>

            {/* Device Mockups */}
            <Card>
              <CardHeader>
                <CardTitle>How your link appears</CardTitle>
                <CardDescription>
                  Preview across mobile, social, and AI surfaces
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">

                  {/* iPhone mockup */}
                  <div>
                    <p className="text-xs font-medium text-muted-foreground mb-2 text-center">iPhone</p>
                    <div className="mx-auto w-[200px] rounded-[28px] border-[3px] border-foreground/20 bg-black p-1.5 shadow-lg">
                      {/* Notch */}
                      <div className="mx-auto w-20 h-4 bg-black rounded-b-xl relative z-10" />
                      {/* Screen */}
                      <div className="bg-[#0a0a0a] rounded-[22px] -mt-2 pt-6 pb-4 px-3 min-h-[340px] flex flex-col items-center justify-center text-center">
                        {data.metadata?.image && (
                          // eslint-disable-next-line @next/next/no-img-element
                          <img src={data.metadata.image} alt="" className="w-12 h-12 rounded-xl mb-3 object-cover" />
                        )}
                        <p className="text-[8px] font-semibold uppercase tracking-[0.15em] text-primary mb-2">
                          {data.metadata?.title || data.link_id}
                        </p>
                        <p className="text-[9px] text-white/60 mb-3 px-2 line-clamp-2">
                          {data.metadata?.description || "Open in app"}
                        </p>
                        <div className="bg-primary rounded-lg px-4 py-1.5">
                          <p className="text-[8px] font-semibold text-white">
                            {data.ios_store_url ? "Get the App" : data.web_url ? "Continue" : "Open"}
                          </p>
                        </div>
                        {data.ios_store_url && (
                          <p className="text-[7px] text-white/30 mt-2 truncate max-w-full">
                            {data.ios_store_url.replace("https://", "")}
                          </p>
                        )}
                      </div>
                      {/* Home indicator */}
                      <div className="mx-auto w-16 h-1 bg-foreground/20 rounded-full mt-1.5" />
                    </div>
                  </div>

                  {/* Slack unfurl mockup */}
                  <div>
                    <p className="text-xs font-medium text-muted-foreground mb-2 text-center">Slack</p>
                    <div className="bg-[#1a1d21] rounded-lg p-3 min-h-[340px] flex flex-col">
                      {/* Message */}
                      <div className="flex gap-2 mb-3">
                        <div className="w-7 h-7 rounded bg-primary/20 flex-shrink-0 flex items-center justify-center">
                          <span className="text-[10px] text-primary font-bold">U</span>
                        </div>
                        <div>
                          <p className="text-[10px] font-bold text-white">User</p>
                          <p className="text-[10px] text-[#d1d2d3]">Check out this link</p>
                        </div>
                      </div>
                      {/* Unfurl card */}
                      <div className="border-l-[3px] border-foreground/20 pl-3 ml-1">
                        <p className="text-[9px] text-[#8b8d90] mb-1">
                          {data._rift_meta.tenant_domain || "riftl.ink"}
                        </p>
                        <p className="text-[10px] font-bold text-[#1d9bd1] mb-0.5">
                          {data.metadata?.title || (
                            <span className="text-[#8b8d90] italic font-normal">No title</span>
                          )}
                        </p>
                        <p className="text-[9px] text-[#d1d2d3] line-clamp-2 mb-2">
                          {data.metadata?.description || (
                            <span className="text-[#8b8d90] italic">No description</span>
                          )}
                        </p>
                        {data.metadata?.image ? (
                          // eslint-disable-next-line @next/next/no-img-element
                          <img src={data.metadata.image} alt="" className="w-full h-28 object-cover rounded" />
                        ) : (
                          <div className="w-full h-16 bg-[#2c2d30] rounded flex items-center justify-center">
                            <span className="text-[8px] text-[#8b8d90]">No image</span>
                          </div>
                        )}
                      </div>
                      <div className="flex-1" />
                    </div>
                  </div>

                  {/* AI chat mockup */}
                  <div>
                    <p className="text-xs font-medium text-muted-foreground mb-2 text-center">AI Agent</p>
                    <div className="bg-[#111113] rounded-lg border border-border p-3 min-h-[340px] flex flex-col">
                      {/* User message */}
                      <div className="self-end bg-primary/10 rounded-2xl rounded-br-sm px-3 py-2 max-w-[85%] mb-3">
                        <p className="text-[10px] text-foreground">What is this link?</p>
                      </div>
                      {/* Agent response */}
                      <div className="self-start bg-muted/50 rounded-2xl rounded-bl-sm px-3 py-2 max-w-[90%]">
                        {data.agent_context?.description ? (
                          <div className="space-y-1.5">
                            {data.agent_context.action && (
                              <Badge variant="outline" className="text-[8px] px-1.5 py-0">
                                {data.agent_context.action}
                              </Badge>
                            )}
                            {data.agent_context.cta && (
                              <p className="text-[10px] font-medium text-foreground">
                                {data.agent_context.cta}
                              </p>
                            )}
                            <p className="text-[9px] text-muted-foreground leading-relaxed line-clamp-4">
                              {data.agent_context.description}
                            </p>
                            {data._rift_meta.tenant_verified && (
                              <p className="text-[8px] text-muted-foreground/50 italic">
                                Source: {data._rift_meta.tenant_domain} (verified)
                              </p>
                            )}
                          </div>
                        ) : (
                          <div className="space-y-1.5">
                            <p className="text-[10px] text-muted-foreground">
                              This appears to be a download link from{" "}
                              {data._rift_meta.tenant_domain || "an unknown source"}.
                            </p>
                            <p className="text-[9px] text-muted-foreground/50 italic">
                              No agent context configured — I can&apos;t tell you more about it.
                            </p>
                          </div>
                        )}
                      </div>
                      <div className="flex-1" />
                      {/* Input bar */}
                      <div className="mt-3 bg-muted/30 rounded-full px-3 py-1.5 flex items-center">
                        <span className="text-[9px] text-muted-foreground/40">Ask a follow-up...</span>
                      </div>
                    </div>
                  </div>
                </div>
              </CardContent>
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
                    {showJson ? "▼" : "▶"}
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
