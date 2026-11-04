# Baritone for Horizon

A Rust port of [Baritone](https://github.com/cabaletta/baritone) (1.21.4) for
[Horizon](https://github.com/horizon-rust/horizon), the Rust rewrite of Minecraft Java Edition 26.3. It
is a Horizon **native mod**: a bundle of Rust source that `horizon-mixins` splices into the game before
it is compiled. The Java mod cannot run in Horizon, so this is written again against Horizon's own
world, collision and input code.

> **Status: close to complete.** [PARITY.md](PARITY.md) lists every class of Baritone 1.21.4 and says which are
> written from the Java, which are simpler, and which are missing. Every setting, command, process and behavior
> is here, `logAsToast` and the beacon beam of `renderGoalXZBeacon` included (the game's own toast and beam; with
> `renderGoalIgnoreDepth` the beam is scaled toward the camera so it shows through terrain). What is not done: the cached
> chunks drawn semitransparent (`renderCachedChunks`), because Horizon gives a mod no renderer of its own.
>
> One deliberate difference from the Java: `#mine` of a block that is not found underground (logs, sand, flowers, crops,
> dirt) walks the surface looking for it instead of digging down to the strip-mining height, which is where the Java goes
> whatever the block is. Ores, ancient debris, the deepslate layer and the like still branch underground as in the Java.

Pinned to Horizon commit `ba6344a59cb9e755b730dcf02285fc0d33b97d3c`. Version 1.1.30 and older are pinned to `142388e` (Nightly 2026-11-01). The mod reads the
game's private state, so a later Nightly may need it ported.

## Using it

Commands are typed in chat with Baritone's prefix, `#`, and never reach the server. `#help` lists them
(click a line for its help), `#baritone` is `#set modified`, `#set list` lists the settings, and TAB
completes commands, settings, block names and properties, files, players and waypoints. Baritone keeps going with the chat or a menu
open. It works on multiplayer servers as the Java does, and most servers ban automation: that choice is yours.

The settings you changed are kept in `horizon-mods/.horizon-mod-data/baritone/settings.txt`, one `name value`
per line as in the Java's `baritone/settings.txt`. Edit it by hand and `#set load` reads it back.

The command names, aliases, descriptions and messages are Baritone 1.21.4's (`baritone.command.defaults`).

## Install

Download `baritone.hnmod` from the [latest release](https://github.com/luhnox/horizon-baritone/releases/latest) and
install it in the launcher's native mods. The launcher builds it into a game of its own (10 to 30 minutes the
first time). It runs native code with your rights: read the source first.

```
horizon-launcher native-build --trust-native-code
```

A `release` build (`horizon-launcher settings native-profile release`) is recommended: the `dev` profile makes the
game's start function too large for the default 2 MB stack (`tools/patch_stack.py` raises it for a `fast` build).
To make the file yourself, `python tools/pack.py` writes `dist/baritone.hnmod`.

## Layout

```
horizon-mixin.toml      what is spliced where: modules (src/) and hook points (hooks/)
horizon-native.json     the registry entry of the .hnmod
src/
  baritone.rs           Baritone and Core: the instance, the tick, the chat, the pathing control
  api/                  goals, settings, utils            (baritone.api.*)
  pathing/              A* search, move generation, costs, the path executor
  behavior/             pathing, look (free look), input, inventory   (baritone.behavior.*)
  process/              mine, follow, farm, explore, build, elytra, ...   (baritone.process.*)
  command/              the commands and their argument parser   (baritone.command.*)
  cache/  selection/  schematic/                          (baritone.cache, .selection, .utils.schematic)
  game/                 what touches the game: the world view, drawing, the bridge into its modules
  lib.rs, tests.rs      a crate of the parts that use no game types, so `cargo test` works on its own
hooks/                  one small function per hook point (chat, tick, keys, tab completion, lines)
tools/                  pack.py (builds the .hnmod), dev-sync.sh (applies it to a Horizon checkout), patch_stack.py
dist/                   where pack.py writes baritone.hnmod (not tracked; releases carry it)
PARITY.md               the class-by-class comparison with Baritone 1.21.4
```

`#[allow(dead_code)]` marks what the Java's API has and nothing in this mod calls, kept so that the API stays whole.

Inside the game every file of `src/` becomes a module of `game.rs` named `baritone_<file>` (see
`horizon-mixin.toml`), so `use super::baritone_goals::...` reads the same in the game and in the test
crate. The `hooks/` files are not modules: each holds exactly one function that the mixin tool inserts at
the start of a game function.

## Working on it

```
cargo test                  # the pure parts: goals, costs, A*, settings, selections, schematics, waypoints, arguments
python tools/pack.py        # rebuild dist/baritone.hnmod from horizon-mixin.toml
```

To check against the game itself, `tools/dev-sync.sh` applies the bundle to a copy of the pinned Horizon
source (it needs `cargo` on `PATH`, `HORIZON_WORK` and `HORIZON_LAUNCHER`), and a normal
`cargo build -p horizon-client --features horizon-client/mods` in that copy builds a game to try it in.

Rules of this code, learned the hard way on Horizon:

- A hook function holds one function and nothing else; keep it short and put the work in `src/`.
- A panic ends the whole game, so fallible calls return `Result` and the commands answer with a
  message instead of crashing.
- The instance is taken out of its cell for the length of a call (`baritone::with`); never keep a
  borrow of it across a call back into the game.
- Blocks are `Pos = [i32; 3]`; the player's own position is `[f64; 3]`.

## License

Baritone is LGPL-3.0 and this is a derivative of it, so it is LGPL-3.0 as well. See [NOTICE.md](NOTICE.md).
