
# The Faery Tale Adventure - Rust

This is a Rust port of "The Faery Tale Adventure" published by MicroIllusions
on the Amiga in 1987.

This project is an exercise in learning Rust. Please do not expect anything
from it. This is a personal project, a labor of love really. Especially do not
expect updates in a timely manner.

# Canonical Sources

- Build/run commands and developer setup: this file (`README.md`)
- Reverse-engineering and asset format notes: `DECODE.md`
- Roadmap/progress and task state: `PLAN.md` and `plan_status.toml`
- Local RAG setup and usage: `RAG.md`
- Agent constraints and working contract: `AGENTS.md`

# Build

To build and run, just clone the repository and run "cargo run" from the root
directory.

There will be no releases, nor efforts to ensure compatibility with other
platforms. Primary development is done on Linux but I see no reason it
wouldn't work on other platforms as Cargo/Rust seem to manage these things
fairly well.

## Linux

This just needs a few dependencies (assuming an apt based system):

    $ sudo apt install rust libsdl2-dev libsdl2-gfx-dev libsdl2-mixer-dev
    $ cargo run

## macOS

This builds and runs on macOS. Install some things:

    $ brew install rust sdl2 sdl2_gfx sdl2_mixer
    $ export LIBRARY_PATH="$LIBRARY_PATH:/opt/homebrew/lib"
    $ cargo run

Place the LIBRARY_PATH line in your .profile, adjust the install path for
homebrew if it's different. Without it, you won't be able to link the SDL2
libs.

## Windows

??? Haven't tried, probably won't try personally.

# Directives

I have goals around this:
1. Be true to the original game, implement (as much as possible) the original
game mechanics.
2. No enhancements or bug fixes, unless I have to go out of my way to
*implement* a bug.
3. Use original assets, as provided.

# License

This project is released under an MIT open source license. You are free to do
whatever you want with it.

The original code this project is based on was written by David "Talin" Joiner,
who very graciously released the source code under an MIT license. This project
is forked from his project on GitHub. The original source has been moved to
a subdirectory and as the project progresses will slowly be butchered to death
as it is deconstructed. Do not expect the original code provided to compile or
be in any sort of working order.

# Note to developers

PRs will not be accepted, please do not submit any. As stated, this is *purely*
a learning project and the source is being posted publicly for academic
purposes only.

## Git hooks

This repository includes a `pre-push` hook in `.githooks/` that runs:

1. `scripts/refresh_issue_map.sh` to regenerate the `Issue Map (Rollups)`
    section in `PLAN.md` from `plan_status.toml`
2. `scripts/plan_sync_check.sh` to validate PLAN/status consistency

Pushes are blocked if either step fails.

Optional maintenance command:

    $ bash scripts/sync_plan_from_github.sh
    $ bash scripts/sync_rollup_issue_states.sh
    $ bash scripts/sync_rollup_issue_states.sh --strict-open

This syncs rollup task states from GitHub issue state (`CLOSED` issues mark
their rollup task as `done`; open issues leave local state unchanged).

`sync_plan_from_github.sh` is the one-liner workflow: strict issue-state sync,
Issue Map refresh, then PLAN/status consistency validation.

With `--strict-open`, open rollup issues force local rollup task state to
`in_progress`.

Enable repo-local hooks once after cloning:

    $ git config core.hooksPath .githooks

## Local RAG helper

RAG setup, script environment variables, and all usage examples are documented in `RAG.md`.

## Common shortcuts

If `make` is available, these shortcuts are provided:

    $ make plan-check
    $ make docs-check
    $ make rag-demo Q="where is page flip handled"
    $ make rag-demo-inc Q="what changed in map rendering"
    $ make sync-issues
    $ make agent-bootstrap

##

    .                         ######                         .
    .                        ########                        .
    .                        #  ##  #                        .
    .                         ##  ##                         .
    .                          ####                          .
    .                       ## #  # ##                       .
    .                         # ## #                         .
    .                         ##  ##                         .
    .                       ##      ##                       .
