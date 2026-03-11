import { useEffect } from "react";
import type { ColorTheme } from "../types";

export function useTheme(colorTheme: ColorTheme) {
  useEffect(() => {
    function resolve(preference: ColorTheme, systemDark: boolean): "light" | "dark" {
      if (preference === "light") return "light";
      if (preference === "dark") return "dark";
      return systemDark ? "dark" : "light";
    }

    const mq = window.matchMedia("(prefers-color-scheme: dark)");

    function apply() {
      const resolved = resolve(colorTheme, mq.matches);
      document.documentElement.dataset.theme = resolved;
    }

    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [colorTheme]);
}
