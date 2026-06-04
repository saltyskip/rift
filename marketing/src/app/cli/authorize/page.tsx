import type { Metadata } from "next";
import { CliAuthorize } from "@/components/cli-authorize";

export const metadata: Metadata = {
  title: "Authorize the Rift CLI · Rift",
  description: "Approve a sign-in for the Rift command-line tool on this device.",
  robots: { index: false, follow: false },
};

export default function CliAuthorizePage() {
  return <CliAuthorize />;
}
