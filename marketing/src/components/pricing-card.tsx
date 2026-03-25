"use client";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { motion } from "motion/react";

interface PricingCardProps {
  name: string;
  price: string;
  period?: string;
  description: string;
  features: string[];
  cta: string;
  highlighted?: boolean;
}

export function PricingCard({
  name,
  price,
  period,
  description,
  features,
  cta,
  highlighted = false,
}: PricingCardProps) {
  return (
    <motion.div
      whileHover={{ y: -3 }}
      transition={{ type: "spring", stiffness: 400, damping: 25 }}
      className={cn(
        "relative flex flex-col rounded-xl border p-7",
        highlighted
          ? "border-primary/30 bg-primary/[0.03] glow-primary"
          : "border-border bg-card/40 hover:border-primary/15 hover:bg-card/60"
      )}
    >
      {highlighted && (
        <div className="absolute -top-3 left-7">
          <span className="rounded-full bg-primary px-3 py-1 text-[11px] font-semibold text-primary-foreground tracking-wide">
            Recommended
          </span>
        </div>
      )}
      <div className="mb-8">
        <p className="text-xs text-muted-foreground uppercase tracking-widest mb-3">
          {name}
        </p>
        <div className="flex items-baseline gap-1">
          <span className="font-display text-4xl tracking-[-0.02em]">
            {price}
          </span>
          {period && (
            <span className="text-sm text-muted-foreground">{period}</span>
          )}
        </div>
        <p className="text-sm text-muted-foreground mt-2">{description}</p>
      </div>
      <div className="h-px bg-border mb-6" />
      <ul className="space-y-3 flex-1 mb-8">
        {features.map((feature) => (
          <li key={feature} className="flex items-start gap-2.5 text-[13px]">
            <div className="size-1 rounded-full bg-primary mt-2 shrink-0" />
            <span className="text-muted-foreground leading-relaxed">
              {feature}
            </span>
          </li>
        ))}
      </ul>
      <Button
        variant={highlighted ? "default" : "outline"}
        className={cn("w-full", highlighted && "glow-primary")}
      >
        {cta}
      </Button>
    </motion.div>
  );
}
