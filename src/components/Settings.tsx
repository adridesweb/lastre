import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { LANGS, STRINGS, type Lang } from "../i18n";
import { openUrl } from "@tauri-apps/plugin-opener";
import { DEFAULT_PALETTE, PALETTES, PALETTES_SOURCE, type PaletteId } from "../palettes";

export interface AppSettings {
  lang: Lang;
  palette: PaletteId;
}

const STORAGE_KEY = "lastre.settings";

export function loadSettings(): AppSettings {
  const defaults: AppSettings = { lang: "es", palette: DEFAULT_PALETTE };
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}") as Partial<AppSettings>;
    return {
      lang: LANGS.some((l) => l.id === saved.lang) ? saved.lang! : defaults.lang,
      palette: PALETTES.some((p) => p.id === saved.palette) ? saved.palette! : defaults.palette,
    };
  } catch {
    return defaults;
  }
}

export function saveSettings(settings: AppSettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Sin almacenamiento disponible los ajustes valen solo para esta sesión.
  }
}

interface Props {
  current: AppSettings;
  onSave: (settings: AppSettings) => void;
  onClose: () => void;
}

export function SettingsDialog({ current, onSave, onClose }: Props) {
  const [draft, setDraft] = useState(current);
  // El diálogo se lee en el idioma que se está eligiendo.
  const t = STRINGS[draft.lang];
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion, () => {});
  }, []);

  return (
    <div className="backdrop" onMouseDown={onClose}>
      <form
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.key === "Escape" && onClose()}
        onSubmit={(e) => {
          e.preventDefault();
          onSave(draft);
        }}
      >
        <h3 id="settings-title">{t.settings}</h3>

        <fieldset className="setting">
          <legend>{t.language}</legend>
          <div className="lang-options">
            {LANGS.map((l) => (
              <label key={l.id} className={`option${draft.lang === l.id ? " chosen" : ""}`}>
                <input
                  type="radio"
                  name="lang"
                  checked={draft.lang === l.id}
                  onChange={() => setDraft({ ...draft, lang: l.id })}
                />
                {l.label}
              </label>
            ))}
          </div>
        </fieldset>

        <fieldset className="setting">
          <legend>{t.palette}</legend>
          <div className="palette-options">
            {PALETTES.map((p) => (
              <label key={p.id} className={`option palette${draft.palette === p.id ? " chosen" : ""}`}>
                <input
                  type="radio"
                  name="palette"
                  checked={draft.palette === p.id}
                  onChange={() => setDraft({ ...draft, palette: p.id })}
                />
                <span className="palette-sample" aria-hidden="true">
                  {p.swatches.map((color) => (
                    <span key={color} style={{ background: color }} />
                  ))}
                </span>
                {p.name}
              </label>
            ))}
          </div>
          <p className="muted small-text credit">
            {t.paletteCredit}{" "}
            <button type="button" className="link" onClick={() => void openUrl(PALETTES_SOURCE)}>
              coolors.co/palettes/trending
            </button>
          </p>
        </fieldset>

        <div className="dialog-actions">
          {version && <span className="muted small-text version">{t.version(version)}</span>}
          <button type="button" className="outline" onClick={onClose}>
            {t.cancel}
          </button>
          <button type="submit" className="solid">
            {t.saveSettings}
          </button>
        </div>
      </form>
    </div>
  );
}
