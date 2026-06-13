# Reference Material

Reference material lives on the `research` branch of this same repo. Agents fetch and index it on demand via context-modes `ctx_fetch_and_index`; it is not checked into the porting branches.

## Reference doc inventory

See `reference/README.md` on the research branch for information about research documents and their structure.

## URL prefixes

As a fallback if context-mode is not available, use these URLs:
- Raw (for indexing / fetching content):
  `https://raw.githubusercontent.com/bacomatic/faery-tale-rs/research/reference/`
- Browse (for human-readable links in markdown):
  `https://github.com/bacomatic/faery-tale-rs/blob/research/reference/`

## Search-first workflow

The `research` branch is pre-indexed in context-mode. Always search first:
```
ctx_search(queries: ["<topic>"], source: "research:reference/<path-prefix>")
```

## Fallback fetch recipe (markdown / JSON)
Use only when `ctx_search` returns no relevant results:

```
ctx_fetch_and_index(
  url: "https://raw.githubusercontent.com/bacomatic/faery-tale-rs/research/reference/<path>",
  source: "research:reference/<path>"
)
```
Then use `ctx_search` against the newly indexed content. Reuse the same `source:` label for follow-ups across many docs so results can be filtered cleanly.

Binary assets, for example PNG region maps, `overworld.png`, cannot be FTS-indexed. Fetch with `web_fetch` (raw URL) or `gh api` only when an image is genuinely needed.
