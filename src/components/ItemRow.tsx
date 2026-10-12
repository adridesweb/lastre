import { formatPercent, formatSize } from "../format";
import { useT } from "../i18n";
import { rankColor } from "../palettes";

/** Lo que las listas necesitan para actuar sobre un elemento. */
export interface ItemActions {
  basketIds: Set<number>;
  toggleBasket: (id: number) => void;
  openFolder: (id: number) => void;
  openMenu: (id: number, x: number, y: number) => void;
}

interface Props {
  id: number;
  name: string;
  /** Segunda línea opcional (p. ej. la carpeta que lo contiene). */
  detail?: string;
  size: number;
  total: number;
  color: string;
  canOpen: boolean;
  actions: ItemActions;
}

export function ItemRow({ id, name, detail, size, total, color, canOpen, actions }: Props) {
  const t = useT();
  const inBasket = actions.basketIds.has(id);
  return (
    <li
      className={`row${inBasket ? " in-basket" : ""}`}
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData("text/plain", String(id));
        e.dataTransfer.effectAllowed = "copy";
      }}
      onDoubleClick={() => canOpen && actions.openFolder(id)}
      onContextMenu={(e) => {
        e.preventDefault();
        actions.openMenu(id, e.clientX, e.clientY);
      }}
    >
      <span className="swatch" style={{ background: color }} aria-hidden="true" />
      <span className="row-name">
        <span className="row-title" title={name}>
          {name}
        </span>
        {detail && (
          <span className="row-detail" title={detail}>
            {detail}
          </span>
        )}
      </span>
      <span className="row-size">{formatSize(size)}</span>
      <span className="row-pct">{formatPercent(size, total)}</span>
      <span className="row-actions">
        {canOpen && (
          <button className="small" onClick={() => actions.openFolder(id)}>
            {t.open}
          </button>
        )}
        <button
          className={`small${inBasket ? " basket-on" : ""}`}
          aria-pressed={inBasket}
          onClick={() => actions.toggleBasket(id)}
        >
          {inBasket ? t.inBasket : t.toBasket}
        </button>
      </span>
    </li>
  );
}

interface ShareBarProps {
  parts: { label: string; size: number }[];
  total: number;
}

const SHARE_PARTS = 7;

/** Barra apilada con el reparto de los elementos que más ocupan. */
export function ShareBar({ parts, total }: ShareBarProps) {
  const t = useT();
  const shown = parts.slice(0, SHARE_PARTS);
  const rest = total - shown.reduce((sum, p) => sum + p.size, 0);
  return (
    <div className="sharebar" role="img" aria-label={t.shareAria}>
      {shown.map((p, i) => (
        <span
          key={i}
          title={`${p.label} · ${formatSize(p.size)}`}
          style={{ flexGrow: p.size, background: rankColor(i) }}
        />
      ))}
      {rest > 0 && (
        <span
          title={`${t.rest} · ${formatSize(rest)}`}
          style={{ flexGrow: rest, background: rankColor(SHARE_PARTS) }}
        />
      )}
    </div>
  );
}
