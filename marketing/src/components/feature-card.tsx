"use client";

import { cn } from "@/lib/utils";
import { motion } from "motion/react";

interface FeatureCardProps {
  title: string;
  description: string;
  children?: React.ReactNode;
  className?: string;
  icon?: React.ReactNode;
  number?: string;
}

export function FeatureCard({
  title,
  description,
  children,
  className,
  icon,
  number,
}: FeatureCardProps) {
  return (
    <motion.div
      whileHover={{ y: -2 }}
      transition={{ type: "spring", stiffness: 500, damping: 30 }}
      className={cn(
        "group relative rounded-xl border border-border bg-card/60 p-7 transition-colors duration-500 hover:border-primary/20 hover:bg-card",
        className
      )}
    >
      <div className="absolute -inset-px rounded-xl bg-gradient-to-b from-primary/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none" />
      <div className="relative">
        <div className="flex items-start justify-between mb-5">
          {icon && (
            <div className="flex size-11 items-center justify-center rounded-lg bg-primary/8 ring-1 ring-primary/15 text-primary">
              {icon}
            </div>
          )}
          {number && (
            <span className="font-display text-[40px] leading-none text-muted-foreground/15 select-none">
              {number}
            </span>
          )}
        </div>
        <h3 className="text-[17px] font-semibold tracking-[-0.01em] mb-2">
          {title}
        </h3>
        <p className="text-sm text-muted-foreground leading-[1.7]">
          {description}
        </p>
        {children}
      </div>
    </motion.div>
  );
}
