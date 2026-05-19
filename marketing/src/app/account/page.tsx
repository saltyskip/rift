import type { Metadata } from "next";
import { AccountDashboard } from "@/components/account-dashboard";

export const metadata: Metadata = {
  title: "Account · Rift",
  description: "Manage your Rift API keys.",
};

export default function AccountPage() {
  return <AccountDashboard />;
}
