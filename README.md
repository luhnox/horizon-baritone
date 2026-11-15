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
> One deliberate difference from the Java: `#mine` and `#goto <block>` of a block that is not found underground (logs, sand,
> flowers, crops, dirt) walk the surface looking for it instead of digging down to the strip-mining height, which is where the Java goes
> whatever the block is. Ores, ancient debris, the deepslate layer and the like still branch underground as in the Java.

Pinned to Horizon commit `9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0` (Nightly 2026-11-13 `9f42064`). Version 1.1.53 targets this Nightly; earlier mod versions target older Nightlies. The mod reads the
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

A mod is built into the Horizon commit it is pinned to. Published releases are titled `Baritone <version> for Nightly <date> (<7 letters>)` and their files are named
`baritone-<version>-nightly-<date>-<7 letters>.hnmod`, the date and the 7 letters the launcher shows in its Versions tab.

| Nightly in the launcher | Baritone | Download |
|---|---|---|
| 2026-11-13 `9f42064` | 1.1.53 | Build with `python tools/pack.py`: `dist/baritone-1.1.53-nightly-2026-11-13-9f42064.hnmod` |

Install the file in the launcher's native mods and use the same Nightly for the game, so the launcher builds on the source it
already has. The build takes 10 to 30 minutes the first time. It runs native code with your rights: read the source first. A mod
pinned to another Nightly makes the launcher ask before it builds; disable other mods whose hooks no longer exist on this Nightly.

```
horizon-launcher native-build --trust-native-code
```

A `release` build (`horizon-launcher settings native-profile release`) is recommended: the `dev` profile makes the
game's start function too large for the default 2 MB stack (`tools/patch_stack.py` raises it for a `fast` build).
To make the file yourself, `python tools/pack.py` writes `dist/baritone.hnmod`.

### Signing key

The versions in the Horizon registry (horizon-rust/horizon-mods) are signed with a [minisign](https://jedisct1.github.io/minisign/) key
of the author of this port (luhnox). Since 2026-11-05 the key is

```
RWTHPH49xk0RxlR1W1RZwYeuLJjnGPW0U/nElRyWuiDh6S1zz9ey1QTj
```

(`minisign -Vm <file>.hnmod -P <the key>` checks a file). It replaces the key of 1.1.38, whose secret half was lost; the
registry entry is changed by a pull request opened from the GitHub account `luhnox`, the owner the entry names, and this paragraph,
committed to the repository the entry names as its source, is the second place the new key is recorded.

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
`cargo check -p horizon-client --locked` in that copy checks the game integration;
`cargo build -p horizon-client --release` builds a game to try it in. This Horizon revision has no `mods` Cargo feature.
The helper selects the mixin tool revision from `horizon-native.json` (`5a64af938ec8`).

Rules of this code, learned the hard way on Horizon:

- A hook function holds one function and nothing else; keep it short and put the work in `src/`.
- A panic ends the whole game, so fallible calls return `Result` and the commands answer with a
  message instead of crashing.
- The instance is taken out of its cell for the length of a call (`baritone::with`); never keep a
  borrow of it across a call back into the game.
- Blocks are `Pos = [i32; 3]`; the player's own position is `[f64; 3]`.

## License

Baritone is LGPL-3.0 (with an anime exception, see its [LICENSE-Part-2.jpg](LICENSE-Part-2.jpg)) and this is a derivative of it,
so it is LGPL-3.0 as well: [LICENSE](LICENSE), [COPYING](COPYING) and [NOTICE.md](NOTICE.md).
