/**
 * Rift → Slack Webhook Proxy
 *
 * Receives Rift webhook payloads, parses user agent + geo data,
 * and forwards enriched notifications to Slack.
 *
 * Set SLACK_WEBHOOK_URL as a secret:
 *   wrangler secret put SLACK_WEBHOOK_URL
 */

function parseUserAgent(ua) {
  if (!ua) return { device: "Unknown", os: "Unknown", browser: "Unknown" };

  let device = "Desktop";
  let os = "Unknown";
  let browser = "Unknown";

  // OS detection
  if (/iPhone/.test(ua)) {
    device = "iPhone";
    const m = ua.match(/iPhone OS (\d+[_\.]\d+)/);
    os = m ? `iOS ${m[1].replace("_", ".")}` : "iOS";
  } else if (/iPad/.test(ua)) {
    device = "iPad";
    const m = ua.match(/OS (\d+[_\.]\d+)/);
    os = m ? `iPadOS ${m[1].replace("_", ".")}` : "iPadOS";
  } else if (/Android/.test(ua)) {
    const m = ua.match(/Android (\d+[\.\d]*)/);
    os = m ? `Android ${m[1]}` : "Android";
    // Try to get device model
    const dm = ua.match(/;\s*([^;)]+)\s*Build/);
    device = dm ? dm[1].trim() : "Android";
  } else if (/Mac OS X/.test(ua)) {
    const m = ua.match(/Mac OS X (\d+[_\.]\d+)/);
    os = m ? `macOS ${m[1].replace(/_/g, ".")}` : "macOS";
  } else if (/Windows NT/.test(ua)) {
    os = "Windows";
  } else if (/Linux/.test(ua)) {
    os = "Linux";
  }

  // Browser detection (order matters — check in-app browsers first)
  if (/Instagram/.test(ua)) {
    browser = "Instagram";
  } else if (/FBAN|FBAV/.test(ua)) {
    browser = "Facebook";
  } else if (/Twitter|X-Twitter/.test(ua)) {
    browser = "Twitter/X";
  } else if (/LinkedInApp/.test(ua)) {
    browser = "LinkedIn";
  } else if (/CriOS/.test(ua)) {
    browser = "Chrome (iOS)";
  } else if (/FxiOS/.test(ua)) {
    browser = "Firefox (iOS)";
  } else if (/EdgA?\//.test(ua)) {
    browser = "Edge";
  } else if (/Chrome\//.test(ua) && !/Chromium/.test(ua)) {
    browser = "Chrome";
  } else if (/Safari\//.test(ua) && !/Chrome/.test(ua)) {
    browser = "Safari";
  } else if (/Firefox\//.test(ua)) {
    browser = "Firefox";
  }

  return { device, os, browser };
}

function cleanReferer(referer) {
  if (!referer) return null;
  try {
    const host = new URL(referer).hostname.replace(/^www\./, "");
    const names = {
      "t.co": "Twitter/X",
      "twitter.com": "Twitter/X",
      "x.com": "Twitter/X",
      "facebook.com": "Facebook",
      "l.facebook.com": "Facebook",
      "lm.facebook.com": "Facebook",
      "instagram.com": "Instagram",
      "l.instagram.com": "Instagram",
      "linkedin.com": "LinkedIn",
      "lnkd.in": "LinkedIn",
      "youtube.com": "YouTube",
      "reddit.com": "Reddit",
      "old.reddit.com": "Reddit",
    };
    return names[host] || host;
  } catch {
    return referer;
  }
}

function formatLocation(data) {
  const parts = [data.city, data.region, data.country].filter(Boolean);
  return parts.length > 0 ? parts.join(", ") : null;
}

export default {
  async fetch(request, env) {
    if (request.method !== "POST") {
      return new Response("Method not allowed", { status: 405 });
    }

    const payload = await request.json();
    const { event, data } = payload;

    let text;
    if (event === "click") {
      const ua = parseUserAgent(data.user_agent);
      const source = cleanReferer(data.referer);
      const location = formatLocation(data);

      const lines = [
        `:link: *${data.link_id}*`,
        `${ua.device} · ${ua.os} · ${ua.browser}`,
      ];
      if (location) lines.push(`\u{1F4CD} ${location}`);
      if (source) lines.push(`From *${source}*`);

      text = lines.join("\n");
    } else if (event === "attribution") {
      text = [
        `:white_check_mark: *Install* · \`${data.link_id}\``,
        `Install: \`${data.install_id}\` · v${data.app_version}`,
      ].join("\n");
    } else {
      text = `Rift event: ${event}\n\`\`\`${JSON.stringify(data, null, 2)}\`\`\``;
    }

    const slackUrl = env.SLACK_WEBHOOK_URL;
    if (!slackUrl) {
      return new Response("SLACK_WEBHOOK_URL not configured", { status: 500 });
    }

    const resp = await fetch(slackUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text }),
    });

    return new Response(resp.ok ? "ok" : "slack error", { status: resp.status });
  },
};
