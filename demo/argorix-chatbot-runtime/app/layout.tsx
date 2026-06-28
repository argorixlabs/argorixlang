import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Argorix Chatbot Runtime Demo",
  description:
    "A chatbot governed by Argorix Lang v1.0 — contracts, policy, evidence and fail-closed enforcement.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
