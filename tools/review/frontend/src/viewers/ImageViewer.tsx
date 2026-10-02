import { useEffect, useRef, useState } from "react";
import { fileUrl } from "../api";

const ZOOMS = [1, 2, 4, 8];
type Zoom = number | "fit";

type Color31 = { label: string; rgb4: string; rgba8: number[] };
let color31Options: Promise<Color31[]> | null = null;
const loadColor31 = () =>
  (color31Options ??= fetch("/api/color31").then((r) => (r.ok ? r.json() : [])).catch(() => []));

// Indexed PNGs (palette colour type) get a colour-31 selector: the backend rewrites palette entry 31
// on the fly (`?color31=`), so the four region_overrides values from T1.1 can be previewed.
export default function ImageViewer({ path }: { path: string }) {
  const [zoom, setZoom] = useState(2);
  const [big, setBig] = useState(false);
  const [bigZoom, setBigZoom] = useState<Zoom>("fit");
  const [size, setSize] = useState<[number, number] | null>(null);
  const [indexed, setIndexed] = useState(false);
  const [options, setOptions] = useState<Color31[]>([]);
  const [color31, setColor31] = useState<string | null>(null);
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let url: string | null = null;
    let alive = true;
    const q = color31 ? `?color31=${encodeURIComponent(color31)}` : "";
    fetch(fileUrl(path) + q)
      .then(async (r) => {
        if (!alive) return;
        const isIndexed = r.headers.get("X-Indexed-Png") === "1";
        setIndexed(isIndexed);
        if (isIndexed) setOptions(await loadColor31());
        url = URL.createObjectURL(await r.blob());
        if (alive) setSrc(url);
      })
      .catch(() => alive && setSrc(fileUrl(path) + q));
    return () => {
      alive = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [path, color31]);

  // Escape closes the lightbox; the page behind it does not scroll while it is open.
  useEffect(() => {
    if (!big) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setBig(false);
    window.addEventListener("keydown", onKey);
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      window.removeEventListener("keydown", onKey);
      document.body.style.overflow = prev;
    };
  }, [big]);

  const zoomButtons = (current: Zoom, set: (z: Zoom) => void, fit: boolean) => (
    <>
      {fit && (
        <button className={current === "fit" ? "active" : ""} onClick={() => set("fit")}>
          fit
        </button>
      )}
      {ZOOMS.map((z) => (
        <button key={z} className={z === current ? "active" : ""} onClick={() => set(z)}>
          {z}×
        </button>
      ))}
    </>
  );

  return (
    <div className="image-viewer">
      <div className="zoom">
        {zoomButtons(zoom, (z) => setZoom(z as number), false)}
        <button onClick={() => setBig(true)} title="Open full-window (click the image does the same; Esc closes)">
          ⤢ expand
        </button>
        {size && <span className="muted small">{size[0]}×{size[1]} px</span>}
        {indexed && options.length > 0 && (
          <label className="small color31">
            colour 31:
            <select value={color31 ?? ""} onChange={(e) => setColor31(e.target.value || null)}>
              <option value="">as shipped</option>
              {options.map((o) => (
                <option key={o.rgb4} value={o.rgb4}>
                  {o.label}
                </option>
              ))}
            </select>
            {color31 && (
              <span
                className="c31-chip"
                style={{ background: `rgb(${options.find((o) => o.rgb4 === color31)?.rgba8.slice(0, 3).join(",")})` }}
              />
            )}
          </label>
        )}
      </div>
      <div className="image-frame">
        {src && (
          <img
            src={src}
            alt={path}
            title="click to open full-window"
            onClick={() => setBig(true)}
            onLoad={(e) => setSize([e.currentTarget.naturalWidth, e.currentTarget.naturalHeight])}
            style={size ? { width: size[0] * zoom, height: size[1] * zoom, cursor: "zoom-in" } : undefined}
          />
        )}
      </div>
      {big && src && size && (
        <Lightbox path={path} src={src} size={size} zoom={bigZoom} onZoom={setBigZoom} onClose={() => setBig(false)}>
          {zoomButtons(bigZoom, setBigZoom, true)}
        </Lightbox>
      )}
    </div>
  );
}

// Full-window view: fit-to-window or fixed zoom, scroll/drag to pan, Esc / × / backdrop click closes.
function Lightbox({
  path, src, size, zoom, onZoom, onClose, children,
}: {
  path: string; src: string; size: [number, number]; zoom: Zoom; onZoom: (z: Zoom) => void;
  onClose: () => void; children: React.ReactNode;
}) {
  const frame = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; sx: number; sy: number } | null>(null);
  const [fitScale, setFitScale] = useState(1);
  useEffect(() => {
    const calc = () => {
      const el = frame.current;
      if (!el) return;
      setFitScale(Math.min((el.clientWidth - 16) / size[0], (el.clientHeight - 16) / size[1]));
    };
    calc();
    window.addEventListener("resize", calc);
    return () => window.removeEventListener("resize", calc);
  }, [size]);
  const scale = zoom === "fit" ? fitScale : zoom;
  // Ctrl+wheel steps the zoom (plain wheel pans). Native listener: React registers wheel as
  // passive, so preventDefault (needed to stop the browser's page zoom) would be ignored.
  const zoomRef = useRef(zoom);
  zoomRef.current = zoom;
  useEffect(() => {
    const el = frame.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey) return;
      e.preventDefault();
      const steps: Zoom[] = ["fit", ...ZOOMS];
      const i = steps.indexOf(zoomRef.current);
      onZoom(steps[Math.max(0, Math.min(steps.length - 1, i + (e.deltaY < 0 ? 1 : -1)))]);
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [onZoom]);
  return (
    <div className="lightbox" onClick={onClose}>
      <div className="lightbox-bar" onClick={(e) => e.stopPropagation()}>
        <span className="lightbox-title">{path}</span>
        <div className="zoom">{children}</div>
        <span className="muted small">
          {size[0]}×{size[1]} px · {zoom === "fit" ? `${(fitScale * 100).toFixed(0)}%` : `${zoom}×`} · drag to pan, Ctrl+wheel to zoom, Esc to close
        </span>
        <button className="lightbox-close" onClick={onClose}>
          ×
        </button>
      </div>
      <div
        ref={frame}
        className="lightbox-frame image-frame"
        onClick={(e) => e.stopPropagation()}
        onMouseDown={(e) => {
          const el = frame.current!;
          drag.current = { x: e.clientX, y: e.clientY, sx: el.scrollLeft, sy: el.scrollTop };
        }}
        onMouseMove={(e) => {
          if (!drag.current || !frame.current) return;
          frame.current.scrollLeft = drag.current.sx - (e.clientX - drag.current.x);
          frame.current.scrollTop = drag.current.sy - (e.clientY - drag.current.y);
        }}
        onMouseUp={() => (drag.current = null)}
        onMouseLeave={() => (drag.current = null)}
      >
        <img src={src} alt={path} draggable={false} style={{ width: size[0] * scale, height: size[1] * scale, margin: "auto" }} />
      </div>
    </div>
  );
}
