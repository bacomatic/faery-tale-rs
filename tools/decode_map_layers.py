#!/usr/bin/env python3
"""Read the assets/maps index layers (T2.5) back into arrays.

Each map directory holds ``map.json`` plus two greyscale PNGs whose pixel values *are* the data
(``assets/maps/README.md``): ``tiles.png`` (8-bit, original tile id in the map's tileset region)
and ``master.png`` (16-bit, master-atlas tile index; 65535 = none). Importable::

    from decode_map_layers import load_map
    m = load_map("assets/maps/interiors/main_castle")
    m["tiles"][row, col], m["master"][row, col], m["meta"]["entries"]

CLI::

    python tools/decode_map_layers.py assets/maps/dungeons/tombs            # summary
    python tools/decode_map_layers.py assets/maps/overworld --cell 1189 984 # one cell, both ids
    python tools/decode_map_layers.py assets/maps/astral_plane --dump json  # layers as JSON rows
    python tools/decode_map_layers.py assets/maps/interiors/crypt --dump csv --layer master
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

NO_MASTER = 0xFFFF


def read_layer(path: Path) -> np.ndarray:
    """A greyscale index PNG as a 2-D integer array (uint8 or uint16, exactly as stored)."""
    img = Image.open(path)
    if img.mode not in ("L", "I;16", "I;16B", "I"):
        raise ValueError(f"{path}: expected a greyscale index layer, got mode {img.mode}")
    arr = np.array(img)
    return arr.astype(np.uint8 if img.mode == "L" else np.uint16)


def load_map(map_dir: str | Path) -> dict:
    """{'meta': map.json, 'tiles': (h, w) uint8, 'master': (h, w) uint16, 'region': (h, w) uint8}.

    ``region`` is the tileset region of every cell: constant for interiors, and for the overworld
    ``(row // 256) * 2 + col // 1024`` (``map.json.tileset.rule``).
    """
    d = Path(map_dir)
    meta = json.loads((d / "map.json").read_text())
    tiles = read_layer(d / meta["layers"]["tiles"]["file"])
    master = read_layer(d / meta["layers"]["master"]["file"])
    w, h = meta["size_tiles"]
    if tiles.shape != (h, w) or master.shape != (h, w):
        raise ValueError(f"{d}: layer shape {tiles.shape}/{master.shape} != map size {(h, w)}")
    if meta["kind"] == "overworld":
        rows, cols = np.indices((h, w))
        region = ((rows // 256) * 2 + cols // 1024).astype(np.uint8)
    else:
        region = np.full((h, w), meta["tileset"]["region"], dtype=np.uint8)
    return {"meta": meta, "tiles": tiles, "master": master, "region": region}


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("map_dir", type=Path)
    p.add_argument("--cell", nargs=2, type=int, metavar=("COL", "ROW"), help="print one cell")
    p.add_argument("--dump", choices=("json", "csv"), help="write the layer(s) to stdout")
    p.add_argument("--layer", choices=("tiles", "master", "region"), default=None,
                   help="with --dump: only this layer (default: all)")
    args = p.parse_args(argv)

    m = load_map(args.map_dir)
    meta = m["meta"]
    if args.cell:
        col, row = args.cell
        print(json.dumps({"col": col, "row": row, "region": int(m["region"][row, col]),
                          "tile": int(m["tiles"][row, col]), "master": int(m["master"][row, col])}))
        return 0
    if args.dump:
        layers = [args.layer] if args.layer else ["tiles", "master", "region"]
        if args.dump == "json":
            json.dump({k: m[k].tolist() for k in layers}, sys.stdout, separators=(",", ":"))
            sys.stdout.write("\n")
        else:
            for k in layers:
                if len(layers) > 1:
                    sys.stdout.write(f"# {k}\n")
                for row in m[k]:
                    sys.stdout.write(",".join(str(int(v)) for v in row) + "\n")
        return 0
    w, h = meta["size_tiles"]
    print(f"{meta['name']} ({meta['kind']}): {w}x{h} tiles")
    print(f"tileset: {meta['tileset'].get('region', 'per cell')}  distinct tiles: {len(np.unique(m['tiles']))}  "
          f"distinct master: {len(np.unique(m['master']))}  no-master cells: {int((m['master'] == NO_MASTER).sum())}")
    for e in meta.get("entries", []):
        print(f"  entry {e['name']!r} ({e['kind']}) at tile {e['landing_tile']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
