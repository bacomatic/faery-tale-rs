"""Self-check aid for tools/extract_masks.py (T2.3). Not an acceptance gate."""

import json
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

import extract_masks as em

REPO = Path(__file__).resolve().parents[2]
IMAGE = (REPO / "src" / "assets" / "image").read_bytes()
TERRA_BLOCK, TERRA_BLOCKS = 149, 11  # fmain.c:608; mtrack.c:47 ("terra", 149, 11)


@pytest.fixture(scope="module")
def bundle(tmp_path_factory):
    out = tmp_path_factory.mktemp("masks")
    em.main(["--out-dir", str(out)])
    return out


def test_blocks_from_mtrack():
    first, count, files = em.mask_blocks(REPO / "src")
    assert (first, count) == (896, 24)                     # fmain.c:1222
    assert count * em.BLOCK_SIZE == 8192 + 4096            # SHADOW_SZ, fmain.c:642
    assert [f["name"] for f in files] == ["mask", "mask2", "mask3"]
    assert files[0]["source"] == "src/mtrack.c:48"


def test_decode_round_trip():
    masks = em.decode_masks(IMAGE, 896, 24)
    assert masks.shape == (192, 32, 16)
    assert em.encode_masks(masks) == IMAGE[896 * 512:920 * 512]


@pytest.mark.parametrize("entry", [0, 191])
def test_spot_entries_by_hand(entry):
    """Rebuild one entry word by word from the raw ADF bytes (fsubs.asm:1060, 1071)."""
    masks = em.decode_masks(IMAGE, 896, 24)
    base = 896 * 512 + entry * 64
    for row in range(32):
        word = (IMAGE[base + 2 * row] << 8) | IMAGE[base + 2 * row + 1]
        expect = [(word >> (15 - x)) & 1 for x in range(16)]
        assert masks[entry, row].tolist() == expect


def test_terra_indices_fit():
    """Every mask index the terrain tables can pass to maskit() (byte 0 of a 4-byte terra
    record, fmain.c:2578, 2595) addresses one of the 192 entries."""
    terra = IMAGE[TERRA_BLOCK * 512:(TERRA_BLOCK + TERRA_BLOCKS) * 512]
    indices = set(terra[0::4])
    assert max(indices) < 192


def test_outputs(bundle):
    pngs = sorted(bundle.glob("mask_*.png"))
    assert len(pngs) == 192
    masks = em.decode_masks(IMAGE, 896, 24)
    for i in (0, 1, 100, 191):
        img = Image.open(bundle / f"mask_{i:03d}.png")
        assert img.size == (16, 32) and img.mode == "1"
        assert (np.array(img, dtype=np.uint8) == masks[i]).all()
    sheet = Image.open(bundle / "masks_sheet.png")
    assert sheet.size == (16 * 16, 12 * 32)
    assert (np.array(sheet, dtype=np.uint8)[32:64, 16:32] == masks[17]).all()
    j = json.loads((bundle / "masks.json").read_text())
    assert j["entry_count"] == 192 and (j["width"], j["height"]) == (16, 32)
    assert j["disk"]["first_block"] == 896 and j["disk"]["block_count"] == 24
