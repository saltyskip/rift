"use client";

import { useState, useEffect } from "react";
import { motion } from "motion/react";
import Image from "next/image";

export function Navbar() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <motion.header
      initial={{ y: -20, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.5, ease: "easeOut" }}
      className="fixed top-0 z-50 w-full"
      style={{
        background: scrolled ? "rgba(9, 9, 11, 0.85)" : "transparent",
        backdropFilter: scrolled ? "blur(16px) saturate(1.5)" : "none",
        borderBottom: scrolled ? "1px solid #222225" : "1px solid transparent",
        transition: "all 0.3s",
      }}
    >
      <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-6">
        <a href="/" className="flex items-center gap-0.5">
          <Image
            src="/logo.svg"
            alt="Rift"
            width={34}
            height={34}
            className="invert brightness-0"
            style={{ filter: "invert(1) sepia(1) saturate(5) hue-rotate(140deg) brightness(0.85)" }}
          />
          <span className="text-base font-semibold tracking-tight">Rift</span>
        </a>

        <nav className="hidden md:flex items-center gap-8">
          {[
            ["How it works", "#how-it-works"],
            ["Pricing", "#pricing"],
            ["Docs", "/docs"],
            ["API Reference", "/api-reference"],
          ].map(([label, href]) => (
            <a
              key={label}
              href={href}
              className="text-[13px] text-[#71717a] hover:text-[#fafafa] transition-colors"
            >
              {label}
            </a>
          ))}
        </nav>

        <div className="flex items-center gap-3">
          <a href="/api-reference" className="text-[13px] text-[#71717a] hover:text-[#fafafa] transition-colors hidden sm:block">
            API Reference
          </a>
          <a
            href="#"
            className="text-[13px] font-medium bg-[#2dd4bf] text-[#042f2e] px-3.5 py-1.5 rounded-lg hover:bg-[#5eead4] transition-colors"
          >
            Get API Key
          </a>
        </div>
      </div>
    </motion.header>
  );
}
