import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Spill.",
  description: "A retro board for teams who like GIFs and do not want to overthink.",
  icons: {
    icon: "/brand/spillitout-logo-mark.svg",
    shortcut: "/brand/spillitout-logo-mark.svg",
    apple: "/brand/spillitout-logo-mark.svg",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="sp-paper">{children}</body>
    </html>
  );
}
