import { createContext, useContext } from "react";

export type Lang = "es" | "en";

export const LANGS: { id: Lang; label: string }[] = [
  { id: "es", label: "Español" },
  { id: "en", label: "English" },
];

const es = {
  navSummary: "Resumen",
  navFolders: "Carpetas",
  navTypes: "Tipos de archivo",
  navBigFiles: "Archivos grandes",
  navAria: "Secciones",

  settings: "Ajustes",
  language: "Idioma",
  palette: "Paleta de colores",
  paletteCredit: "Paletas de Coolors:",
  saveSettings: "Guardar ajustes",
  version: (v: string) => `Lastre ${v}`,
  cancel: "Cancelar",

  driveUnit: (drive: string) => `Unidad ${drive}`,
  driveFree: (free: string) => `${free} libres`,
  driveUsage: (used: string, free: string) => `${used} ocupados · ${free} libres`,
  measureOther: "Medir otra unidad…",
  measuring: (files: string) => `Midiendo… ${files} archivos`,
  measureCancelled: "Medición cancelada.",
  startTitle: "¿Qué quieres medir?",
  orFolder: "O una carpeta concreta",
  choose: "Elegir…",
  chooseTitle: "Elegir carpeta para medir",
  measure: "Medir",
  backTo: (path: string) => `Volver a ${path}`,

  carries: (name: string) => `${name} carga con`,
  measureAgain: "Medir otra vez",
  files: (n: string) => `${n} archivos`,
  denied: (n: string) => `${n} carpetas sin acceso`,
  deniedTip: "Ejecuta Lastre como administrador para incluirlas.",
  whatTakesMost: "Qué ocupa más",
  seeAll: (n: string) => `Ver los ${n} elementos`,
  shareAria: "Reparto del espacio entre los elementos que más ocupan",
  rest: "Resto",

  open: "Abrir",
  toBasket: "A la cesta",
  inBasket: "En la cesta",

  pathAria: "Ruta",
  emptyFolder: "Esta carpeta está vacía.",
  remaining: (n: string) => `Quedan ${n} elementos más pequeños.`,
  showMore: "Mostrar más",

  bySpace: "Por espacio ocupado",
  noExtension: "Sin extensión",
  seeFiles: "Ver archivos",
  topFiles: (n: number) => `Los ${n} que más ocupan`,
  onlyType: (type: string) => `Solo ${type} ✕`,
  clearTypeAria: "Quitar filtro de tipo",

  basketTitle: "Cesta de limpieza",
  basketHint: "Ve soltando aquí lo que sobra y mándalo todo a la Papelera de una vez.",
  dropHere: "Arrastra aquí carpetas o archivos",
  youRecover: "Recuperarás",
  basketEmpty: "La cesta está vacía",
  sending: "Enviando…",
  items: (n: number, formatted: string) => `${formatted} ${n === 1 ? "elemento" : "elementos"}`,
  sendItems: (items: string) => `Enviar ${items} a la Papelera`,
  nothingLost: "Nada se borra del todo: podrás restaurarlo desde la Papelera.",
  openBin: "Abrir papelera de reciclaje",
  removeFromBasketAria: (name: string) => `Quitar ${name} de la cesta`,
  cannotBasketRoot: "No se puede echar a la cesta la carpeta que estás midiendo.",
  alreadyIncluded: (name: string, container: string) =>
    `«${name}» ya va incluido en «${container}», que está en la cesta.`,
  sentToBin: (items: string, size: string) => `Enviado a la Papelera: ${items} (${size}).`,
  sendFailed: (n: number, details: string) => `No se pudo con ${n}: ${details}`,

  reveal: "Mostrar en el Explorador",
  copyPath: "Copiar ruta",
  addToBasket: "Echar a la cesta",
  removeFromBasket: "Quitar de la cesta",

  confirmTitle: (items: string) => `¿Enviar ${items} a la Papelera?`,
  confirmBody: (size: string) =>
    `Liberarás ${size} cuando vacíes la Papelera. Hasta entonces podrás restaurarlo.`,
  confirmSend: "Enviar a la Papelera",
};

export type Strings = typeof es;

const en: Strings = {
  navSummary: "Summary",
  navFolders: "Folders",
  navTypes: "File types",
  navBigFiles: "Large files",
  navAria: "Sections",

  settings: "Settings",
  language: "Language",
  palette: "Color palette",
  paletteCredit: "Palettes from Coolors:",
  saveSettings: "Save settings",
  version: (v) => `Lastre ${v}`,
  cancel: "Cancel",

  driveUnit: (drive) => `Drive ${drive}`,
  driveFree: (free) => `${free} free`,
  driveUsage: (used, free) => `${used} used · ${free} free`,
  measureOther: "Measure another drive…",
  measuring: (files) => `Measuring… ${files} files`,
  measureCancelled: "Measurement cancelled.",
  startTitle: "What do you want to measure?",
  orFolder: "Or a specific folder",
  choose: "Choose…",
  chooseTitle: "Choose a folder to measure",
  measure: "Measure",
  backTo: (path) => `Back to ${path}`,

  carries: (name) => `${name} is carrying`,
  measureAgain: "Measure again",
  files: (n) => `${n} files`,
  denied: (n) => `${n} folders without access`,
  deniedTip: "Run Lastre as administrator to include them.",
  whatTakesMost: "What takes up the most",
  seeAll: (n) => `See all ${n} items`,
  shareAria: "How space is split among the largest items",
  rest: "Rest",

  open: "Open",
  toBasket: "Add to basket",
  inBasket: "In basket",

  pathAria: "Path",
  emptyFolder: "This folder is empty.",
  remaining: (n) => `${n} smaller items left.`,
  showMore: "Show more",

  bySpace: "By space used",
  noExtension: "No extension",
  seeFiles: "See files",
  topFiles: (n) => `The ${n} largest`,
  onlyType: (type) => `Only ${type} ✕`,
  clearTypeAria: "Clear type filter",

  basketTitle: "Cleanup basket",
  basketHint: "Drop what you don't need here and send it all to the Recycle Bin at once.",
  dropHere: "Drag folders or files here",
  youRecover: "You'll recover",
  basketEmpty: "The basket is empty",
  sending: "Sending…",
  items: (n, formatted) => `${formatted} ${n === 1 ? "item" : "items"}`,
  sendItems: (items) => `Send ${items} to the Recycle Bin`,
  nothingLost: "Nothing is deleted for good: you can restore it from the Recycle Bin.",
  openBin: "Open Recycle Bin",
  removeFromBasketAria: (name) => `Remove ${name} from the basket`,
  cannotBasketRoot: "The folder you are measuring can't go in the basket.",
  alreadyIncluded: (name, container) =>
    `"${name}" is already included in "${container}", which is in the basket.`,
  sentToBin: (items, size) => `Sent to the Recycle Bin: ${items} (${size}).`,
  sendFailed: (n, details) => `${n} failed: ${details}`,

  reveal: "Show in Explorer",
  copyPath: "Copy path",
  addToBasket: "Add to basket",
  removeFromBasket: "Remove from basket",

  confirmTitle: (items) => `Send ${items} to the Recycle Bin?`,
  confirmBody: (size) =>
    `You'll free ${size} once you empty the Recycle Bin. Until then you can restore it.`,
  confirmSend: "Send to Recycle Bin",
};

export const STRINGS: Record<Lang, Strings> = { es, en };

export const LangContext = createContext<Strings>(es);

export function useT(): Strings {
  return useContext(LangContext);
}
