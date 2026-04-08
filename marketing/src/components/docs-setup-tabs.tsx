"use client";

import { useState } from "react";

type Tab = {
  id: string;
  label: string;
  children: React.ReactNode;
};

export function DocsSetupTabs({
  title,
  tabs,
}: {
  title: string;
  tabs: Tab[];
}) {
  const [activeTab, setActiveTab] = useState(tabs[0]?.id ?? "");
  const active = tabs.find((tab) => tab.id === activeTab) ?? tabs[0];

  return (
    <section className="space-y-5">
      <h2 className="text-2xl font-bold text-[#fafafa]">{title}</h2>

      <div className="rounded-2xl border border-[#1e1e22] bg-[#0f1115] p-2">
        <div className="flex flex-wrap gap-2 border-b border-[#1e1e22] px-2 pb-3 pt-1">
          {tabs.map((tab) => {
            const active = tab.id === activeTab;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveTab(tab.id)}
                className={`rounded-full px-4 py-2 text-[13px] font-medium transition-colors ${
                  active
                    ? "bg-[#2dd4bf]/10 text-[#2dd4bf]"
                    : "bg-[#111113] text-[#71717a] hover:text-[#fafafa]"
                }`}
              >
                {tab.label}
              </button>
            );
          })}
        </div>

        <div className="p-4 md:p-5">{active?.children}</div>
      </div>
    </section>
  );
}
