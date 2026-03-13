import { cn } from "@/lib/utils";
import { FadeIn } from "@/components/motion-wrapper";

interface SectionProps {
  children: React.ReactNode;
  className?: string;
  title?: string;
  titleAccent?: string;
  subtitle?: string;
  id?: string;
  align?: "center" | "left";
}

export function Section({
  children,
  className,
  title,
  titleAccent,
  subtitle,
  id,
  align = "center",
}: SectionProps) {
  return (
    <section
      id={id}
      className={cn("relative py-28 px-6", className)}
    >
      <div className="mx-auto max-w-7xl">
        {(title || subtitle) && (
          <FadeIn
            className={cn(
              "mb-16",
              align === "center" ? "text-center" : "max-w-3xl"
            )}
          >
            {title && (
              <h2
                className={cn(
                  "font-display text-[clamp(2rem,4vw,3.25rem)] leading-[1.1] tracking-[-0.02em]",
                  align === "center" && "mx-auto max-w-4xl"
                )}
              >
                {title}
                {titleAccent && (
                  <span className="text-gradient-primary"> {titleAccent}</span>
                )}
              </h2>
            )}
            {subtitle && (
              <p
                className={cn(
                  "mt-5 text-[17px] text-muted-foreground leading-relaxed",
                  align === "center" && "mx-auto max-w-2xl"
                )}
              >
                {subtitle}
              </p>
            )}
          </FadeIn>
        )}
        {children}
      </div>
    </section>
  );
}
