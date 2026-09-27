import { useState } from "react";
import { fileUrl } from "../api";

const ZOOMS = [1, 2, 4, 8];

export default function ImageViewer({ path }: { path: string }) {
  const [zoom, setZoom] = useState(2);
  const [size, setSize] = useState<[number, number] | null>(null);
  return (
    <div className="image-viewer">
      <div className="zoom">
        {ZOOMS.map((z) => (
          <button key={z} className={z === zoom ? "active" : ""} onClick={() => setZoom(z)}>
            {z}×
          </button>
        ))}
        {size && <span className="muted small">{size[0]}×{size[1]} px</span>}
      </div>
      <div className="image-frame">
        <img
          src={fileUrl(path)}
          alt={path}
          onLoad={(e) => setSize([e.currentTarget.naturalWidth, e.currentTarget.naturalHeight])}
          style={size ? { width: size[0] * zoom, height: size[1] * zoom } : undefined}
        />
      </div>
    </div>
  );
}
