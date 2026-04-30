"use client";

import { useEffect, useState } from "react";
import dynamic from "next/dynamic";

const WarpTunnel = dynamic(
  () => import("@/components/warp-tunnel").then((module) => module.WarpTunnel),
  { ssr: false }
);

export function DeferredWarpTunnel() {
  const [shouldMount, setShouldMount] = useState(false);
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    let idleHandle: number | undefined;
    let timeoutHandle: number | undefined;
    let frameHandle: number | undefined;

    const startMount = () => {
      setShouldMount(true);
      frameHandle = window.requestAnimationFrame(() => {
        frameHandle = window.requestAnimationFrame(() => setIsVisible(true));
      });
    };

    if (typeof window.requestIdleCallback === "function") {
      idleHandle = window.requestIdleCallback(() => startMount());
    } else {
      timeoutHandle = window.setTimeout(startMount, 180);
    }

    return () => {
      if (typeof idleHandle === "number" && typeof window.cancelIdleCallback === "function") {
        window.cancelIdleCallback(idleHandle);
      }
      if (typeof timeoutHandle === "number") {
        window.clearTimeout(timeoutHandle);
      }
      if (typeof frameHandle === "number") {
        window.cancelAnimationFrame(frameHandle);
      }
    };
  }, []);

  if (!shouldMount) return null;

  return (
    <div
      className="absolute inset-0 z-0 transition-opacity duration-700 ease-out"
      style={{ opacity: isVisible ? 1 : 0 }}
    >
      <WarpTunnel />
    </div>
  );
}
