"use client";

import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { cn } from "@/lib/utils";

interface RevealOnViewProps {
  children: ReactNode;
  className?: string;
  delay?: number;
}

export function RevealOnView({
  children,
  className,
  delay = 0,
}: RevealOnViewProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    const node = ref.current;
    if (!node) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry?.isIntersecting) return;
        setIsVisible(true);
        observer.disconnect();
      },
      { threshold: 0.15, rootMargin: "0px 0px -10% 0px" }
    );

    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      ref={ref}
      className={cn("reveal", isVisible && "reveal-visible", className)}
      style={{ animationDelay: `${delay}s` } as CSSProperties}
    >
      {children}
    </div>
  );
}
