import type { Metadata } from "next";
import { notoSansJP } from "@/lib/fonts";
import ThemeProvider from "@/components/ThemeProvider";
import "./globals.css";

export const metadata: Metadata = {
  // Makes relative og:url / og:image absolute for link-preview crawlers.
  metadataBase: new URL(process.env.SITE_URL ?? "https://fund.korucha.com"),
  title: "こるちゃファンド",
  description: "個人投資ポートフォリオ・トラッカー",
  icons: { icon: "/favicon.ico" },
};

// Apply the saved theme before first paint to avoid a flash of the wrong
// palette. Mirrors the early-script approach from the old Askama base.html.
const themeBootstrap = `(function(){var t='default';try{t=localStorage.getItem('korucha-theme')||'default';}catch(e){}if(t!=='default'&&t!=='win95')t='default';document.documentElement.setAttribute('data-theme',t);})();`;

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="ja" className={notoSansJP.variable}>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeBootstrap }} />
      </head>
      <body className="font-sans antialiased">
        <ThemeProvider>{children}</ThemeProvider>
      </body>
    </html>
  );
}
