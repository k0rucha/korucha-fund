"use client";

import { useTheme, type Theme } from "./ThemeProvider";

const THEMES: { value: Theme; label: string }[] = [
  { value: "default", label: "Default" },
  { value: "win95", label: "Windows 95" },
];

export default function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  return (
    <select
      aria-label="テーマ"
      value={theme}
      onChange={(e) => setTheme(e.target.value as Theme)}
      className="border border-da-gray-400 bg-da-gray-50 px-2 py-1 text-sm text-da-gray-800"
    >
      {THEMES.map((t) => (
        <option key={t.value} value={t.value}>
          {t.label}
        </option>
      ))}
    </select>
  );
}
