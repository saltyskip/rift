export function DocsStep({
  n,
  title,
  id,
  children,
}: {
  n: number;
  title: string;
  id?: string;
  children: React.ReactNode;
}) {
  return (
    <div id={id} className="relative pl-10 scroll-mt-24">
      <div className="absolute left-0 top-0 flex h-7 w-7 items-center justify-center rounded-full border border-[#2dd4bf]/20 bg-[#2dd4bf]/10 text-sm font-semibold text-[#2dd4bf]">
        {n}
      </div>
      <h3 className="mb-3 text-lg font-semibold text-[#fafafa]">{title}</h3>
      <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">{children}</div>
    </div>
  );
}
