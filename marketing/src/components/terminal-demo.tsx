"use client";

import { motion } from "motion/react";

type ChatMessage = {
  role: "user" | "agent";
  text: string;
  linkCard?: {
    url: string;
    title: string;
    description: string;
  };
};

const AGENT_CHAT: ChatMessage[] = [
  {
    role: "user",
    text: "Can you handle this for me?",
    linkCard: {
      url: "pay.acme.com/inv_2048",
      title: "Acme Invoice #2048",
      description: "$420.00 due today.",
    },
  },
  {
    role: "agent",
    text: "Yes. It resolves to a payment action for Invoice #2048, so I can complete it directly instead of sending you to the payment page.",
  },
  { role: "user", text: "Perfect, thank you." },
];

function UserJourneyDemo({ title }: { title: string }) {
  return (
    <div className="window flex-1">
      <div className="window-bar">
        <div className="window-dot" />
        <div className="window-dot" />
        <div className="window-dot" />
        <span className="ml-2 text-[11px] text-[#52525b] font-mono">{title}</span>
      </div>
      <div className="p-5 min-h-[260px] space-y-4">
        <motion.div
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.18, delay: 0.04 }}
          className="rounded-2xl border border-[#27272a] bg-[#101012] p-4"
        >
          <div className="mb-2 flex items-center justify-between gap-3">
            <div>
              <p className="text-[10px] font-mono uppercase tracking-widest text-[#71717a]">
                Shared link
              </p>
              <p className="text-[14px] font-medium text-[#fafafa]">
                Acme Invoice #2048
              </p>
            </div>
            <span className="rounded-full bg-[#2dd4bf]/10 px-2 py-1 text-[10px] font-mono text-[#2dd4bf]">
              Tap to open
            </span>
          </div>
          <p className="text-[12px] text-[#a1a1aa]">
            Review and pay the invoice due today.
          </p>
          <p className="mt-3 font-mono text-[11px] text-[#71717a]">
            pay.acme.com/inv_2048
          </p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.18, delay: 0.1 }}
          className="flex items-center justify-center"
        >
          <div className="rounded-full border border-[#2dd4bf]/20 bg-[#0c0c0e] px-3 py-1 text-[11px] font-mono text-[#2dd4bf]">
            opens the right app or webpage
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.18, delay: 0.16 }}
          className="grid gap-3 md:grid-cols-2"
        >
          <div className="rounded-2xl border border-[#1e1e22] bg-[#111113] p-4">
            <div className="mb-3 flex items-center justify-between">
              <p className="text-[10px] font-mono uppercase tracking-widest text-[#2dd4bf]">
                In app
              </p>
              <span className="rounded-full bg-[#18181b] px-2 py-1 text-[10px] font-mono text-[#71717a]">
                Deep link
              </span>
            </div>
            <h3 className="text-base font-semibold text-[#fafafa]">
              Acme Pay app
            </h3>
            <p className="mt-1 text-[12px] text-[#a1a1aa]">
              Opens the invoice and payment flow directly in the app.
            </p>
            <div className="mt-4 rounded-xl border border-[#222225] bg-[#0f1012] p-3">
              <p className="text-[10px] font-mono uppercase tracking-widest text-[#71717a]">
                Destination
              </p>
              <p className="mt-1 font-mono text-[11px] text-[#d4d4d8]">
                acmepay://invoices/2048
              </p>
            </div>
          </div>

          <div className="rounded-2xl border border-[#1e1e22] bg-[#111113] p-4">
            <div className="mb-3 flex items-center justify-between">
              <p className="text-[10px] font-mono uppercase tracking-widest text-[#60a5fa]">
                On web
              </p>
              <span className="rounded-full bg-[#18181b] px-2 py-1 text-[10px] font-mono text-[#71717a]">
                Fallback
              </span>
            </div>
            <h3 className="text-base font-semibold text-[#fafafa]">
              Invoice payment page
            </h3>
            <p className="mt-1 text-[12px] text-[#a1a1aa]">
              Falls back to the hosted payment page when the app is unavailable.
            </p>
            <div className="mt-4 rounded-xl border border-[#222225] bg-[#0f1012] p-3">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <p className="text-[10px] font-mono uppercase tracking-widest text-[#71717a]">
                    Amount
                  </p>
                  <p className="mt-1 text-sm text-[#fafafa]">$420.00</p>
                </div>
                <div>
                  <p className="text-[10px] font-mono uppercase tracking-widest text-[#71717a]">
                    Due
                  </p>
                  <p className="mt-1 text-sm text-[#fafafa]">Today</p>
                </div>
              </div>
              <p className="mt-3 font-mono text-[11px] text-[#d4d4d8]">
                pay.acme.com/inv_2048
              </p>
            </div>
          </div>
        </motion.div>
      </div>
    </div>
  );
}

function ChatDemo({ title, messages }: { title: string; messages: ChatMessage[] }) {
  return (
    <div className="window flex-1">
      <div className="window-bar">
        <div className="window-dot" />
        <div className="window-dot" />
        <div className="window-dot" />
        <span className="ml-2 text-[11px] text-[#52525b] font-mono">{title}</span>
      </div>
      <div className="p-5 min-h-[220px] space-y-3">
        {messages.map((message, i) => (
          <motion.div
            key={`${message.role}-${i}`}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.18, delay: i * 0.08 }}
            className={`flex ${message.role === "user" ? "justify-start" : "justify-end"}`}
          >
            <div
              className={`max-w-[85%] rounded-2xl px-4 py-3 text-[13px] leading-relaxed ${
                message.role === "user"
                  ? "bg-[#18181b] text-[#d4d4d8] border border-[#27272a]"
                  : "bg-[#2dd4bf]/10 text-[#ccfbf1] border border-[#2dd4bf]/20"
              }`}
            >
              <div className="mb-1 text-[10px] font-mono uppercase tracking-widest text-[#71717a]">
                {message.role}
              </div>
              <p>{message.text}</p>
              {message.linkCard ? (
                <div className="mt-3 overflow-hidden rounded-xl border border-[#2dd4bf]/15 bg-[#0f1012]">
                  <div className="h-1 bg-gradient-to-r from-[#2dd4bf] via-[#5eead4] to-[#2dd4bf]/30" />
                  <div className="space-y-1 px-3 py-3">
                    <p className="text-[10px] font-mono uppercase tracking-widest text-[#2dd4bf]">
                      Link Preview
                    </p>
                    <p className="text-[13px] font-medium text-[#fafafa]">
                      {message.linkCard.title}
                    </p>
                    <p className="text-[12px] text-[#a1a1aa]">
                      {message.linkCard.description}
                    </p>
                    <p className="font-mono text-[11px] text-[#71717a]">
                      {message.linkCard.url}
                    </p>
                  </div>
                </div>
              ) : null}
            </div>
          </motion.div>
        ))}
      </div>
    </div>
  );
}

export function TerminalDemo() {
  return (
    <div className="flex flex-col lg:flex-row gap-4">
      <UserJourneyDemo title="user journey" />
      <ChatDemo title="agent session" messages={AGENT_CHAT} />
    </div>
  );
}
