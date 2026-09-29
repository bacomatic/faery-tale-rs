import { useState, type ComponentType } from "react";
import { fileUrl } from "../api";
import ImageViewer from "./ImageViewer";
import JsonViewer, { JsonTree, type Json } from "./JsonViewer";
import PaletteViewer from "./PaletteViewer";
import SpriteViewer from "./SpriteViewer";
import StringsViewer from "./StringsViewer";
import TableViewer from "./TableViewer";
import { useFetched } from "./useFetched";

type Native = ComponentType<{ data: Json }>;

const NATIVE: Record<string, Native> = {
  palette: PaletteViewer,
  table: TableViewer,
  text: StringsViewer,
};

const IMAGE = /\.(png|gif|jpe?g|webp|bmp)$/i;
const JSON_FILE = /\.json$/i;
const TEXT = /\.(md|txt|glsl|frag|vert|csv)$/i;

function TextViewer({ path }: { path: string }) {
  const { data, err } = useFetched<string>(path, (r) => r.text());
  if (err) return <p className="error">{err}</p>;
  if (data === undefined) return <p className="muted">Loading…</p>;
  return <pre className="text-viewer">{data}</pre>;
}

function NativeJson({ path, View }: { path: string; View: Native }) {
  const { data, err } = useFetched<Json>(path, (r) => r.json());
  const [raw, setRaw] = useState(false);
  if (err) return <p className="error">{err}</p>;
  if (data === undefined) return <p className="muted">Loading…</p>;
  return (
    <>
      <div className="zoom small">
        <button className={raw ? "" : "active"} onClick={() => setRaw(false)}>rendered</button>
        <button className={raw ? "active" : ""} onClick={() => setRaw(true)}>raw JSON</button>
      </div>
      {raw ? <JsonTree data={data} /> : <View data={data} />}
    </>
  );
}

// Images/text dispatch on extension; JSON files use the item's `view` when a native viewer exists.
export default function FileViewer({ path, view }: { path: string; view: string }) {
  let body;
  if (IMAGE.test(path)) body = <ImageViewer path={path} />;
  else if (JSON_FILE.test(path) && view === "sprite") body = <SpriteViewer path={path} />;
  else if (JSON_FILE.test(path))
    body = NATIVE[view] ? <NativeJson path={path} View={NATIVE[view]} /> : <JsonViewer path={path} />;
  else if (TEXT.test(path)) body = <TextViewer path={path} />;
  else body = <a href={fileUrl(path)}>open file</a>;
  return (
    <div className="file">
      <div className="file-name">
        <a href={fileUrl(path)} target="_blank" rel="noreferrer">
          {path}
        </a>
      </div>
      {body}
    </div>
  );
}
