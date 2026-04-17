import Image from "next/image";

export function Footer() {
  return (
    <footer className="border-t border-[#222225]">
      <div className="mx-auto max-w-6xl px-6 py-12">
        <div className="flex flex-col sm:flex-row justify-between items-start gap-10">
          <div>
            <div className="flex items-center gap-0.5 mb-3">
              <Image
                src="/logo.svg"
                alt="Rift"
                width={28}
                height={28}
                className="invert brightness-0"
                style={{ filter: "invert(1) sepia(1) saturate(5) hue-rotate(140deg) brightness(0.85)" }}
              />
              <span className="text-sm font-medium">Rift</span>
            </div>
            <p className="text-[13px] text-[#52525b] max-w-[260px] leading-relaxed">
              Deep links for humans and agents.
              One URL, two audiences.
            </p>
          </div>

          <div className="flex gap-16">
            {Object.entries({
              Product: [
                ["Quick Setup", "/docs"],
                ["API Reference", "/api-reference"],
                ["Pricing", "/#pricing"],
              ],
              Resources: [
                ["Blog", "/blog"],
                ["OpenAPI Spec", "/api-reference"],
                ["Status", "#"],
              ],
              Legal: [
                ["Privacy", "#"],
                ["Terms", "#"],
              ],
            }).map(([heading, links]) => (
              <div key={heading}>
                <p className="text-[11px] font-medium text-[#52525b] uppercase tracking-widest mb-3">{heading}</p>
                <ul className="space-y-2">
                  {(links as string[][]).map(([label, href]) => (
                    <li key={label}>
                      <a href={href} className="text-[13px] text-[#71717a] hover:text-[#fafafa] transition-colors">{label}</a>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>
        <div className="gradient-line mt-10 mb-6" />
        <p className="text-[11px] text-[#3f3f46]">&copy; {new Date().getFullYear()} Rift</p>
      </div>
    </footer>
  );
}
