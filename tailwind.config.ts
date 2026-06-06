import type { Config } from "tailwindcss";

/**
 * Theme palette is defined as RGB-channel CSS variables in globals.css
 * (`:root` and `[data-theme="..."]`). Tailwind references them via
 * `rgb(var(--x) / <alpha-value>)`, so a single `data-theme` swap re-themes
 * every `*-da-*` utility at once. New theme = add a `[data-theme="..."]`
 * block in globals.css.
 */
const config: Config = {
  content: ["./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        "da-blue": {
          1200: "rgb(var(--da-blue-1200) / <alpha-value>)",
          900: "rgb(var(--da-blue-900) / <alpha-value>)",
          600: "rgb(var(--da-blue-600) / <alpha-value>)",
          50: "rgb(var(--da-blue-50) / <alpha-value>)",
        },
        "da-orange": {
          600: "rgb(var(--da-orange-600) / <alpha-value>)",
          400: "rgb(var(--da-orange-400) / <alpha-value>)",
          50: "rgb(var(--da-orange-50) / <alpha-value>)",
        },
        "da-gray": {
          800: "rgb(var(--da-gray-800) / <alpha-value>)",
          600: "rgb(var(--da-gray-600) / <alpha-value>)",
          400: "rgb(var(--da-gray-400) / <alpha-value>)",
          200: "rgb(var(--da-gray-200) / <alpha-value>)",
          50: "rgb(var(--da-gray-50) / <alpha-value>)",
        },
      },
      fontFamily: {
        sans: ["var(--font-noto-sans-jp)", "system-ui", "sans-serif"],
      },
    },
  },
  plugins: [],
};

export default config;
