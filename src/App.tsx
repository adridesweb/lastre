import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  api,
  type Drive,
  type FileType,
  type NodeDto,
  type NodeInfo,
  type ScanDone,
  type ScanProgress,
} from "./api";
import type { ItemActions } from "./components/ItemRow";
import { BigFiles, Folders, Summary, Types } from "./components/Sections";
import { loadSettings, saveSettings, SettingsDialog, type AppSettings } from "./components/Settings";
import { formatCount, formatSize, setFormatLang } from "./format";
import { LangContext, STRINGS, type Strings } from "./i18n";
import { applyPalette } from "./palettes";
import "./App.css";

interface Scan {
  root: NodeDto;
  path: string;
  denied: number;
}

/** Elemento de la cesta; `caution` marca los que pueden afectar a programas instalados. */
type BasketItem = NodeInfo & { caution: boolean };

/** Traduce el código con el que el backend se niega a enviar algo a la Papelera. */
function describeTrashError(code: string, name: string, t: Strings): string {
  const [kind, ...args] = code.split(":");
  const bytes = (i: number) => Number(args[i]) || 0;
  switch (kind) {
    case "protected":
      return t.errProtected(name);
    case "no-bin":
      return t.errNoBin(name);
    case "bin-disabled":
      return t.errBinDisabled(name);
    case "too-big":
      return t.errTooBig(name, formatSize(bytes(0)));
    case "bin-full":
      return t.errBinFull(name, formatSize(Math.max(0, bytes(0) - bytes(1))));
    default:
      return `${name}: ${code}`;
  }
}

type Section = "summary" | "folders" | "types" | "bigFiles";

const SECTIONS: { id: Section; label: (t: Strings) => string }[] = [
  { id: "summary", label: (t) => t.navSummary },
  { id: "folders", label: (t) => t.navFolders },
  { id: "types", label: (t) => t.navTypes },
  { id: "bigFiles", label: (t) => t.navBigFiles },
];

// Los ajustes guardados se aplican antes del primer pintado.
const initialSettings = loadSettings();
applyPalette(initialSettings.palette);
setFormatLang(initialSettings.lang);

export default function App() {
  const [settings, setSettings] = useState<AppSettings>(initialSettings);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [drives, setDrives] = useState<Drive[]>([]);
  const [path, setPath] = useState("");
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scan, setScan] = useState<Scan | null>(null);
  const [choosing, setChoosing] = useState(false);
  const [section, setSection] = useState<Section>("summary");
  const [folder, setFolder] = useState<NodeInfo | null>(null);
  const [typeFilter, setTypeFilter] = useState<FileType | null>(null);
  const [basket, setBasket] = useState<BasketItem[]>([]);
  const [version, setVersion] = useState(0);
  const [menu, setMenu] = useState<{ info: NodeInfo; x: number; y: number } | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [emptying, setEmptying] = useState(false);
  const [dropping, setDropping] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const t = STRINGS[settings.lang];
  const tRef = useRef(t);
  tRef.current = t;
  const basketRef = useRef(basket);
  basketRef.current = basket;

  const applySettings = (next: AppSettings) => {
    applyPalette(next.palette);
    setFormatLang(next.lang);
    saveSettings(next);
    setSettings(next);
    setSettingsOpen(false);
  };

  const refreshDrives = useCallback(async () => {
    const list = await api.listDrives();
    setDrives(list);
    return list;
  }, []);

  useEffect(() => {
    refreshDrives().then((list) => setPath((p) => p || list[0]?.path || ""), () => {});
  }, [refreshDrives]);

  useEffect(() => {
    const onDone = async (done: ScanDone) => {
      setScanning(false);
      setProgress(null);
      if (done.cancelled || !done.root) {
        setNotice(tRef.current.measureCancelled);
        return;
      }
      setFolder(await api.nodeInfo(0));
      setBasket([]);
      setTypeFilter(null);
      setSection("summary");
      setChoosing(false);
      setScan({ root: done.root, path: done.path, denied: done.denied });
      setVersion((v) => v + 1);
      void refreshDrives();
    };
    const unlisten = [
      listen<ScanProgress>("scan-progress", (e) => setProgress(e.payload)),
      listen<ScanDone>("scan-done", (e) => void onDone(e.payload)),
    ];
    return () => unlisten.forEach((p) => void p.then((off) => off()));
  }, [refreshDrives]);

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("mousedown", close);
    window.addEventListener("blur", close);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("blur", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [menu]);

  const startScan = async (target = path) => {
    if (!target.trim() || scanning) return;
    setNotice(null);
    try {
      await api.startScan(target.trim());
    } catch (err) {
      setNotice(String(err));
      return;
    }
    // El árbol anterior ya no existe en el backend.
    setScan(null);
    setBasket([]);
    setProgress({ files: 0, bytes: 0, current: target });
    setScanning(true);
  };

  // Carpeta pasada por línea de comandos: se mide nada más abrir.
  const autoStarted = useRef(false);
  useEffect(() => {
    if (autoStarted.current) return;
    autoStarted.current = true;
    api.initialPath().then((initial) => {
      if (!initial) return;
      setPath(initial);
      void startScan(initial);
    }, () => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const pickFolder = async () => {
    const dir = await open({ directory: true, title: t.chooseTitle });
    if (typeof dir === "string") setPath(dir);
  };

  const openFolder = async (id: number) => {
    const info = await api.nodeInfo(id);
    if (!info) return;
    setFolder(info);
    setSection("folders");
  };

  const addToBasket = async (info: NodeInfo) => {
    if (info.id === 0) {
      setNotice(t.cannotBasketRoot);
      return;
    }
    if (basketRef.current.some((b) => b.id === info.id)) return;
    // Lo que Windows borraría para siempre, o de lo que depende el sistema, no entra en la cesta.
    const check = await api.trashCheck(info.id);
    if (check.blocked) {
      setNotice(describeTrashError(check.blocked, info.name, t));
      return;
    }
    const current = basketRef.current;
    if (current.some((b) => b.id === info.id)) return;
    const container = current.find((b) => info.ancestors.includes(b.id));
    if (container) {
      setNotice(t.alreadyIncluded(info.name, container.name));
      return;
    }
    // Lo que ya estaba dentro de esta carpeta queda cubierto por ella.
    setBasket([...current.filter((b) => !b.ancestors.includes(info.id)), { ...info, caution: check.caution }]);
    setNotice(null);
  };

  const toggleBasket = async (id: number) => {
    if (basketRef.current.some((b) => b.id === id)) {
      setBasket((prev) => prev.filter((b) => b.id !== id));
      return;
    }
    const info = await api.nodeInfo(id);
    if (info) await addToBasket(info);
  };

  const emptyBasket = async () => {
    setConfirming(false);
    setEmptying(true);
    const failed: BasketItem[] = [];
    const errors: string[] = [];
    const sent: BasketItem[] = [];
    for (const item of basketRef.current) {
      try {
        await api.trash(item.id);
        sent.push(item);
      } catch (err) {
        failed.push(item);
        errors.push(describeTrashError(String(err), item.name, t));
      }
    }
    setBasket(failed);
    setEmptying(false);

    if (sent.length > 0) {
      const root = await api.getNode(0);
      setScan((s) => s && { ...s, root });
      // Si la carpeta abierta ha desaparecido, subimos a la que contenía lo eliminado.
      if (folder) {
        const gone = sent.find((s) => folder.ancestors.includes(s.id));
        const target = gone ? gone.ancestors[gone.ancestors.length - 2] : folder.id;
        setFolder(await api.nodeInfo(target));
      }
      setVersion((v) => v + 1);
      void refreshDrives();
    }
    const messages: string[] = [];
    if (sent.length > 0) {
      const freed = formatSize(sent.reduce((sum, s) => sum + s.size, 0));
      messages.push(t.sentToBin(t.items(sent.length, formatCount(sent.length)), freed));
    }
    if (errors.length > 0) messages.push(...errors);
    setNotice(messages.join(" "));
  };

  const basketIds = useMemo(() => new Set(basket.map((b) => b.id)), [basket]);
  const basketSize = basket.reduce((sum, b) => sum + b.size, 0);
  const itemsLabel = t.items(basket.length, formatCount(basket.length));

  const actions: ItemActions = {
    basketIds,
    toggleBasket: (id) => void toggleBasket(id),
    openFolder: (id) => void openFolder(id),
    openMenu: (id, x, y) => void api.nodeInfo(id).then((info) => info && setMenu({ info, x, y })),
  };

  const drive = drives.find((d) => (scan?.path ?? path).toUpperCase().startsWith(d.path.toUpperCase()));
  const showStart = !scan || choosing;
  const usedPercent = (d: Drive) => (d.total ? ((d.total - d.free) / d.total) * 100 : 0);

  return (
    <LangContext.Provider value={t}>
      <div className="app" lang={settings.lang} onContextMenu={(e) => e.preventDefault()}>
        <nav className="sidebar" aria-label={t.navAria}>
          <div className="brand-row">
            <div className="brand">Lastre</div>
            <button className="icon" aria-label={t.settings} title={t.settings} onClick={() => setSettingsOpen(true)}>
              <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h0a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h0a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v0a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </button>
          </div>
          {SECTIONS.map((s) => (
            <button
              key={s.id}
              className={`nav-item${!showStart && section === s.id ? " active" : ""}`}
              aria-current={!showStart && section === s.id ? "page" : undefined}
              disabled={!scan || scanning}
              onClick={() => {
                setSection(s.id);
                setChoosing(false);
              }}
            >
              {s.label(t)}
            </button>
          ))}

          {drive && (
            <div className="drive-meter">
              <span>{t.driveUnit(drive.path.slice(0, 2))}</span>
              <span className="meter">
                <span style={{ width: `${usedPercent(drive)}%` }} />
              </span>
              <span>{t.driveFree(formatSize(drive.free))}</span>
            </div>
          )}
          {scan && !scanning && (
            <button className="small" onClick={() => setChoosing(true)}>
              {t.measureOther}
            </button>
          )}
        </nav>

        <main className="content">
          {scanning && progress ? (
            <div className="center">
              <div className="spinner" />
              <h1 className="big-number">{formatSize(progress.bytes)}</h1>
              <p className="muted">{t.measuring(formatCount(progress.files))}</p>
              <p className="muted scan-path">{progress.current}</p>
              <button className="outline" onClick={() => void api.cancelScan()}>
                {t.cancel}
              </button>
            </div>
          ) : showStart ? (
            <div className="center">
              <h1 className="start-title">{t.startTitle}</h1>
              <div className="drive-cards">
                {drives.map((d) => (
                  <button
                    key={d.path}
                    className="drive-card"
                    onClick={() => {
                      setPath(d.path);
                      void startScan(d.path);
                    }}
                  >
                    <strong>{d.path}</strong>
                    <span className="meter">
                      <span style={{ width: `${usedPercent(d)}%` }} />
                    </span>
                    <span>{t.driveUsage(formatSize(d.total - d.free), formatSize(d.free))}</span>
                  </button>
                ))}
              </div>
              <div className="path-row">
                <label htmlFor="path">{t.orFolder}</label>
                <input
                  id="path"
                  value={path}
                  spellCheck={false}
                  onChange={(e) => setPath(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && void startScan()}
                />
                <button className="outline" onClick={() => void pickFolder()}>
                  {t.choose}
                </button>
                <button className="solid" disabled={!path.trim()} onClick={() => void startScan()}>
                  {t.measure}
                </button>
              </div>
              {scan && (
                <button className="link" onClick={() => setChoosing(false)}>
                  {t.backTo(scan.path)}
                </button>
              )}
            </div>
          ) : (
            scan && (
              <>
                {section === "summary" && (
                  <Summary
                    root={scan.root}
                    denied={scan.denied}
                    version={version}
                    actions={actions}
                    onRescan={() => void startScan(scan.path)}
                    onSeeAll={() => void openFolder(0)}
                  />
                )}
                {section === "folders" && folder && (
                  <Folders folder={folder} rootPath={scan.path} version={version} actions={actions} />
                )}
                {section === "types" && (
                  <Types
                    total={scan.root.size}
                    version={version}
                    onPick={(type) => {
                      setTypeFilter(type);
                      setSection("bigFiles");
                    }}
                  />
                )}
                {section === "bigFiles" && (
                  <BigFiles
                    total={scan.root.size}
                    version={version}
                    filter={typeFilter}
                    onClearFilter={() => setTypeFilter(null)}
                    actions={actions}
                  />
                )}
              </>
            )
          )}
          {notice && (
            <p className="notice" role="status">
              {notice}
            </p>
          )}
        </main>

        {scan && !scanning && (
          <aside
            className={`basket${dropping ? " dropping" : ""}`}
            aria-label={t.basketTitle}
            onDragOver={(e) => {
              e.preventDefault();
              e.dataTransfer.dropEffect = "copy";
              setDropping(true);
            }}
            onDragLeave={() => setDropping(false)}
            onDrop={(e) => {
              e.preventDefault();
              setDropping(false);
              const id = Number(e.dataTransfer.getData("text/plain"));
              if (Number.isInteger(id)) void api.nodeInfo(id).then((info) => info && void addToBasket(info));
            }}
          >
            <div>
              <h2>{t.basketTitle}</h2>
              <p className="muted">{t.basketHint}</p>
            </div>
            <ul className="basket-items">
              {basket.map((item) => (
                <li key={item.id}>
                  <div className="basket-text">
                    <strong title={item.name}>{item.name}</strong>
                    <span className="muted" title={item.path}>
                      {formatSize(item.size)} · {item.path}
                    </span>
                  </div>
                  <button
                    className="icon"
                    aria-label={t.removeFromBasketAria(item.name)}
                    onClick={() => setBasket((prev) => prev.filter((b) => b.id !== item.id))}
                  >
                    <svg aria-hidden="true" width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                      <path d="M3 3l10 10M13 3L3 13" />
                    </svg>
                  </button>
                </li>
              ))}
            </ul>
            <div className="dropzone">{t.dropHere}</div>
            <div className="basket-total">
              <div>
                <div className="muted">{t.youRecover}</div>
                <div className="big-number recover">{formatSize(basketSize)}</div>
              </div>
              <button className="trash" disabled={basket.length === 0 || emptying} onClick={() => setConfirming(true)}>
                {emptying ? t.sending : basket.length === 0 ? t.basketEmpty : t.sendItems(itemsLabel)}
              </button>
              <p className="muted small-text">{t.nothingLost}</p>
              <button className="link bin-link" onClick={() => api.openRecycleBin().catch((e) => setNotice(String(e)))}>
                {t.openBin}
              </button>
            </div>
          </aside>
        )}

        {menu && (
          <div
            className="menu"
            style={{
              left: Math.min(menu.x, window.innerWidth - 260),
              top: Math.min(menu.y, window.innerHeight - 190),
            }}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={() => setMenu(null)}
          >
            <div className="menu-title">{menu.info.name}</div>
            {menu.info.isDir && <button onClick={() => void openFolder(menu.info.id)}>{t.open}</button>}
            <button onClick={() => api.reveal(menu.info.id).catch((e) => setNotice(String(e)))}>{t.reveal}</button>
            <button onClick={() => void navigator.clipboard.writeText(menu.info.path)}>{t.copyPath}</button>
            {menu.info.id !== 0 && (
              <button onClick={() => void toggleBasket(menu.info.id)}>
                {basketIds.has(menu.info.id) ? t.removeFromBasket : t.addToBasket}
              </button>
            )}
          </div>
        )}

        {confirming && (
          <div className="backdrop" onMouseDown={() => setConfirming(false)}>
            <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="confirm-title" onMouseDown={(e) => e.stopPropagation()}>
              <h3 id="confirm-title">{t.confirmTitle(itemsLabel)}</h3>
              <ul className="dialog-list">
                {basket.map((item) => (
                  <li key={item.id}>
                    <span>
                      {item.path}
                      {item.caution && <span className="caution-tag">{t.cautionTag}</span>}
                    </span>
                    <span>{formatSize(item.size)}</span>
                  </li>
                ))}
              </ul>
              {basket.some((item) => item.caution) && <p className="caution">{t.cautionWarning}</p>}
              <p>{t.confirmBody(formatSize(basketSize))}</p>
              <div className="dialog-actions">
                <button className="outline" autoFocus onClick={() => setConfirming(false)}>
                  {t.cancel}
                </button>
                <button className="trash" onClick={() => void emptyBasket()}>
                  {t.confirmSend}
                </button>
              </div>
            </div>
          </div>
        )}

        {settingsOpen && (
          <SettingsDialog current={settings} onSave={applySettings} onClose={() => setSettingsOpen(false)} />
        )}
      </div>
    </LangContext.Provider>
  );
}
