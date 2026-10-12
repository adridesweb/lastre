// Paletas tomadas de Coolors (https://coolors.co/palettes/trending).
// `swatches` son los colores originales; el resto de tonos de interfaz se derivan de ellos.

export type PaletteId = "deep-sea" | "white-greens" | "earthy-forest-hues" | "vibrant-sunset";

export const PALETTES_SOURCE = "https://coolors.co/palettes/trending";

export interface Palette {
  id: PaletteId;
  name: string;
  dark: boolean;
  /** Colores originales de la paleta en Coolors. */
  swatches: string[];
  /** Variables CSS de la interfaz. */
  vars: Record<string, string>;
  /** Colores por puesto en los rankings (7) y, al final, el del resto. */
  ranks: string[];
}

export const PALETTES: Palette[] = [
  {
    id: "deep-sea",
    name: "Deep Sea",
    dark: false,
    swatches: ["#0d1b2a", "#1b263b", "#415a77", "#778da9", "#e0e1dd"],
    vars: {
      "--bg": "#e0e1dd",
      "--side": "#d1d5d4",
      "--ink": "#0d1b2a",
      "--on-ink": "#e0e1dd",
      "--muted": "#415a77",
      "--line": "#cbcfce",
      "--line-strong": "#a9b4c2",
      "--accent": "#415a77",
      "--accent-soft": "#d0d7df",
      "--trash": "#1b263b",
      "--on-trash": "#e0e1dd",
      "--trash-soft": "#c9d1db",
      "--hover": "#d7d9d6",
      "--side-hover": "#c2c8c9",
      "--track": "#b4bcc6",
    },
    ranks: ["#0d1b2a", "#415a77", "#778da9", "#1b263b", "#a3b3c6", "#5c748f", "#c0cad6", "#cdd1d2"],
  },
  {
    // Blanco con toques verdes; el blanco y los neutros se añaden a los verdes de la paleta.
    id: "white-greens",
    name: "White & Greens",
    dark: false,
    swatches: ["#004b23", "#006400", "#007200", "#008000", "#38b000", "#70e000", "#9ef01a", "#ccff33"],
    vars: {
      "--bg": "#ffffff",
      "--side": "#f4f6f3",
      "--ink": "#0f1a14",
      "--on-ink": "#ffffff",
      "--muted": "#4f5b54",
      "--line": "#e6eae5",
      "--line-strong": "#c5cdc6",
      "--accent": "#007200",
      "--accent-soft": "#ecfbd9",
      "--trash": "#38b000",
      "--on-trash": "#001a0c",
      // Los verdes claros no se leen como texto sobre blanco.
      "--trash-ink": "#006400",
      "--trash-soft": "#f0fbdc",
      "--hover": "#f7f9f6",
      "--side-hover": "#e6ebe4",
      "--track": "#d5dcd4",
    },
    ranks: ["#004b23", "#38b000", "#008000", "#9ef01a", "#006400", "#70e000", "#ccff33", "#e6eae5"],
  },
  {
    id: "earthy-forest-hues",
    name: "Earthy Forest Hues",
    dark: false,
    swatches: ["#dad7cd", "#a3b18a", "#588157", "#3a5a40", "#344e41"],
    vars: {
      "--bg": "#dad7cd",
      "--side": "#cccabd",
      "--ink": "#344e41",
      "--on-ink": "#dad7cd",
      "--muted": "#3a5a40",
      "--line": "#c8c6b8",
      "--line-strong": "#a3b18a",
      "--accent": "#3a5a40",
      "--accent-soft": "#c9d1bc",
      "--trash": "#344e41",
      "--on-trash": "#dad7cd",
      "--trash-soft": "#c5ceb6",
      "--hover": "#d2cfc4",
      "--side-hover": "#bdbcae",
      "--track": "#b4b8a6",
    },
    ranks: ["#344e41", "#588157", "#a3b18a", "#3a5a40", "#7d9a72", "#c0c9ae", "#8fa389", "#cbc9bc"],
  },
  {
    // Versión calmada: fondo lavanda claro derivado de los morados, tinta morada
    // y el naranja solo como detalle (Papelera y algún puesto del ranking).
    id: "vibrant-sunset",
    name: "Vibrant Sunset",
    dark: false,
    swatches: [
      "#ff6d00",
      "#ff7900",
      "#ff8500",
      "#ff9100",
      "#ff9e00",
      "#240046",
      "#3c096c",
      "#5a189a",
      "#7b2cbf",
      "#9d4edd",
    ],
    vars: {
      "--bg": "#f1ecf7",
      "--side": "#e4dbef",
      "--ink": "#3c096c",
      "--on-ink": "#f1ecf7",
      "--muted": "#5e3f86",
      "--line": "#ddd2ea",
      "--line-strong": "#b9a3d6",
      "--accent": "#7b2cbf",
      "--accent-soft": "#e3d5f5",
      // #ff6d00 oscurecido para que se lea como texto y con texto blanco encima.
      "--trash": "#c75400",
      "--on-trash": "#ffffff",
      "--trash-soft": "#ffe9d6",
      "--hover": "#eae3f3",
      "--side-hover": "#d5c8e6",
      "--track": "#c9b8df",
    },
    ranks: ["#3c096c", "#7b2cbf", "#9d4edd", "#ff9100", "#5a189a", "#c3a0ea", "#ffc27a", "#ddd2ea"],
  },
];

export const DEFAULT_PALETTE: PaletteId = "deep-sea";

export function applyPalette(id: PaletteId) {
  const palette = PALETTES.find((p) => p.id === id) ?? PALETTES[0];
  const style = document.documentElement.style;
  for (const [name, value] of Object.entries(palette.vars)) style.setProperty(name, value);
  // Color de la Papelera cuando se usa como texto; por defecto, el mismo del botón.
  style.setProperty("--trash-ink", palette.vars["--trash-ink"] ?? palette.vars["--trash"]);
  palette.ranks.forEach((color, i) => style.setProperty(`--rank-${i}`, color));
  style.setProperty("color-scheme", palette.dark ? "dark" : "light");
}

/** Color CSS para el puesto `rank` de un ranking; del octavo en adelante, el del resto. */
export function rankColor(rank: number): string {
  return `var(--rank-${Math.min(rank, 7)})`;
}
