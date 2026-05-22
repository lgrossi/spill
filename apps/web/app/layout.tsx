import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "SpillItOut",
  description: "Board-first team retros with GIFs, actions, and optional AI.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
