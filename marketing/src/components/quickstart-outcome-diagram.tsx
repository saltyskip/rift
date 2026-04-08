export function QuickstartOutcomeDiagram() {
  return (
    <section className="space-y-5">
      <div className="flex items-end justify-between gap-4">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
            What You End Up With
          </p>
          <h2 className="mt-3 text-2xl font-bold text-[#fafafa]">One branded link, routed everywhere</h2>
        </div>
        <div className="hidden md:flex flex-wrap justify-end gap-2 text-[12px] text-[#d4d4d8]">
          <span className="rounded-full border border-[#2dd4bf]/20 bg-[#2dd4bf]/10 px-3 py-1">
            click tracked
          </span>
          <span className="rounded-full border border-[#60a5fa]/20 bg-[#60a5fa]/10 px-3 py-1">
            attribution ready
          </span>
          <span className="rounded-full border border-[#f59e0b]/20 bg-[#f59e0b]/10 px-3 py-1">
            agent-readable
          </span>
        </div>
      </div>

      <div className="rounded-[28px] border border-[#1e1e22] bg-[linear-gradient(180deg,#101318_0%,#0c0d10_100%)] p-5 md:p-6">
        <div className="grid gap-5 lg:grid-cols-[1.2fr_auto_1.8fr] lg:items-center">
          <div className="rounded-2xl border border-[#2dd4bf]/20 bg-[#0f1416] p-5">
            <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
              Branded Link
            </p>
            <p className="mt-3 text-xl font-semibold text-[#fafafa]">go.yourcompany.com/summer-sale</p>
            <p className="mt-2 text-[14px] leading-relaxed text-[#a1a1aa]">
              The one URL you share in ads, on your website, in email, or in messages.
            </p>
          </div>

          <div className="flex items-center justify-center text-[#2dd4bf] lg:flex-col">
            <span className="text-2xl">→</span>
            <span className="hidden text-[11px] uppercase tracking-[0.18em] text-[#52525b] lg:block">
              Rift routes it
            </span>
          </div>

          <div className="grid gap-3 md:grid-cols-3">
            <div className="rounded-2xl border border-[#1f2937] bg-[#111317] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#93bbfd]">
                iPhone
              </p>
              <p className="mt-2 text-[15px] font-medium text-[#fafafa]">Open the app</p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#8c93a3]">
                Universal Links or the App Store fallback, depending on what is installed.
              </p>
            </div>

            <div className="rounded-2xl border border-[#1f2937] bg-[#111317] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#fbbf24]">
                Android
              </p>
              <p className="mt-2 text-[15px] font-medium text-[#fafafa]">Open the app</p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#8c93a3]">
                App Links or Play Store fallback, with the same link staying shareable.
              </p>
            </div>

            <div className="rounded-2xl border border-[#1f2937] bg-[#111317] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#f472b6]">
                Web
              </p>
              <p className="mt-2 text-[15px] font-medium text-[#fafafa]">Show the landing page</p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#8c93a3]">
                Send desktop users to the web destination while keeping analytics and metadata.
              </p>
            </div>
          </div>
        </div>

        <div className="mt-5 grid gap-3 border-t border-[#1e1e22] pt-4 text-[13px] text-[#a1a1aa] md:grid-cols-3">
          <div className="rounded-xl bg-[#0b0d10] px-4 py-3">
            <strong className="text-[#fafafa]">Behind the scenes</strong>
            <p className="mt-1">Clicks and downstream attribution can be measured without changing the shared link.</p>
          </div>
          <div className="rounded-xl bg-[#0b0d10] px-4 py-3">
            <strong className="text-[#fafafa]">For your team</strong>
            <p className="mt-1">The CLI gets your account, domains, and first links into a working state quickly.</p>
          </div>
          <div className="rounded-xl bg-[#0b0d10] px-4 py-3">
            <strong className="text-[#fafafa]">For agents</strong>
            <p className="mt-1">The same link can return machine-readable metadata for tools and automations.</p>
          </div>
        </div>
      </div>
    </section>
  );
}
