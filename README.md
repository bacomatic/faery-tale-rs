# The Faery Tale Adventure - Amiga

This repository contains the source code to the original "The Faery Tale Adventure" published
by MicroIllusions on the Amiga in 1987. It was written by me (Talin) over the course of seven
months.

## State of the code

This code was written very early in my programming career, and in a hurry; the code is of generally
poor quality with few comments. I don't remember very much about it, and probably won't be able
to give useful answers to questions about it.

The code is primarily written in Aztec C, with some 68000 assembly.

I don't know whether it would be possible to actually get the game running on some other platform;
but even so the code may have some historical interest.

### Bug fixes by bacomatic

This branch contains bug fixes against the original code.

In no particular order:
- De-K&R'ed the source, it was causing problems
- Save files were attempting to write 80 bytes from map_x but Aztec C is moving global variables around so this was broken. Fixed by explicitly writing each variable separately.
- The -pp arg (in makefile) being passed to cc caused 'char' to be compiled as unsigned, which lead to a number of bugs, replaced with -wn to suppress warnings about implicit char* to UBYTE* conversions. (should just add explicit casts in the code)

Exploits fixed (Sorry, gotta play the game the way it was intended!):
- Attacking the turtle is no longer allowed!
- Heroes can no longer sleep-teleport through locked gates and doors.
- An early exploit allowing infinite looting of an item while paused seems to have been previously fixed.


The makefile has been updated to work with Aztec C 5.2a (at the least). It should properly generate the precompiled header and use it when compiling the .c files. It should also properly compile the .asm files using the Aztec C assembler.

To build with Aztec C 5.2 installed, make sure you run the aztec.sh script to set up your environment, then simply run "make" in the project directory.

## Copyright status

Under U.S. Copyright law, a creator may reclaim the copyright of their work after 35 years,
a process known as "termination of transfer". Accordingly, in 2022 I sent a termination of transfer
notice to Hollyware, Inc., the successors-in-interest to the intellectual property of
MicroIllusions. Unfortunately, they have not responded to my letter or any other inquiries I have
made over the years.

Thus, I cannot say for certain exactly what the copyright status of this code is. However, whatever
rights I do have, I hereby make freely available under an MIT-style permissive license.

## Active Forks

I'm not planning on making any changes to this code, it's purpose is mainly to serve as a historical
reference (so please don't send me PRs). However, a number of folks have forked the code and are
trying to get the game to run in various environments:

* https://github.com/XarkLabs/faery-tale-amiga - an effort to getting the game running on "modern" hardware using SDL.
* https://github.com/tomdionysus/faery-tale-amiga - targeted towards running the game on an Amiga emulator.
