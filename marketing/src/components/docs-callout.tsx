export function DocsCallout({
  type,
  children,
}: {
  type: "info" | "warning";
  children: React.ReactNode;
}) {
  const styles = {
    info: "border-[#60a5fa]/30 bg-[#60a5fa]/5 text-[#93bbfd]",
    warning: "border-[#f59e0b]/30 bg-[#f59e0b]/5 text-[#fbbf24]",
  };
  const labels = { info: "Note", warning: "Important" };
  return (
    <div className={`rounded-lg border px-4 py-3 text-[13px] leading-relaxed ${styles[type]}`}>
      <strong>{labels[type]}:</strong> {children}
    </div>
  );
}

export function DocsCalloutCard({
  eyebrow,
  title,
  children,
}: {
  eyebrow: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-2xl border border-[#1f2937] bg-[#0f1115] p-5">
      <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
        {eyebrow}
      </p>
      <h3 className="mt-3 text-lg font-semibold text-[#fafafa]">{title}</h3>
      <div className="mt-2 space-y-3 text-[14px] leading-relaxed text-[#a1a1aa]">{children}</div>
    </div>
  );
}
