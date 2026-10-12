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

/** Tramos con color propio en la barra de reparto; el resto se agrupa en uno más. */
export const SHARE_PARTS = 7;

/** Tramo de la barra de reparto que corresponde a la fila `index` de una lista. */
export function shareSegment(index: number): number {
  return Math.min(index, SHARE_PARTS);
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
  /** Resaltada porque el ratón está sobre su tramo de la barra de reparto. */
  highlighted?: boolean;
  onHover?: (inside: boolean) => void;
}

export function ItemRow(props: Props) {
  const { id, name, detail, size, total, color, canOpen, actions, highlighted, onHover } = props;
  const t = useT();
  const inBasket = actions.basketIds.has(id);
  return (
    <li
      className={`row${inBasket ? " in-basket" : ""}${highlighted ? " highlighted" : ""}`}
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
      onMouseEnter={onHover && (() => onHover(true))}
      onMouseLeave={onHover && (() => onHover(false))}
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
  /** Tramo resaltado (compartido con la lista de debajo), o `null`. */
  active: number | null;
  onActive: (segment: number | null) => void;
}

/** Barra apilada con el reparto de los elementos que más ocupan y, debajo, qué es el tramo señalado. */
export function ShareBar({ parts, total, active, onActive }: ShareBarProps) {
  const t = useT();
  const segments = parts.slice(0, SHARE_PARTS).map((p) => ({ label: p.label, size: p.size }));
  const rest = total - segments.reduce((sum, p) => sum + p.size, 0);
  if (rest > 0) segments.push({ label: t.rest, size: rest });
  const current = active !== null ? segments[active] : undefined;

  return (
    <div className="share">
      <div
        className={`sharebar${current ? " has-active" : ""}`}
        role="img"
        aria-label={t.shareAria}
        onMouseLeave={() => onActive(null)}
      >
        {segments.map((p, i) => (
          <span
            key={i}
            className={i === active ? "active" : undefined}
            style={{ flexGrow: p.size, background: rankColor(i) }}
            onMouseEnter={() => onActive(i)}
          />
        ))}
      </div>
      <div className="share-caption" aria-live="polite">
        {current ? (
          <>
            <span className="swatch" style={{ background: rankColor(active!) }} aria-hidden="true" />
            <strong>{current.label}</strong>
            <span>
              {formatSize(current.size)} · {formatPercent(current.size, total)}
            </span>
          </>
        ) : (
          <span className="muted">{segments.length > 0 ? t.shareHint : ""}</span>
        )}
      </div>
    </div>
  );
}
