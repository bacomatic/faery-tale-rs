# Narrative text

Directory: `text/`. Every string the game prints, parsed from `src/narr.asm` (the `dc.b` tables)
and, for the riddle answers, `src/fmain2.c:1306-1307`. Strings are ASCII as stored; `%` is the
placeholder the game replaces with the current brother's name (`extract()`, `src/fmain2.c:514-548`,
names `datanames`, `src/fmain.c:604`). No string contains `$`.

```
text/
  event_msg.json  place_msg.json  inside_msg.json  speeches.json    string lists
  place_tbl.json  inside_tbl.json                                  code → message lookup tables
  question.json                                                    the 8 copy-protection riddles + answers
  placard_text.json                                                positioned text of the full-screen placards
  previews/placard_NN_<name>.png                                   the 12 placard cards rendered (non-authoritative)
```

## String lists — `event_msg`, `place_msg`, `inside_msg`, `speeches`

```json
{"name": "_event_msg", "source": "src/narr.asm", "line": 11, "lines": "11-57",
 "strings": ["% was getting rather hungry.", "..."]}
```

| List | Count | Printed by |
|---|---|---|
| `event_msg` | 39 | `event(n)` — hunger, fatigue, time of day, pick-ups, deaths… |
| `place_msg` | 27 | `place_message` when the hero enters an outdoor place (via `place_tbl`) |
| `inside_msg` | 23 | the same indoors (via `inside_tbl`) |
| `speeches` | 61 | `speak(n)` — NPC dialogue |

`strings[i]` is message `i`. Empty strings are real entries (`place_msg[0..1]`, `inside_msg[0..1]`,
`speeches[52]` are `""` — `dc.b 0` slots with no text). `line`/`lines` locate the table in `narr.asm`.

The game wraps at 37 columns on spaces (or a `\r` in the string) and prints with Amber/9;
`extract()` above is the exact rule.

## Lookup tables — `place_tbl`, `inside_tbl`

```json
{"name": "_place_tbl", "fields": ["lo", "hi", "msg_index", "comment", "line"],
 "semantics": "Rows are scanned in order; the first row with lo <= code <= hi selects msg_index into the matching *_msg list (0 = no message).",
 "rows": [{"lo": 51, "hi": 51, "msg_index": 19, "comment": "small keep", "line": 87}, ...]}
```

29 rows map outdoor place codes to `place_msg`, 37 rows map indoor codes to `inside_msg`.
`comment` is the assembler comment on the row (what the place is).

## Riddles — `question.json`

`rows[]`: `{index, label, line, question, answer}` (8 rows, `q1`…`q8`). `copy_protect_junk`
picks `j = rand8()`, shows `question[j]` and compares the typed text with `answer[j]` (uppercase;
`semantics`, `answers_source`).

## Placards — `placard_text.json`

The full-screen story cards (intro, deaths, set-outs, game over, victory, rescues, departure,
copy-protection lead-in). `entries[]` (20): `{index, label, line, segments[]}` where each segment is
`{text, x_half, x, y}`: the text run starts at pixel `(x, y)` = `(x_half*2, y)` with `y` the
baseline, as `_ssp` decodes the `XY` marker (`src/fsubs.asm:497-536`). Segments with `x`/`y` `null`
have no marker and continue at the current pen position — the game prints the brother's name
in between (`name()`), e.g. `placard_text(8+i); name(); placard_text(9+i)` (`src/fmain2.c:1588`).
`format` documents the encoding.

The cards are drawn on the 320×200 page in Amber/9 with pen 24 inside the `placard()` meander
border; the copy-protection lead-in (entry 19) is topaz-8, white on `0x006`, no border. The
`previews/` PNGs show the 12 cards as the call sites compose them (`src/fmain.c:2859-2879`,
`src/fmain2.c:1586-1591`, `1607`, `src/fmain.c:1235`), with "Julian" spliced in for `name()`.
