### ToDo list

As if having TODO:s in code wasn't enough (and no way in hell I'm using a bug
tracker), here's a damn ToDo list, in no particular order.

* Basic Infrastructure
    * Set up core engine (SDL based)
        * SDL code is piling up in main.rs, it needs to be abstracted out properly
        * System (configuration) state machine
        * Persistent settings for things like window size/loc, etc.
        * Game state machine
    * Set up game data (characters, objects, maps, etc)
        * map data
        * tilesets
    * Core game FSM
    * Implement scenes
        * Static text
        * Storybook, page turning
    * Implement opening sequence
        * Credits
        * Story
        * Make haste but take ...
    * Main viewport setup (play field, scroll, UI buttons, compass)
    * Play view map loading and scrolling
    * Scrolling text output in scroll viewport
* Core Game Mechanics
    * Player movement
    * Player commands (look, give, get, yell, ask, etc)
    * Player terrain effects (blocked, walking in bushes, sinking, etc)
    * NPC behavior (goal, tactic)
    * Player attack
* Audio
    * Sound effects
    * Parse music file, build song list
    * Play selected song
* Graphics Effects
    * viewport based drawing
    * animated/timed effects, e.g., swirly border drawn over time
    * Parse copper lists
    * Day/Night cycle
    * Witch effect
    * Teleport effect
* Persistence
    * Save file support (protobuf)


Completed tasks:
* Basic Infrastructure
    * Set up game data (characters, objects, maps, etc)
        * color palettes
        * font loading
        * placard loading
        * IFF image loading
        * sprite data (cursor)
    * Set mouse cursor
    * Amber font loading
    * Font rendering
    * IFF loading
    * Correct aspect ratio, it's too wide on my 32:9 monitor (requires scaling output rects :sadface:)
    * Implement scenes
        * Swirly bordered text (placards)
* Core Game Mechanics
* Audio
* Graphics Effects
* Persistence
