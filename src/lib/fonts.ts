import localFont from "next/font/local";

/**
 * Noto Sans JP, self-hosted from the weights archived in `assets/fonts/`.
 * Local (not next/font/google) because the UI is Japanese-heavy and we want
 * full CJK glyph coverage without a build-time network fetch. Exposed as the
 * `--font-noto-sans-jp` CSS variable consumed by tailwind.config.ts.
 */
export const notoSansJP = localFont({
  src: [
    { path: "../../assets/fonts/NotoSansJP-Regular.ttf", weight: "400", style: "normal" },
    { path: "../../assets/fonts/NotoSansJP-Medium.ttf", weight: "500", style: "normal" },
    { path: "../../assets/fonts/NotoSansJP-Bold.ttf", weight: "700", style: "normal" },
    { path: "../../assets/fonts/NotoSansJP-Black.ttf", weight: "900", style: "normal" },
  ],
  variable: "--font-noto-sans-jp",
  display: "swap",
});
