#!/usr/bin/env python3
"""Sound-effect extractor for The Faery Tale Adventure (T2.7).

The six PCM effects live in ADF blocks 920-930 (11 blocks, ``SAMPLE_SZ`` = 5632,
``fmain.c:645``; ``load_track_range(920,11,sample_mem,8)`` ``fmain.c:1028``).
``read_sample()`` (``fmain.c:1033-1041``) walks the buffer: each sample is a
4-byte big-endian length followed by that many bytes of 8-bit signed PCM
(Paula's native format). This script walks the same prefixes and writes each
sample byte-exact to ``assets/audio/sfx/sfx_<n>.wav``.

WAV 8-bit PCM is *unsigned*, so every byte is XOR 0x80 (signed -> unsigned
offset binary); no resampling or normalisation. The header sample rate is
**nominal only**: the game plays each effect at a Paula period randomised per
trigger (``effect(num,speed)`` -> ``playsample(sample[num],sample_size[num]/2,
speed)`` ``fmain.c:3616-3619``; ``speed`` lands in AUD2PER ``gdriver.asm:314``).
We use the NTSC Paula clock 3,579,545 Hz / the base period at the trigger call
site; the per-trigger period expressions are written to ``sfx.json``.

Browsers refuse WAVs below 3000 Hz (effect 5's nominal rate is 1989 Hz), so
``previews/`` holds each effect rendered the way Paula outputs it -- every byte
held for ``period`` clock ticks (zero-order hold) -- resampled to 44.1 kHz, one
file per distinct trigger base. Non-authoritative; the ``sfx_<n>.wav`` bytes
are the asset.

Usage::

    python tools/extract_sfx.py            # -> assets/audio/sfx/
"""

from __future__ import annotations

import struct
import sys
import wave
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

import asset_common as ac  # noqa: E402

REPO_ROOT = TOOLS_DIR.parent
BLOCK_SIZE = 512
FIRST_BLOCK, NUM_BLOCKS = 920, 11          # fmain.c:1028
NUM_SAMPLES = 6                            # fmain.c:1014-1015, 1034
NTSC_PAULA_CLOCK = 3_579_545               # Hz (NTSC); period -> rate = clock / period

# Every effect(num, speed) call site in src/ (grep "effect(" over src/*.c).
# rng ranges follow fsubs.asm:308-330 (bitrand(x) = rand() & x, rand64 = rand() & 63,
# rand256 = rand() & 255), so the period is base + [0, rng_max].
TRIGGERS = {
    0: [
        {"expr": "800+bitrand(511)", "base": 800, "rng": "bitrand(511)", "rng_max": 511,
         "source": "src/fmain2.c:240",
         "when": "dohit(): a melee blow lands on the hero (j==0) -- the hero is hit"},
    ],
    1: [
        {"expr": "150+rand256()", "base": 150, "rng": "rand256()", "rng_max": 255,
         "source": "src/fmain.c:2262",
         "when": "melee swing that comes within bv+2 of a target but does not hit (near miss), "
                 "not for touch attacks (wt==5)"},
    ],
    2: [
        {"expr": "500+rand64()", "base": 500, "rng": "rand64()", "rng_max": 63,
         "source": "src/fmain2.c:238",
         "when": "dohit(i==-1): an arrow (missile_type 1, fmain.c:1697, 2293) hits a figure, "
                 "or the witch's effect hits the hero (fmain.c:2375)"},
    ],
    3: [
        {"expr": "400+rand256()", "base": 400, "rng": "rand256()", "rng_max": 255,
         "source": "src/fmain2.c:241",
         "when": "dohit(): a melee blow lands on a non-hero figure (j!=0) -- an enemy is hit"},
    ],
    4: [
        {"expr": "400+rand256()", "base": 400, "rng": "rand256()", "rng_max": 255,
         "source": "src/fmain.c:1680",
         "when": "SHOOT3 with a bow: an arrow is loosed (missile launched, ms->speed=3)"},
    ],
    5: [
        {"expr": "1800 + rand256()", "base": 1800, "rng": "rand256()", "rng_max": 255,
         "source": "src/fmain.c:1488",
         "when": "the dragon breathes fire (rand4()==0; missile_type 2, speed 5)"},
        {"expr": "1800+rand256()", "base": 1800, "rng": "rand256()", "rng_max": 255,
         "source": "src/fmain.c:1690",
         "when": "SHOOT1 with the wand (weapon 5): a wand bolt is fired (missile_type 2)"},
        {"expr": "3200+bitrand(511)", "base": 3200, "rng": "bitrand(511)", "rng_max": 511,
         "source": "src/fmain2.c:239",
         "when": "dohit(i==-2): a missile_type-2 bolt (dragon fire / wand bolt, fmain.c:2292) "
                 "hits a figure"},
    ],
}

# Effect 5 has two bases (1800 at the two launch sites, 3200 at the hit site); the WAV
# header uses 1800, the base of both launch triggers and the lower (faster) pitch.
NOMINAL_BASE = {n: t[0]["base"] for n, t in TRIGGERS.items()}


def read_samples(image: bytes) -> tuple[list[dict], int]:
    """Walk the 6 length-prefixed samples (fmain.c:1033-1041).

    Returns ([{index, offset, length, pcm}], bytes_remaining_after_6th)."""
    buf = image[FIRST_BLOCK * BLOCK_SIZE:(FIRST_BLOCK + NUM_BLOCKS) * BLOCK_SIZE]
    out, pos = [], 0
    for i in range(NUM_SAMPLES):
        (n,) = struct.unpack_from(">I", buf, pos)           # fmain.c:1035-1037
        pcm = buf[pos + 4:pos + 4 + n]                        # fmain.c:1038-1039
        if len(pcm) != n:
            raise SystemExit(f"sample {i}: length {n} overruns the {len(buf)}-byte buffer")
        out.append({"index": i, "offset": pos + 4, "length": n, "pcm": pcm})
        pos += 4 + n                                          # fmain.c:1040
    return out, len(buf) - pos


def write_wav(path: Path, pcm: bytes, rate: int) -> None:
    """8-bit mono WAV; signed Paula bytes -> unsigned WAV bytes by XOR 0x80."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(1)
        w.setframerate(rate)
        w.writeframes(bytes(b ^ 0x80 for b in pcm))


PREVIEW_RATE = 44_100


def render_preview(pcm: bytes, period: int, rate: int = PREVIEW_RATE) -> bytes:
    """Zero-order-hold resample: sample i plays for period/NTSC_PAULA_CLOCK seconds."""
    paula_rate = NTSC_PAULA_CLOCK / period
    n_out = int(len(pcm) * rate / paula_rate)
    return bytes((pcm[min(int(i * paula_rate / rate), len(pcm) - 1)] ^ 0x80) for i in range(n_out))


def write_preview(path: Path, data: bytes, rate: int = PREVIEW_RATE) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(1)
        w.setframerate(rate)
        w.writeframes(data)


def main(argv=None) -> int:
    parser = ac.build_arg_parser("Extract the 6 PCM sound effects to assets/audio/sfx/")
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "assets" / "audio" / "sfx")
    args = parser.parse_args(argv)

    image = (args.game_dir / "image").read_bytes()
    samples, remaining = read_samples(image)

    effects = []
    for s in samples:
        n = s["index"]
        rate = round(NTSC_PAULA_CLOCK / NOMINAL_BASE[n])
        write_wav(args.out_dir / f"sfx_{n}.wav", s["pcm"], rate)
        previews = []
        for base in sorted({t["base"] for t in TRIGGERS[n]}):
            name = f"sfx_{n}.wav" if base == NOMINAL_BASE[n] else f"sfx_{n}_period{base}.wav"
            write_preview(args.out_dir / "previews" / name, render_preview(s["pcm"], base))
            previews.append({"file": f"previews/{name}", "period": base,
                             "rate_hz": round(NTSC_PAULA_CLOCK / base), "wav_rate_hz": PREVIEW_RATE})
        effects.append({
            "effect": n,
            "file": f"sfx_{n}.wav",
            "previews": previews,
            "buffer_offset": s["offset"],
            "byte_length": s["length"],
            "nominal_period": NOMINAL_BASE[n],
            "nominal_rate_hz": rate,
            "triggers": TRIGGERS[n],
        })
        print(f"sfx_{n}.wav  offset {s['offset']:5d}  length {s['length']:5d}  "
              f"period {NOMINAL_BASE[n]:5d} -> {rate} Hz  triggers {len(TRIGGERS[n])}")

    ac.write_json(args.out_dir / "sfx.json", {
        "source": {
            "image_blocks": f"{FIRST_BLOCK}-{FIRST_BLOCK + NUM_BLOCKS - 1}",
            "buffer_bytes": NUM_BLOCKS * BLOCK_SIZE,
            "framing": "4-byte big-endian length prefix + that many bytes of 8-bit signed PCM, "
                       "6 samples back to back; buffer_offset is the offset of the first PCM "
                       "byte within the 5632-byte buffer (prefix excluded)",
            "citations": ["src/fmain.c:645", "src/fmain.c:1028", "src/fmain.c:1033-1041"],
            "bytes_after_last_sample": remaining,
        },
        "format": {
            "wav": "8-bit unsigned mono PCM; each byte = original signed byte XOR 0x80; "
                   "no resampling or normalisation",
            "sample_rate": "nominal only: NTSC Paula clock 3579545 / nominal_period, rounded. "
                           "The game plays each trigger at period = base + rng (see triggers); "
                           "actual rate = 3579545 / period",
            "playback": "effect(num,speed) -> playsample(sample[num], sample_size[num]/2, speed): "
                        "length in words to AUD2LEN, speed to AUD2PER, volume 64, channel 2",
            "citations": ["src/fmain.c:3616-3619", "src/gdriver.asm:296-322",
                          "src/fsubs.asm:308-330"],
            "nominal_period_choice": "effect 5 has bases 1800 (fmain.c:1488, 1690) and 3200 "
                                     "(fmain2.c:239); 1800 is used for the header",
            "previews": "previews/*.wav: the same bytes rendered as Paula plays them at the trigger's "
                        "base period (zero-order hold, i.e. each byte held for period clock ticks), "
                        f"resampled to {PREVIEW_RATE} Hz so every player accepts them (browsers reject "
                        "rates below 3000 Hz; effect 5 is 1989 Hz). Non-authoritative.",
        },
        "effects": effects,
    })
    print(f"bytes after 6th sample: {remaining}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
