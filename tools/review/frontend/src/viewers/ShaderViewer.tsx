import { useEffect, useMemo, useRef, useState } from "react";
import { fileUrl } from "../api";
import CitationLink from "../components/CitationLink";
import { useFetched } from "./useFetched";

// Runs a shipped GLSL ES 3.00 fragment shader (assets/shaders/*.glsl) verbatim in WebGL2 on a
// chosen subject (tile atlas / actor sheet), with uniform controls from shaders.json, and diffs
// the result against the Python fade_page() render in previews/ when one exists for the
// current uniform values.

type UType = "int" | "float" | "bool" | "ivec3";
interface Uniform { name: string; type: UType; label: string; min?: number; max?: number; step?: number; default: number | boolean | number[] }
interface Mode { id: string; label: string; rule?: string; param?: { label: string; min: number; max: number; step: number; default: number } }
interface ShaderDesc { title: string; source: string[]; textures: Record<string, "indexed" | "highlight_mask" | "bank">; uniforms: Uniform[]; modes?: Mode[]; compare?: "fade_page" | "bank" | "full_bright" }
interface Descriptor { shaders: Record<string, ShaderDesc> }
interface Subject { id: string; kind: string; region: number; source: { indexed: string; highlight_mask: string }; levels: number[]; files: Record<string, string>; strip: string }
interface Previews { levels: number[]; subjects: Subject[] }
type Values = Record<string, number | boolean | number[]>;

const ZOOMS = [1, 2, 4];
const VERTEX = `#version 300 es
in vec2 aPos;
out vec2 vUV;
void main() { vUV = vec2(aPos.x * 0.5 + 0.5, 0.5 - aPos.y * 0.5); gl_Position = vec4(aPos, 0.0, 1.0); }`;

const loadImage = (path: string) =>
  new Promise<HTMLImageElement>((ok, fail) => {
    const i = new Image();
    i.onload = () => ok(i);
    i.onerror = () => fail(new Error(`cannot load ${path}`));
    i.src = fileUrl(path);
  });

function compile(gl: WebGL2RenderingContext, type: number, src: string) {
  const s = gl.createShader(type)!;
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "compile failed");
  return s;
}

function upload(gl: WebGL2RenderingContext, target: number) {
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
  gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
  gl.texParameteri(target, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(target, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(target, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(target, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
}

// Which baked preview the current uniforms must reproduce exactly, if any.
function compareFile(desc: ShaderDesc, values: Values, subject: Subject): string | null {
  const jewel = values.uLightTimer === true;
  const f = (k: string) => (subject.files[k] ? `shaders/previews/${subject.files[k]}` : null);
  if (desc.compare === "fade_page") {
    const lv = Number(values.uLightlevel);
    if (jewel) return lv === 0 ? f("jewel") : null;
    return f(String(lv)) ?? (lv >= 180 && subject.levels.includes(180) ? f("180") : null);
  }
  if (desc.compare === "bank") {
    const n = subject.levels.length;
    const pos = Number(values.uLevel01) * (n - 1);
    const k = Math.round(pos);
    return Math.abs(pos - k) < 1e-6 ? f(String(subject.levels[k])) : null;
  }
  if (desc.compare === "full_bright") {
    const w = values.uWeight as number[];
    return !jewel && w.every((x) => x === 100) ? f("180") : null;
  }
  return null;
}

const defaults = (desc: ShaderDesc): Values =>
  Object.fromEntries(desc.uniforms.map((u) => [u.name, Array.isArray(u.default) ? [...u.default] : u.default]));

// Linked-weight modes of fade_to_black: one parameter drives uWeight the way the game does.
function modeWeights(mode: Mode, p: number): number[] | null {
  if (mode.id === "fade") return [p, p, p];                                      // fade_page(i,i,i,FALSE), fmain2.c:623-629
  if (mode.id === "intro_zoom") { const y = Math.trunc((p * 5) / 8); return [2 * y - 40, 2 * y - 70, 2 * y - 100]; } // fmain.c:2917, 2930
  return null;
}

export default function ShaderViewer({ path }: { path: string }) {
  const dir = path.slice(0, path.lastIndexOf("/"));
  const name = path.slice(path.lastIndexOf("/") + 1);
  const glsl = useFetched<string>(path, (r) => r.text());
  const descr = useFetched<Descriptor>(`${dir}/shaders.json`, (r) => r.json());
  const prev = useFetched<Previews>(`${dir}/previews/previews.json`, (r) => r.json());
  const err = glsl.err ?? descr.err ?? prev.err;
  if (err) return <p className="error">{err}</p>;
  if (!glsl.data || !descr.data || !prev.data) return <p className="muted">Loading…</p>;
  const desc = descr.data.shaders[name];
  if (!desc) return <p className="error">{name} is not described in {dir}/shaders.json</p>;
  return <Stage key={name} glsl={glsl.data} desc={desc} previews={prev.data} />;
}

function Stage({ glsl, desc, previews }: { glsl: string; desc: ShaderDesc; previews: Previews }) {
  const needsBank = Object.values(desc.textures).includes("bank");
  const subjects = useMemo(
    () => previews.subjects.filter((s) => !needsBank || s.levels.length > 1),
    [previews, needsBank],
  );
  const [subjectId, setSubjectId] = useState(subjects.find((s) => s.id === "region_07")?.id ?? subjects[0]?.id ?? "");
  const subject = subjects.find((s) => s.id === subjectId) ?? subjects[0];
  const [rawValues, setValues] = useState<Values>(() => defaults(desc));
  const [modeId, setModeId] = useState(desc.modes?.[0]?.id ?? "free");
  const mode = desc.modes?.find((m) => m.id === modeId);
  const [param, setParam] = useState(mode?.param?.default ?? 0);
  const linked = mode?.param ? modeWeights(mode, param) : null;
  const [zoom, setZoom] = useState(2);
  const [showCode, setShowCode] = useState(false);
  const [status, setStatus] = useState<string>("");
  const [diff, setDiff] = useState<{ file: string; differing: number; total: number } | null>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const glRef = useRef<{ gl: WebGL2RenderingContext; prog: WebGLProgram } | null>(null);
  const texRef = useRef<{ id: string; size: [number, number]; textures: Record<string, WebGLTexture> } | null>(null);
  const [texVersion, setTexVersion] = useState(0);

  // Compile once per shader source.
  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const gl = c.getContext("webgl2", { premultipliedAlpha: false, preserveDrawingBuffer: true, antialias: false });
    if (!gl) { setStatus("WebGL2 is not available in this browser"); return; }
    try {
      const prog = gl.createProgram()!;
      gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, VERTEX));
      gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, glsl));
      gl.linkProgram(prog);
      if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog) ?? "link failed");
      const buf = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
      const loc = gl.getAttribLocation(prog, "aPos");
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
      glRef.current = { gl, prog };
      setStatus("compiled OK");
    } catch (e) {
      setStatus(`GLSL error: ${(e as Error).message}`);
    }
  }, [glsl]);

  // Load the subject's textures.
  useEffect(() => {
    const ctx = glRef.current;
    if (!ctx || !subject) return;
    const { gl } = ctx;
    let live = true;
    (async () => {
      const textures: Record<string, WebGLTexture> = {};
      let size: [number, number] = [0, 0];
      for (const [uniform, kind] of Object.entries(desc.textures)) {
        const tex = gl.createTexture()!;
        if (kind === "bank") {
          const imgs = await Promise.all(subject.levels.map((lv) => loadImage(`shaders/previews/${subject.files[String(lv)]}`)));
          if (!live) return;
          size = [imgs[0].naturalWidth, imgs[0].naturalHeight];
          gl.bindTexture(gl.TEXTURE_2D_ARRAY, tex);
          upload(gl, gl.TEXTURE_2D_ARRAY);
          gl.texStorage3D(gl.TEXTURE_2D_ARRAY, 1, gl.RGBA8, size[0], size[1], imgs.length);
          imgs.forEach((im, k) => gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, 0, 0, 0, k, size[0], size[1], 1, gl.RGBA, gl.UNSIGNED_BYTE, im));
        } else {
          const im = await loadImage(kind === "indexed" ? subject.source.indexed : subject.source.highlight_mask);
          if (!live) return;
          if (kind === "indexed") size = [im.naturalWidth, im.naturalHeight];
          gl.bindTexture(gl.TEXTURE_2D, tex);
          upload(gl, gl.TEXTURE_2D);
          gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, im);
        }
        textures[uniform] = tex;
      }
      texRef.current = { id: subject.id, size, textures };
      setTexVersion((v) => v + 1);
    })().catch((e) => live && setStatus(`texture error: ${(e as Error).message}`));
    return () => { live = false; };
  }, [subject, desc, status === "compiled OK"]);

  // Draw, then diff against the baked preview when one applies.
  useEffect(() => {
    const ctx = glRef.current, tex = texRef.current, c = canvas.current;
    if (!ctx || !tex || !c || tex.id !== subject?.id) return;
    const { gl, prog } = ctx;
    const values = linked ? { ...rawValues, uWeight: linked } : rawValues;
    const [w, h] = tex.size;
    c.width = w; c.height = h;
    gl.viewport(0, 0, w, h);
    gl.useProgram(prog);
    let unit = 0;
    for (const [uniform, kind] of Object.entries(desc.textures)) {
      gl.activeTexture(gl.TEXTURE0 + unit);
      gl.bindTexture(kind === "bank" ? gl.TEXTURE_2D_ARRAY : gl.TEXTURE_2D, tex.textures[uniform]);
      gl.uniform1i(gl.getUniformLocation(prog, uniform), unit++);
    }
    for (const u of desc.uniforms) {
      const loc = gl.getUniformLocation(prog, u.name), v = values[u.name];
      if (u.type === "int") gl.uniform1i(loc, Number(v));
      else if (u.type === "bool") gl.uniform1i(loc, v ? 1 : 0);
      else if (u.type === "float") gl.uniform1f(loc, Number(v));
      else if (u.type === "ivec3") { const a = v as number[]; gl.uniform3i(loc, a[0], a[1], a[2]); }
    }
    if (needsBank) gl.uniform1i(gl.getUniformLocation(prog, "uLayerCount"), subject.levels.length);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);

    const file = compareFile(desc, values, subject);
    setDiff(null);
    if (!file) return;
    let live = true;
    loadImage(file).then((im) => {
      if (!live) return;
      const got = new Uint8Array(w * h * 4);
      gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, got);
      const off = document.createElement("canvas");
      off.width = w; off.height = h;
      const c2 = off.getContext("2d", { willReadFrequently: true })!;
      c2.drawImage(im, 0, 0);
      const want = c2.getImageData(0, 0, w, h).data;
      let differing = 0;
      for (let y = 0; y < h; y++)
        for (let x = 0; x < w; x++) {
          const i = (y * w + x) * 4, j = ((h - 1 - y) * w + x) * 4;   // readPixels rows are bottom-up
          const a = want[i + 3];
          // 2D canvas premultiplies, so colour is only comparable where the PNG is fully opaque.
          if (a !== got[j + 3] || (a === 255 && (want[i] !== got[j] || want[i + 1] !== got[j + 1] || want[i + 2] !== got[j + 2]))) differing++;
        }
      setDiff({ file, differing, total: w * h });
    }).catch(() => live && setDiff(null));
    return () => { live = false; };
  }, [rawValues, linked?.join(), subject, desc, texVersion, needsBank]);

  const values = linked ? { ...rawValues, uWeight: linked } : rawValues;
  const set = (n: string, v: number | boolean | number[]) => setValues((o) => ({ ...o, [n]: v }));
  const cmp = diff ? diff : null;

  return (
    <div className="shader-viewer">
      <div className="sprite-controls">
        <div className="zoom">
          <span className="muted small">subject</span>
          <select value={subject?.id ?? ""} onChange={(e) => setSubjectId(e.target.value)}>
            {subjects.map((s) => (
              <option key={s.id} value={s.id}>{s.kind === "tiles" ? `tiles: ${s.id}` : `actor: ${s.id}`}</option>
            ))}
          </select>
          <span className="muted small">zoom</span>
          {ZOOMS.map((z) => (
            <button key={z} className={z === zoom ? "active" : ""} onClick={() => setZoom(z)}>{z}×</button>
          ))}
          <button onClick={() => setValues(defaults(desc))}>reset</button>
          <button className={showCode ? "active" : ""} onClick={() => setShowCode(!showCode)}>source</button>
          {desc.compare && subject && (
            <span className="muted small">
              baked levels:{" "}
              {subject.levels.map((lv) => (
                <button key={lv} className="small" onClick={() => {
                  if (desc.compare === "bank") set("uLevel01", subject.levels.indexOf(lv) / (subject.levels.length - 1));
                  else if (desc.compare === "fade_page") setValues((o) => ({ ...o, uLightlevel: lv, uLightTimer: false }));
                  else { setParam(mode?.param?.max ?? 100); setValues((o) => ({ ...o, uWeight: [100, 100, 100], uLightTimer: false })); }
                }}>{lv}</button>
              ))}
              {desc.compare === "fade_page" && subject.files.jewel && (
                <button className="small" onClick={() => setValues((o) => ({ ...o, uLightlevel: 0, uLightTimer: true }))}>jewel</button>
              )}
            </span>
          )}
        </div>
        {desc.modes && (
          <div className="zoom">
            <span className="muted small">mode</span>
            {desc.modes.map((m) => (
              <button key={m.id} className={m.id === modeId ? "active" : ""} onClick={() => { setModeId(m.id); setParam(m.param?.default ?? 0); }}>{m.label}</button>
            ))}
          </div>
        )}
        {mode?.param && (
          <div className="zoom">
            <span className="muted small">{mode.param.label}</span>
            <input type="range" min={mode.param.min} max={mode.param.max} step={mode.param.step} value={param} onChange={(e) => setParam(Number(e.target.value))} />
            <span className="small">{param}</span>
            <span className="muted small">→ uWeight ({(linked ?? []).join(", ")})</span>
            {mode.rule && <span className="muted small">· {mode.rule}</span>}
          </div>
        )}
        {desc.uniforms.filter((u) => !(linked && u.name === "uWeight")).map((u) => (
          <div className="zoom" key={u.name}>
            <span className="muted small"><code>{u.name}</code> {u.label}</span>
            {u.type === "bool" && (
              <input type="checkbox" checked={values[u.name] === true} onChange={(e) => set(u.name, e.target.checked)} />
            )}
            {(u.type === "int" || u.type === "float") && (
              <>
                <input type="range" min={u.min} max={u.max} step={u.step ?? (u.type === "int" ? 1 : 0.01)}
                  value={Number(values[u.name])} onChange={(e) => set(u.name, Number(e.target.value))} />
                <input type="number" className="small" min={u.min} max={u.max} step={u.step ?? (u.type === "int" ? 1 : 0.01)}
                  value={Number(values[u.name])} onChange={(e) => set(u.name, Number(e.target.value))} style={{ width: "5rem" }} />
              </>
            )}
            {u.type === "ivec3" && ["r", "g", "b"].map((ch, k) => (
              <label key={ch} className="small">
                {ch}
                <input type="range" min={u.min} max={u.max} step={1} value={(values[u.name] as number[])[k]}
                  onChange={(e) => { const a = [...(values[u.name] as number[])]; a[k] = Number(e.target.value); set(u.name, a); }} />
                <span className="small">{(values[u.name] as number[])[k]}</span>
              </label>
            ))}
          </div>
        ))}
      </div>
      <div className="sprite-stage">
        <div className="image-frame">
          <canvas ref={canvas} style={{ imageRendering: "pixelated", width: (texRef.current?.size[0] ?? 0) * zoom, height: (texRef.current?.size[1] ?? 0) * zoom }} />
        </div>
        <div className="small sprite-info">
          <div><b>{desc.title}</b></div>
          <div>status: <code>{status}</code></div>
          {desc.compare && (
            <div>
              {cmp ? (
                <>
                  vs Python render <code>{cmp.file.split("/").pop()}</code>:{" "}
                  {cmp.differing === 0 ? <b style={{ color: "#9ece6a" }}>identical ({cmp.total} px)</b> : <b className="error">{cmp.differing} / {cmp.total} px differ</b>}
                </>
              ) : (
                <span className="muted">no baked render for these uniform values (pick a baked level to diff)</span>
              )}
            </div>
          )}
          <div>source {desc.source.map((c) => <CitationLink key={c} cite={c} />)}</div>
        </div>
      </div>
      {showCode && <pre className="text-viewer">{glsl}</pre>}
    </div>
  );
}
