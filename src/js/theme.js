// Theme management: light / dark / system. Applies a `data-theme` attribute on
// <html> and reacts to OS preference changes in "system" mode.

const mql = window.matchMedia("(prefers-color-scheme: dark)");

export const theme = {
  mode: "system",

  setMode(mode) {
    this.mode = mode === "light" || mode === "dark" ? mode : "system";
    this.apply();
  },

  apply() {
    const dark = this.mode === "dark" || (this.mode === "system" && mql.matches);
    document.documentElement.setAttribute("data-theme", dark ? "dark" : "light");
  },

  listen() {
    mql.addEventListener("change", () => {
      if (this.mode === "system") this.apply();
    });
  },
};
