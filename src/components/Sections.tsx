import { useEffect, useState } from "react";
import { api, type Children, type FileType, type NodeDto, type NodeInfo } from "../api";
import { formatCount, formatPercent, formatSize } from "../format";
import { useT } from "../i18n";
import { rankColor } from "../palettes";
import { ItemRow, ShareBar, shareSegment, type ItemActions } from "./ItemRow";

const PAGE = 200;

/** Vuelve a pedir datos cuando cambian las dependencias; ignora respuestas obsoletas. */
function useLoaded<T>(load: () => Promise<T>, deps: unknown[]): T | null {
  const [value, setValue] = useState<T | null>(null);
  useEffect(() => {
    let stale = false;
    load().then(
      (v) => !stale && setValue(v),
      () => !stale && setValue(null),
    );
    return () => {
      stale = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return value;
}

interface ChildListProps {
  items: NodeDto[];
  total: number;
  actions: ItemActions;
  active: number | null;
  onActive: (segment: number | null) => void;
}

function ChildList(props: ChildListProps) {
  return (
    <ul className="rows">
      {props.items.map((item, i) => (
        <ItemRow
          key={item.id}
          id={item.id}
          name={item.name}
          size={item.size}
          total={props.total}
          color={rankColor(i)}
          canOpen={item.isDir && item.hasChildren}
          actions={props.actions}
          highlighted={props.active === shareSegment(i)}
          onHover={(inside) => props.onActive(inside ? shareSegment(i) : null)}
        />
      ))}
    </ul>
  );
}

interface SummaryProps {
  root: NodeDto;
  denied: number;
  version: number;
  actions: ItemActions;
  onRescan: () => void;
  onSeeAll: () => void;
}

export function Summary({ root, denied, version, actions, onRescan, onSeeAll }: SummaryProps) {
  const t = useT();
  const children = useLoaded<Children>(() => api.getChildren(0), [version]);
  const [active, setActive] = useState<number | null>(null);
  const items = children?.items ?? [];
  return (
    <>
      <section className="hero">
        <div className="hero-top">
          <div>
            <div className="muted">{t.carries(root.name)}</div>
            <h1 className="big-number">{formatSize(root.size)}</h1>
          </div>
          <button className="outline" onClick={onRescan}>
            {t.measureAgain}
          </button>
        </div>
        <ShareBar
          parts={items.map((i) => ({ label: i.name, size: i.size }))}
          total={root.size}
          active={active}
          onActive={setActive}
        />
        <div className="muted">
          {t.files(formatCount(root.files))}
          {denied > 0 && <span title={t.deniedTip}> · {t.denied(formatCount(denied))}</span>}
        </div>
      </section>
      <section>
        <div className="section-head">
          <h2>{t.whatTakesMost}</h2>
          {items.length > 8 && (
            <button className="link" onClick={onSeeAll}>
              {t.seeAll(formatCount(items.length + (children?.hiddenCount ?? 0)))}
            </button>
          )}
        </div>
        <ChildList
          items={items.slice(0, 8)}
          total={root.size}
          actions={actions}
          active={active}
          onActive={setActive}
        />
      </section>
    </>
  );
}

interface FoldersProps {
  folder: NodeInfo;
  rootPath: string;
  version: number;
  actions: ItemActions;
}

export function Folders({ folder, rootPath, version, actions }: FoldersProps) {
  const t = useT();
  const children = useLoaded<Children>(() => api.getChildren(folder.id), [folder.id, version]);
  const [shown, setShown] = useState(PAGE);
  const [active, setActive] = useState<number | null>(null);
  useEffect(() => {
    setShown(PAGE);
    setActive(null);
  }, [folder.id]);

  const items = children?.items ?? [];
  const crumbs = [rootPath, ...folder.path.slice(rootPath.length).split("\\").filter(Boolean)].map(
    (label, i) => ({ label, id: folder.ancestors[i] }),
  );
  const pending = items.length - shown + (children?.hiddenCount ?? 0);

  return (
    <section>
      <nav className="crumbs" aria-label={t.pathAria}>
        {crumbs.map((c, i) => (
          <span key={c.id}>
            {i > 0 && <span className="crumb-sep">›</span>}
            <button
              className="link"
              disabled={i === crumbs.length - 1}
              onClick={() => actions.openFolder(c.id)}
            >
              {c.label}
            </button>
          </span>
        ))}
      </nav>
      <div className="section-head">
        <h2>{formatSize(folder.size)}</h2>
        <span className="muted">{t.files(formatCount(folder.files))}</span>
      </div>
      <ShareBar
        parts={items.map((i) => ({ label: i.name, size: i.size }))}
        total={folder.size}
        active={active}
        onActive={setActive}
      />
      <ChildList
        items={items.slice(0, shown)}
        total={folder.size}
        actions={actions}
        active={active}
        onActive={setActive}
      />
      {children && items.length === 0 && <p className="muted pad">{t.emptyFolder}</p>}
      {pending > 0 && (
        <div className="more">
          <span className="muted">{t.remaining(formatCount(pending))}</span>
          {shown < items.length && (
            <button className="small" onClick={() => setShown((n) => n + PAGE)}>
              {t.showMore}
            </button>
          )}
        </div>
      )}
    </section>
  );
}

interface TypesProps {
  total: number;
  version: number;
  onPick: (type: FileType) => void;
}

export function Types({ total, version, onPick }: TypesProps) {
  const t = useT();
  const types = useLoaded<FileType[]>(() => api.getFileTypes(0), [version]) ?? [];
  const [active, setActive] = useState<number | null>(null);
  const label = (type: FileType) => (type.name ? `.${type.name}` : t.noExtension);
  return (
    <section>
      <div className="section-head">
        <h2>{t.navTypes}</h2>
        <span className="muted">{t.bySpace}</span>
      </div>
      <ShareBar
        parts={types.map((type) => ({ label: label(type), size: type.size }))}
        total={total}
        active={active}
        onActive={setActive}
      />
      <ul className="rows">
        {types.map((type, i) => (
          <li
            key={type.ext}
            className={`row${active === shareSegment(i) ? " highlighted" : ""}`}
            onMouseEnter={() => setActive(shareSegment(i))}
            onMouseLeave={() => setActive(null)}
          >
            <span className="swatch" style={{ background: rankColor(i) }} aria-hidden="true" />
            <span className="row-name">
              <span className="row-title">{label(type)}</span>
              <span className="row-detail">{t.files(formatCount(type.count))}</span>
            </span>
            <span className="row-size">{formatSize(type.size)}</span>
            <span className="row-pct">{formatPercent(type.size, total)}</span>
            <span className="row-actions">
              <button className="small" onClick={() => onPick(type)}>
                {t.seeFiles}
              </button>
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** Carpeta que contiene el archivo; la raíz de una unidad conserva su barra final. */
function parentPath(file: NodeInfo): string {
  const parent = file.path.slice(0, file.path.length - file.name.length - 1);
  return parent.endsWith(":") ? parent + "\\" : parent;
}

interface BigFilesProps {
  total: number;
  version: number;
  filter: FileType | null;
  onClearFilter: () => void;
  actions: ItemActions;
}

export function BigFiles({ total, version, filter, onClearFilter, actions }: BigFilesProps) {
  const t = useT();
  const ext = filter?.ext ?? null;
  const files = useLoaded<NodeInfo[]>(() => api.largestFiles(0, ext, PAGE), [ext, version]) ?? [];
  return (
    <section>
      <div className="section-head">
        <h2>{t.navBigFiles}</h2>
        {filter ? (
          <button className="chip" onClick={onClearFilter} aria-label={t.clearTypeAria}>
            {t.onlyType(filter.name ? `.${filter.name}` : t.noExtension)}
          </button>
        ) : (
          <span className="muted">{t.topFiles(PAGE)}</span>
        )}
      </div>
      <ul className="rows">
        {files.map((f, i) => (
          <ItemRow
            key={f.id}
            id={f.id}
            name={f.name}
            detail={parentPath(f)}
            size={f.size}
            total={total}
            color={rankColor(i)}
            canOpen={false}
            actions={actions}
          />
        ))}
      </ul>
    </section>
  );
}
