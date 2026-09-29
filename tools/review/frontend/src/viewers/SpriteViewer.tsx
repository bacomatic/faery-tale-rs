import { useEffect, useMemo, useRef, useState } from "react";
import { fileUrl } from "../api";
import CitationLink from "../components/CitationLink";
import { useFetched } from "./useFetched";

// Renders an actor atlas written by tools/extract_sprites.py: pick a mode, facing and weapon,
// then play the resolved step sequence with the weapon overlay drawn from the OBJECTS sheet.

interface Overlay { frame: number; rows: [number, number]; dx: number; dy: number; behind: boolean }
interface Step { frame: number; ticks?: number; state_index?: number; overlays?: Record<string, Overlay> }
interface Mode {
  id: string; label: string; source: string; playback: "loop" | "once" | "transitions" | "random";
  directional: boolean; weapons?: number[]; transitions?: number[][]; note?: string;
  facings?: Record<string, Step[]>; steps?: Step[];
}
interface Frame { index: number; rect: [number, number, number, number]; origin?: { cfile: number; sheet: string; frame: number; padded_from?: number[] } }
interface Atlas {
  title: string; frame_size: [number, number];
  sheet: { file: string; highlight_mask: string; silhouette_mask: string };
  overlay_sheet?: { file: string; frame_size: [number, number]; columns: number };
  weapons?: { id: number; name: string }[];
  frames: Frame[]; modes: Mode[]; notes?: { text: string; source: string }[];
}

const COMPASS = ["NW", "N", "NE", "W", "", "E", "SW", "S", "SE"];
const ZOOMS = [2, 4, 6, 8];
const LAYERS = ["colour", "highlight", "silhouette"] as const;
type Layer = (typeof LAYERS)[number];

function useImage(path: string | null) {
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  useEffect(() => {
    setImg(null);
    if (!path) return;
    const i = new Image();
    i.onload = () => setImg(i);
    i.src = fileUrl(path);
  }, [path]);
  return img;
}

const layerFile = (sheet: string, layer: Layer) =>
  layer === "colour" ? sheet : sheet.replace(/_sheet\.png$/, layer === "highlight" ? "_highlightmask.png" : "_silhouettemask.png");

function stepsOf(mode: Mode, facing: string): Step[] {
  return (mode.directional ? mode.facings?.[facing] : mode.steps) ?? [];
}

export default function SpriteViewer({ path }: { path: string }) {
  const { data, err } = useFetched<Atlas>(path, (r) => r.json());
  if (err) return <p className="error">{err}</p>;
  if (!data) return <p className="muted">Loading…</p>;
  return <Player atlas={data} dir={path.slice(0, path.lastIndexOf("/"))} />;
}

function Player({ atlas, dir }: { atlas: Atlas; dir: string }) {
  const [modeId, setModeId] = useState(atlas.modes[0]?.id ?? "");
  const [facing, setFacing] = useState("S");
  const [weapon, setWeapon] = useState(0);
  const [layer, setLayer] = useState<Layer>("colour");
  const [zoom, setZoom] = useState(4);
  const [fps, setFps] = useState(8);
  const [playing, setPlaying] = useState(true);
  const [pos, setPos] = useState({ idx: 0, left: 1 });
  const canvas = useRef<HTMLCanvasElement>(null);

  const mode = atlas.modes.find((m) => m.id === modeId) ?? atlas.modes[0];
  const steps = stepsOf(mode, facing);
  const allowed = (atlas.weapons ?? []).filter((w) => mode.weapons?.includes(w.id));
  const w = allowed.some((x) => x.id === weapon) ? weapon : (allowed[0]?.id ?? 0);

  const body = useImage(`${dir}/${layerFile(atlas.sheet.file, layer)}`);
  const objs = useImage(atlas.overlay_sheet ? layerFile(atlas.overlay_sheet.file, layer) : null);

  // Canvas bounds cover every overlay in the atlas so the body never jumps between modes.
  const box = useMemo(() => {
    const [fw, fh] = atlas.frame_size;
    let x0 = 0, y0 = 0, x1 = fw, y1 = fh;
    for (const m of atlas.modes)
      for (const seq of m.directional ? Object.values(m.facings ?? {}) : [m.steps ?? []])
        for (const s of seq)
          for (const o of Object.values(s.overlays ?? {})) {
            x0 = Math.min(x0, o.dx); y0 = Math.min(y0, o.dy);
            x1 = Math.max(x1, o.dx + 16); y1 = Math.max(y1, o.dy + o.rows[1] - o.rows[0]);
          }
    return { x0, y0, w: x1 - x0, h: y1 - y0 };
  }, [atlas]);

  // Reset playback when the sequence changes.
  useEffect(() => setPos({ idx: 0, left: steps[0]?.ticks ?? 1 }), [modeId, facing]);

  const advance = () =>
    setPos(({ idx, left }) => {
      if (!steps.length) return { idx: 0, left: 1 };
      if (mode.playback === "random") {
        const n = Math.floor(Math.random() * steps.length);
        return { idx: n, left: 1 };
      }
      if (left > 1) return { idx, left: left - 1 };
      let next: number;
      if (mode.playback === "transitions" && mode.transitions)
        next = mode.transitions[idx][Math.floor(Math.random() * 4)];
      else if (mode.playback === "once") next = Math.min(idx + 1, steps.length - 1);
      else next = (idx + 1) % steps.length;
      return { idx: next, left: steps[next]?.ticks ?? 1 };
    });

  useEffect(() => {
    if (!playing) return;
    const t = setInterval(advance, 1000 / fps);
    return () => clearInterval(t);
  }, [playing, fps, modeId, facing, steps]);

  const step = steps[Math.min(pos.idx, steps.length - 1)];
  const ov = w && step?.overlays ? step.overlays[String(w)] : undefined;
  const frame = step ? atlas.frames[step.frame] : undefined;

  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const ctx = c.getContext("2d")!;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, c.width, c.height);
    if (!frame || !body) return;
    const drawOverlay = () => {
      if (!ov || !objs || !atlas.overlay_sheet) return;
      const [ow, oh] = atlas.overlay_sheet.frame_size;
      const cols = atlas.overlay_sheet.columns;
      const sx = (ov.frame % cols) * ow, sy = Math.floor(ov.frame / cols) * oh + ov.rows[0];
      const h = ov.rows[1] - ov.rows[0];
      ctx.drawImage(objs, sx, sy, ow, h, (ov.dx - box.x0) * zoom, (ov.dy - box.y0) * zoom, ow * zoom, h * zoom);
    };
    const [x, y, fw, fh] = frame.rect;
    if (ov?.behind) drawOverlay();
    ctx.drawImage(body, x, y, fw, fh, -box.x0 * zoom, -box.y0 * zoom, fw * zoom, fh * zoom);
    if (ov && !ov.behind) drawOverlay();
  }, [frame, ov, body, objs, zoom, box, atlas]);

  return (
    <div className="sprite-viewer">
      <div className="sprite-controls">
        <div className="zoom">
          <span className="muted small">mode</span>
          {atlas.modes.map((m) => (
            <button key={m.id} className={m.id === mode.id ? "active" : ""} onClick={() => setModeId(m.id)}>
              {m.label}
            </button>
          ))}
        </div>
        <div className="zoom">
          <span className="muted small">weapon</span>
          {allowed.length === 0 && <span className="muted small">none</span>}
          {allowed.map((x) => (
            <button key={x.id} className={x.id === w ? "active" : ""} onClick={() => setWeapon(x.id)}>
              {x.name}
            </button>
          ))}
        </div>
        <div className="zoom">
          <span className="muted small">view</span>
          {LAYERS.map((l) => (
            <button key={l} className={l === layer ? "active" : ""} onClick={() => setLayer(l)}>{l}</button>
          ))}
          <span className="muted small">zoom</span>
          {ZOOMS.map((z) => (
            <button key={z} className={z === zoom ? "active" : ""} onClick={() => setZoom(z)}>{z}×</button>
          ))}
        </div>
        <div className="zoom">
          <button onClick={() => setPlaying(!playing)}>{playing ? "pause" : "play"}</button>
          <button disabled={playing} onClick={advance}>step</button>
          <button onClick={() => setPos({ idx: 0, left: steps[0]?.ticks ?? 1 })}>restart</button>
          <span className="muted small">ticks/s</span>
          <input type="range" min={1} max={30} value={fps} onChange={(e) => setFps(Number(e.target.value))} />
          <span className="small">{fps}</span>
        </div>
      </div>

      <div className="sprite-stage">
        <div className={`compass ${mode.directional ? "" : "disabled"}`}>
          {COMPASS.map((d, i) =>
            d ? (
              <button key={d} disabled={!mode.directional} className={d === facing ? "active" : ""} onClick={() => setFacing(d)}>
                {d}
              </button>
            ) : (
              <span key={i} />
            ),
          )}
        </div>
        <div className="image-frame">
          <canvas ref={canvas} width={box.w * zoom} height={box.h * zoom} style={{ imageRendering: "pixelated" }} />
        </div>
        <div className="small sprite-info">
          <div>
            step <b>{pos.idx + 1}</b>/{steps.length} · playback <code>{mode.playback}</code>
            {mode.directional ? <> · facing <b>{facing}</b></> : null}
          </div>
          {frame && (
            <div>
              frame <b>{frame.index}</b>
              {frame.origin && (
                <> ← {frame.origin.sheet} #{frame.origin.frame}{frame.origin.padded_from ? " (padded)" : ""}</>
              )}
              {step?.state_index !== undefined && <> · statelist[{step.state_index}]</>}
            </div>
          )}
          {ov && (
            <div>
              overlay objects #{ov.frame} rows {ov.rows[0]}–{ov.rows[1] - 1} · dx {ov.dx} dy {ov.dy} ·{" "}
              {ov.behind ? "behind" : "in front"}
            </div>
          )}
          <div className="sequence">
            {steps.map((s, i) => (
              <span key={i} className={i === pos.idx ? "current" : ""}>
                {s.frame}{s.ticks && s.ticks > 1 ? `×${s.ticks}` : ""}
              </span>
            ))}
          </div>
          <div>
            source <CitationLink cite={mode.source} />
          </div>
          {mode.note && <div className="muted">{mode.note}</div>}
        </div>
      </div>
      {atlas.notes && atlas.notes.length > 0 && (
        <ul className="small sprite-notes">
          {atlas.notes.map((n, i) => (
            <li key={i}>
              {n.text} <CitationLink cite={n.source} />
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
