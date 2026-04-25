
# The Faery Tale Adventure - Rust

This is a Rust port of "The Faery Tale Adventure" published by MicroIllusions
on the Amiga in 1987.

This project is an exercise in learning Rust and agentic development. Please
do not expect anything from it. This is a personal project, a labor of love
really. Especially do not expect updates in a timely manner.

# Canonical Sources

- Build/run commands and developer setup: this file (`README.md`)
- Agent constraints and working contract: `AGENTS.md`
- Reference/source-of-truth docs (on the `research` branch — fetch on demand): [`reference/RESEARCH.md`](https://github.com/bacomatic/faery-tale-rs/blob/research/reference/RESEARCH.md), [`reference/ARCHITECTURE.md`](https://github.com/bacomatic/faery-tale-rs/blob/research/reference/ARCHITECTURE.md), [`reference/STORYLINE.md`](https://github.com/bacomatic/faery-tale-rs/blob/research/reference/STORYLINE.md). See `AGENTS.md` § "Reference docs (remote)" for the full inventory and fetch recipe.
- Implementation contract: `docs/SPECIFICATION.md`
- Requirements and user stories: `docs/REQUIREMENTS.md`

# Build

This repository is now developed **from the reference documents on the `research` branch** (see `AGENTS.md`) plus the local `docs/SPECIFICATION.md`. To build locally, clone the repository and run Cargo from the project root.

There will be no releases, nor special efforts to ensure compatibility with every platform. Primary development is done on Linux, but Cargo and SDL2 should make other platforms workable as well.

## Common commands

    $ cargo build
    $ cargo run
    $ cargo run -- --debug --skip-intro # run with a TUI debug console and skip the intro sequence
    $ cargo test

## Linux

Install the required dependencies first (assuming an apt-based system):

    $ sudo apt install rust libsdl2-dev libsdl2-gfx-dev libsdl2-mixer-dev
    $ cargo run

## macOS

This builds and runs on macOS. Install the dependencies with Homebrew:

    $ brew install rust sdl2 sdl2_gfx sdl2_mixer
    $ export LIBRARY_PATH="$LIBRARY_PATH:/opt/homebrew/lib"
    $ cargo run

Place the `LIBRARY_PATH` line in your shell profile and adjust the install path if your Homebrew prefix differs.

## Windows

Not currently a supported development platform.

# Directives

Project goals:
1. Be true to the original game and reproduce its mechanics and presentation as faithfully as practical.
2. Avoid enhancements or bug fixes unless reproducing the original behavior would require disproportionate extra work.
3. Use the checked-in assets and the reference/specification documents (research branch + `docs/`) as the basis for ongoing development.

# License

This project is released under the MIT open source license. You are free to do whatever you want with it.

Historical reverse-engineering work informed the reference documents, but the repository now proceeds from the documentation/specification (reference docs on the `research` branch and `docs/SPECIFICATION.md`) rather than from a checked-in copy of the old source tree.

# Note to developers

PRs will not be accepted; this remains a personal learning project shared publicly for academic interest.

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
