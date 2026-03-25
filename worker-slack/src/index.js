/**
 * Rift → Slack Webhook Proxy
 *
 * Receives Rift webhook payloads and forwards them to Slack
 * as formatted messages.
 *
 * Set SLACK_WEBHOOK_URL as a secret:
 *   wrangler secret put SLACK_WEBHOOK_URL
 */

export default {
  async fetch(request, env) {
    if (request.method !== "POST") {
      return new Response("Method not allowed", { status: 405 });
    }

    const payload = await request.json();
    const { event, data } = payload;

    let text;
    if (event === "click") {
      text = [
        `:link: *Click* on \`${data.link_id}\``,
        `Platform: ${data.platform}`,
        data.referer ? `Referer: ${data.referer}` : null,
        `Time: ${data.timestamp}`,
      ]
        .filter(Boolean)
        .join("\n");
    } else if (event === "attribution") {
      text = [
        `:white_check_mark: *Attribution* on \`${data.link_id}\``,
        `Install: \`${data.install_id}\``,
        `App version: ${data.app_version}`,
        `Time: ${data.timestamp}`,
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
