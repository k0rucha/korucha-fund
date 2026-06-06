"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
} from "react";

export type Theme = "default" | "win95";

interface ThemeContextValue {
  theme: Theme;
  setTheme: (t: Theme) => void;
}

const ThemeContext = createContext<ThemeContextValue>({
  theme: "default",
  setTheme: () => {},
});

export const useTheme = () => useContext(ThemeContext);

/**
 * Tracks the active theme and keeps <html data-theme> + localStorage in sync.
 * The initial data-theme is applied pre-paint by the inline script in
 * layout.tsx; this provider re-reads it on mount and drives later changes so
 * Chart.js components (which can't use CSS vars) can re-render on theme change.
 */
export default function ThemeProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const [theme, setThemeState] = useState<Theme>("default");

  useEffect(() => {
    const saved = (localStorage.getItem("korucha-theme") as Theme) ?? "default";
    const t: Theme = saved === "win95" ? "win95" : "default";
    setThemeState(t);
    document.documentElement.setAttribute("data-theme", t);
  }, []);

  const setTheme = useCallback((t: Theme) => {
    setThemeState(t);
    document.documentElement.setAttribute("data-theme", t);
    try {
      localStorage.setItem("korucha-theme", t);
    } catch {
      /* ignore storage failures (private mode etc.) */
    }
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}
