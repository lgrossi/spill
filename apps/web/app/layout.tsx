import type { Metadata } from "next";
import "./globals.css";
import { readTheme } from "./lib/theme";

export const metadata: Metadata = {
  title: "Spill.",
  description: "A board-first retro app for teams who like GIFs and do not want to overthink.",
  icons: {
    icon: "/brand/spill-mug-icon.svg",
    shortcut: "/brand/spill-mug-icon.svg",
    apple: "/brand/spill-mug-icon.svg",
  },
};

export default async function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const theme = await readTheme();
  return (
    <html lang="en" data-theme={theme}>
      <body className="sp-paper">{children}</body>
    </html>
  );
}
