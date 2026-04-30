"use client";

import { useEffect, useState } from "react";

const HERO_PHRASES = ["Built for humans.", "Ready for agents."];

export function HeroTypewriter() {
  const [phraseIndex, setPhraseIndex] = useState(0);
  const [visibleText, setVisibleText] = useState("");
  const [isDeleting, setIsDeleting] = useState(false);

  useEffect(() => {
    const phrase = HERO_PHRASES[phraseIndex];
    const atFullPhrase = visibleText === phrase;
    const atEmptyPhrase = visibleText.length === 0;

    const timeout = window.setTimeout(() => {
      if (!isDeleting) {
        if (atFullPhrase) {
          setIsDeleting(true);
          return;
        }

        setVisibleText(phrase.slice(0, visibleText.length + 1));
        return;
      }

      if (atEmptyPhrase) {
        setIsDeleting(false);
        setPhraseIndex((current) => (current + 1) % HERO_PHRASES.length);
        return;
      }

      setVisibleText(phrase.slice(0, visibleText.length - 1));
    }, atFullPhrase ? 1400 : isDeleting ? 45 : 75);

    return () => window.clearTimeout(timeout);
  }, [isDeleting, phraseIndex, visibleText]);

  return (
    <span className="text-[#2dd4bf] inline-flex min-h-[1.2em] items-center">
      {visibleText}
      <span className="ml-1 cursor-blink text-[#5eead4]">|</span>
    </span>
  );
}
