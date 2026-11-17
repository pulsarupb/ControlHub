// Panels colour themes. A theme only redefines the base palette tokens in routes/global.css
// (backgrounds, borders, text, primary and accent); everything else reads those tokens.
export const THEMES = [
  { id: "graphite", name: "Graphite", note: "Neutral dark, amber primary (default)" },
  { id: "midnight", name: "Midnight", note: "Deep navy, blue primary" },
  { id: "forest", name: "Forest", note: "Dark green, gold primary" },
  { id: "oxblood", name: "Oxblood", note: "Warm maroon, amber primary" },
  { id: "violet", name: "Violet", note: "Plum dark, lavender primary" },
  { id: "ocean", name: "Ocean", note: "Deep teal, aqua primary" },
  { id: "copper", name: "Copper", note: "Warm brown, copper primary" },
  { id: "contrast", name: "High contrast", note: "Black and white, maximum legibility" },
] as const

export type ThemeId = (typeof THEMES)[number]["id"]

const KEY = "panels-theme"
const DEFAULT: ThemeId = "graphite"

function isTheme(value: string | null): value is ThemeId {
  return THEMES.some(t => t.id === value)
}

function readStored(): ThemeId {
  try {
    const value = typeof localStorage === "undefined" ? null : localStorage.getItem(KEY)
    return isTheme(value) ? value : DEFAULT
  } catch {
    return DEFAULT
  }
}

class ThemeState {
  current = $state<ThemeId>(DEFAULT)

  constructor() {
    this.apply(readStored())
  }

  apply(id: ThemeId) {
    this.current = id
    if (typeof document !== "undefined") document.documentElement.dataset.theme = id
    try {
      if (typeof localStorage !== "undefined") localStorage.setItem(KEY, id)
    } catch {
      // Storage can be blocked; the theme still applies for this visit.
    }
  }
}

export const theme = new ThemeState()
