"use client";

import { useState } from "react";
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
  label: string;
  configured: boolean;
  value: string | null;
  hint: string;
  docs: string;
}

function extractLinkId(input: string): string {
  const trimmed = input.trim();
  // Handle full URLs: https://go.example.com/my-link or https://api.riftl.ink/r/my-link
  try {
    const url = new URL(trimmed);
    const segments = url.pathname.split("/").filter(Boolean);
    // /r/link-id → take last segment
    return segments[segments.length - 1] || trimmed;
  } catch {
    // Not a URL — treat as bare link_id
    return trimmed;
  }
}

function computeChecks(data: LinkData): CheckItem[] {
  return [
    {
      label: "iOS Store URL",
      configured: !!data.ios_store_url,
      value: data.ios_store_url,
      hint: "iOS users without the app won't have a store fallback",
      docs: "/docs/links",
    },
    {
      label: "Android Store URL",
      configured: !!data.android_store_url,
      value: data.android_store_url,
      hint: "Android users without the app won't have a store fallback",
      docs: "/docs/links",
    },
    {
      label: "Web URL",
      configured: !!data.web_url,
      value: data.web_url,
      hint: "Desktop users won't have a destination",
      docs: "/docs/links",
    },
    {
      label: "iOS Deep Link",
      configured: !!data.ios_deep_link,
      value: data.ios_deep_link,
      hint: "App won't know which screen to open after Universal Link",
      docs: "/docs/links",
    },
    {
      label: "Android Deep Link",
      configured: !!data.android_deep_link,
      value: data.android_deep_link,
      hint: "App won't know which screen to open after App Link",
      docs: "/docs/links",
    },
    {
      label: "Social Title",
      configured: !!data.metadata?.title,
      value: data.metadata?.title || null,
      hint: "Social previews will show the raw URL instead of a title",
      docs: "/docs/links",
    },
    {
      label: "Social Description",
      configured: !!data.metadata?.description,
      value: data.metadata?.description || null,
      hint: "Social previews won't have a description",
      docs: "/docs/links",
    },
    {
      label: "Social Image",
      configured: !!data.metadata?.image,
      value: data.metadata?.image || null,
      hint: "Social previews won't have an image",
      docs: "/docs/links",
    },
    {
      label: "Agent Context",
      configured: !!(
        data.agent_context?.action ||
        data.agent_context?.cta ||
        data.agent_context?.description
      ),
      value: data.agent_context?.action || null,
      hint: "AI agents won't understand what this link does",
      docs: "/docs/links",
    },
  ];
}

export default function AuditPage() {
  const [input, setInput] = useState("");
  const [data, setData] = useState<LinkData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function audit() {
    const linkId = extractLinkId(input);
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

      // Update URL for shareability
      window.history.replaceState(null, "", `/tools/audit?link=${linkId}`);
    } catch {
      setError("Failed to fetch link data");
    } finally {
      setLoading(false);
    }
  }

  // Load from query param on mount
  if (typeof window !== "undefined" && !data && !loading && !input) {
    const params = new URLSearchParams(window.location.search);
    const link = params.get("link");
    if (link) {
      setInput(link);
      // Defer the audit to next tick
      setTimeout(() => {
        const id = extractLinkId(link);
        if (id) {
          setInput(link);
          audit();
        }
      }, 0);
    }
  }

  const checks = data ? computeChecks(data) : [];
  const configured = checks.filter((c) => c.configured).length;
  const total = checks.length;
  const score = total > 0 ? Math.round((configured / total) * 100) : 0;

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
            Paste a Rift link to see how it appears across every surface.
          </p>
        </div>

        {/* Input */}
        <div className="flex gap-2 mb-8">
          <Input
            placeholder="https://go.yourcompany.com/link-id"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && audit()}
            className="flex-1"
          />
          <Button onClick={audit} disabled={loading || !input.trim()}>
            {loading ? "Auditing..." : "Audit"}
          </Button>
        </div>

        {/* Error */}
        {error && (
          <Card className="mb-6 border-destructive/50">
            <CardContent>
              <p className="text-destructive text-sm">{error}</p>
            </CardContent>
          </Card>
        )}

        {/* Results */}
        {data && (
          <div className="space-y-6">
            {/* Score */}
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle>
                      {configured}/{total} configured
                    </CardTitle>
                    <CardDescription>
                      {data.link_id}
                      {data._rift_meta.tenant_domain && (
                        <span>
                          {" "}
                          &middot; {data._rift_meta.tenant_domain}
                          {data._rift_meta.tenant_verified && (
                            <span className="text-primary"> ✓</span>
                          )}
                        </span>
                      )}
                    </CardDescription>
                  </div>
                  <Badge
                    variant={
                      data._rift_meta.status === "active"
                        ? "default"
                        : "destructive"
                    }
                  >
                    {data._rift_meta.status}
                  </Badge>
                </div>
              </CardHeader>
              <CardContent>
                <Progress value={score} className="h-2" />
              </CardContent>
            </Card>

            {/* Platform Coverage */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <Card size="sm">
                <CardHeader>
                  <CardTitle>iOS</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Store URL</span>
                    <Badge variant={data.ios_store_url ? "default" : "outline"}>
                      {data.ios_store_url ? "✓" : "—"}
                    </Badge>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Deep Link</span>
                    <Badge
                      variant={data.ios_deep_link ? "default" : "outline"}
                    >
                      {data.ios_deep_link ? "✓" : "—"}
                    </Badge>
                  </div>
                </CardContent>
              </Card>

              <Card size="sm">
                <CardHeader>
                  <CardTitle>Android</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Store URL</span>
                    <Badge
                      variant={data.android_store_url ? "default" : "outline"}
                    >
                      {data.android_store_url ? "✓" : "—"}
                    </Badge>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Deep Link</span>
                    <Badge
                      variant={data.android_deep_link ? "default" : "outline"}
                    >
                      {data.android_deep_link ? "✓" : "—"}
                    </Badge>
                  </div>
                </CardContent>
              </Card>

              <Card size="sm">
                <CardHeader>
                  <CardTitle>Desktop</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Web URL</span>
                    <Badge variant={data.web_url ? "default" : "outline"}>
                      {data.web_url ? "✓" : "—"}
                    </Badge>
                  </div>
                </CardContent>
              </Card>
            </div>

            {/* Social Preview */}
            <Card>
              <CardHeader>
                <CardTitle>Social Preview</CardTitle>
                <CardDescription>
                  How this link appears when shared on Twitter, Slack, iMessage
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="rounded-lg border border-border overflow-hidden">
                  {data.metadata?.image && (
                    <div className="h-40 bg-muted flex items-center justify-center overflow-hidden">
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img
                        src={data.metadata.image}
                        alt="Social preview"
                        className="w-full h-full object-cover"
                      />
                    </div>
                  )}
                  {!data.metadata?.image && (
                    <div className="h-32 bg-muted/50 flex items-center justify-center">
                      <span className="text-xs text-muted-foreground">
                        No preview image configured
                      </span>
                    </div>
                  )}
                  <div className="p-3 space-y-1">
                    <p className="text-sm font-medium text-foreground">
                      {data.metadata?.title || (
                        <span className="text-muted-foreground italic">
                          No title
                        </span>
                      )}
                    </p>
                    <p className="text-xs text-muted-foreground line-clamp-2">
                      {data.metadata?.description || (
                        <span className="italic">No description</span>
                      )}
                    </p>
                    <p className="text-xs text-muted-foreground/60">
                      {data._rift_meta.tenant_domain || "riftl.ink"}
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Agent Context */}
            <Card>
              <CardHeader>
                <CardTitle>AI Agent View</CardTitle>
                <CardDescription>
                  What AI agents see when they resolve this link
                </CardDescription>
              </CardHeader>
              <CardContent>
                {data.agent_context &&
                (data.agent_context.action ||
                  data.agent_context.cta ||
                  data.agent_context.description) ? (
                  <div className="space-y-3">
                    <div className="flex gap-2">
                      {data.agent_context.action && (
                        <Badge variant="outline">
                          {data.agent_context.action}
                        </Badge>
                      )}
                      {data._rift_meta.tenant_verified && (
                        <Badge variant="outline" className="text-primary">
                          verified
                        </Badge>
                      )}
                    </div>
                    {data.agent_context.cta && (
                      <p className="text-sm font-medium text-foreground">
                        {data.agent_context.cta}
                      </p>
                    )}
                    {data.agent_context.description && (
                      <p className="text-sm text-muted-foreground">
                        {data.agent_context.description}
                      </p>
                    )}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground italic">
                    No agent context configured. AI agents won&apos;t understand
                    what this link does.{" "}
                    <a
                      href="/docs/links"
                      className="text-primary hover:underline"
                    >
                      Learn more
                    </a>
                  </p>
                )}
              </CardContent>
            </Card>

            {/* Checklist */}
            <Card>
              <CardHeader>
                <CardTitle>Configuration Checklist</CardTitle>
                <CardDescription>
                  {configured === total
                    ? "Everything is configured"
                    : `${total - configured} item${total - configured === 1 ? "" : "s"} missing`}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {checks.map((check) => (
                    <div
                      key={check.label}
                      className="flex items-center justify-between text-sm"
                    >
                      <div className="flex items-center gap-2">
                        <span
                          className={
                            check.configured
                              ? "text-primary"
                              : "text-muted-foreground"
                          }
                        >
                          {check.configured ? "✓" : "○"}
                        </span>
                        <span
                          className={
                            check.configured
                              ? "text-foreground"
                              : "text-muted-foreground"
                          }
                        >
                          {check.label}
                        </span>
                      </div>
                      {!check.configured && (
                        <a
                          href={check.docs}
                          className="text-xs text-primary hover:underline"
                        >
                          Fix
                        </a>
                      )}
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>

            {/* Raw JSON */}
            <Card>
              <CardHeader>
                <CardTitle>Raw JSON Response</CardTitle>
                <CardDescription>
                  What{" "}
                  <code className="text-xs bg-muted px-1 py-0.5 rounded">
                    Accept: application/json
                  </code>{" "}
                  returns
                </CardDescription>
              </CardHeader>
              <CardContent>
                <pre className="text-xs text-muted-foreground bg-muted/50 rounded-lg p-4 overflow-x-auto">
                  {JSON.stringify(data, null, 2)}
                </pre>
              </CardContent>
            </Card>
          </div>
        )}
      </div>
    </div>
  );
}
