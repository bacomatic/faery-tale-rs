
# The Faery Tale Adventure - Rust

This is a Rust port of "The Faery Tale Adventure" published by MicroIllusions
on the Amiga in 1987.

This project is an exercise in learning Rust. Please do not expect anything
from it. This is a personal project, a labor of love really. Especially do not
expect updates in a timely manner.

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


    .                         ######                         .
    .                        ########                        .
    .                        #  ##  #                        .
    .                         ##  ##                         .
    .                          ####                          .
    .                       ## #  # ##                       .
    .                         # ## #                         .
    .                         ##  ##                         .
    .                       ##      ##                       .
