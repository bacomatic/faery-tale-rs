import { useEffect, useState } from "react";
import { errText, fileUrl } from "../api";

export function useFetched<T>(assetPath: string, parse: (r: Response) => Promise<T>) {
  const [data, setData] = useState<T | undefined>(undefined);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    fetch(fileUrl(assetPath))
      .then((r) => {
        if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
        return parse(r);
      })
      .then((d) => live && setData(d))
      .catch((e) => live && setErr(errText(e)));
    return () => {
      live = false;
    };
  }, [assetPath]);
  return { data, err };
}
