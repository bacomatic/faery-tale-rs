import { useEffect, useState } from "react";
import { fileUrl } from "../api";

const ZOOMS = [1, 2, 4, 8];

type Color31 = { label: string; rgb4: string; rgba8: number[] };
let color31Options: Promise<Color31[]> | null = null;
const loadColor31 = () =>
  (color31Options ??= fetch("/api/color31").then((r) => (r.ok ? r.json() : [])).catch(() => []));

// Indexed PNGs (palette colour type) get a colour-31 selector: the backend rewrites palette entry 31
// on the fly (`?color31=`), so the four region_overrides values from T1.1 can be previewed.
export default function ImageViewer({ path }: { path: string }) {
  const [zoom, setZoom] = useState(2);
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

  return (
    <div className="image-viewer">
      <div className="zoom">
        {ZOOMS.map((z) => (
          <button key={z} className={z === zoom ? "active" : ""} onClick={() => setZoom(z)}>
            {z}×
          </button>
        ))}
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
            onLoad={(e) => setSize([e.currentTarget.naturalWidth, e.currentTarget.naturalHeight])}
            style={size ? { width: size[0] * zoom, height: size[1] * zoom } : undefined}
          />
        )}
      </div>
    </div>
  );
}
