import { cn } from "@/lib/utils";

interface LogoProps {
  className?: string;
  size?: number;
}

export function Logo({ className, size = 22 }: LogoProps) {
  return (
    <span className={cn("inline-flex items-center gap-1.5", className)}>
      <span
        className="flex items-center justify-center rounded-lg bg-primary text-primary-foreground font-bold text-[11px]"
        style={{ width: size, height: size }}
      >
        R
      </span>
      <span className="text-[15px] font-semibold tracking-[-0.02em]">
        Rift
      </span>
    </span>
  );
}
