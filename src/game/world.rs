// The game as the pathfinder sees it: what each block state means for walking, the terrain a search
// reads, the hotbar, the entities, and a cache of where blocks are. This is the one place that
// touches the game's world; everything in pathing/ stays free of game types.
use super::baritone_legacy_helper::{Cell, Terrain};
use super::baritone_utils::{up, Pos};
use super::Game;
use horizon_client::blocks::Fluid;
use horizon_core::geometry::Axis;
use horizon_world::registry::BlockStateId;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

fn is_danger(name: &str) -> bool {
    matches!(
        name,
        "minecraft:fire"
            | "minecraft:soul_fire"
            | "minecraft:cactus"
            | "minecraft:magma_block"
            | "minecraft:sweet_berry_bush"
            | "minecraft:wither_rose"
            | "minecraft:powder_snow"
            | "minecraft:cobweb"
            | "minecraft:campfire"
            | "minecraft:soul_campfire"
            | "minecraft:pointed_dripstone"
            | "minecraft:lava_cauldron"
            | "minecraft:bubble_column"
    )
}

fn is_climbable(name: &str) -> bool {
    name == "minecraft:ladder"
        || name == "minecraft:scaffolding"
        || name == "minecraft:vine"
        || name.ends_with("_vines")
        || name.ends_with("_vines_plant")
}

/// A door or gate a click opens: wooden ones, never iron.
pub fn is_door(name: &str) -> bool {
    (name.ends_with("_door") && name != "minecraft:iron_door") || name.ends_with("_fence_gate")
}

/// The game's own answer for one block state: its facts and its collision shape.
pub fn classify(game: &Game<'_>, state: BlockStateId) -> Cell {
    let facts = game.res.facts().get(state);
    let name: &str = &facts.name;
    if facts.fluid == Fluid::Lava || name == "minecraft:lava" {
        return Cell::Lava;
    }
    if is_danger(name) {
        return Cell::Danger;
    }
    if is_climbable(name) {
        return Cell::Climb;
    }
    if is_door(name) {
        // An open one is a doorway; a closed one is opened on the way through.
        return if game.state_string(state).contains("open=true") { Cell::Air } else { Cell::Door };
    }
    if name.ends_with("_stairs") {
        return Cell::Low;
    }
    let shape = game.res.collision().cached_shapes().cached_shape(state);
    match shape {
        Ok(Some(shape)) if !shape.is_empty() => {
            let footprint = [Axis::X, Axis::Z].into_iter().all(|a| shape.min(a) <= 0.0 && shape.max(a) >= 1.0);
            if footprint && shape.max(Axis::Y) >= 1.0 {
                Cell::Solid
            } else if shape.min(Axis::Y) >= 0.0 && shape.max(Axis::Y) <= 0.5 + 1e-6 {
                Cell::Low
            } else {
                Cell::Partial
            }
        }
        Ok(Some(_)) => match facts.fluid {
            Fluid::Water => Cell::Water,
            _ => Cell::Air,
        },
        // A shape that depends on its neighbours (a fence, a wall): not something to walk through.
        _ => Cell::Partial,
    }
}

/// baritone.utils.ToolSet.getStrVsBlock: how much of the block one tick of the best tool at hand breaks
/// (the breaking progress per tick, not rounded to whole ticks), and which hotbar slot that tool is in.
/// None when nothing can break it.
pub fn break_progress(game: &Game<'_>, state: BlockStateId, auto_tool: bool) -> Option<(f64, Option<usize>)> {
    let facts = game.res.facts().get(state);
    if facts.hardness < 0.0 {
        return None;
    }
    if facts.hardness == 0.0 {
        return Some((1.0, None));
    }
    let items = game.res.items();
    let mut best: Option<(f64, Option<usize>)> = None;
    let slots: Vec<usize> = if auto_tool { (0..9).collect() } else { vec![game.selected] };
    // Bare hands, and each stack in the hotbar.
    let candidates = std::iter::once((None, None)).chain(slots.into_iter().map(|i| (game.inventory.get(i).and_then(Option::as_ref), Some(i))));
    for (stack, slot) in candidates {
        let speed = f64::from(items.mining_speed(stack, &facts.name));
        let harvest = !facts.requires_tool || items.correct_for_drops(stack, &facts.name);
        let progress = speed / f64::from(facts.hardness) / if harvest { 30.0 } else { 100.0 };
        if progress > 0.0 && best.is_none_or(|(b, _)| progress > b) {
            best = Some((progress, slot));
        }
    }
    best
}

/// The ticks to break `state` with the best of what is in the hotbar (or the held item alone, when
/// the tool is not chosen), and which slot that is. None for what is not worth breaking: a fluid, or
/// a block no tool at hand can harvest.
pub fn best_tool(game: &Game<'_>, state: BlockStateId, auto_tool: bool) -> Option<(f64, Option<usize>)> {
    let facts = game.res.facts().get(state);
    if facts.block_entity || facts.fluid != Fluid::None {
        return None;
    }
    let (progress, slot) = break_progress(game, state, auto_tool)?;
    // Whole ticks, and a block that requires a tool nothing at hand harvests is skipped, as before.
    if facts.requires_tool && facts.hardness > 0.0 {
        let items = game.res.items();
        let stacks = std::iter::once(None).chain((0..9).map(|i| game.inventory.get(i).and_then(Option::as_ref)));
        if !stacks.into_iter().any(|st| items.correct_for_drops(st, &facts.name)) {
            return None;
        }
    }
    Some(((1.0 / progress).ceil(), slot))
}

pub fn block_name(game: &Game<'_>, p: Pos) -> String {
    game.res.facts().get(game.world.block(p[0], p[1], p[2])).name.to_string()
}

pub fn block_state_text(game: &Game<'_>, p: Pos) -> String {
    game.state_string(game.world.block(p[0], p[1], p[2]))
}

pub fn is_air_at(game: &Game<'_>, p: Pos) -> bool {
    game.res.facts().is_air(game.world.block(p[0], p[1], p[2]))
}

/// Playing on someone else's server. A single-player world is a server of its own here (and so is a
/// world opened to LAN), but not one the rules about automation are about.
pub fn on_server(game: &Game<'_>) -> bool {
    game.remote.as_ref().is_some_and(|r| r.session.address != "local")
}

pub fn loaded(game: &Game<'_>, p: Pos) -> bool {
    game.world.chunks.contains_key(&(p[0] >> 4, p[2] >> 4))
}

/// The world a search reads: each position classified once, each block state once.
pub struct GameTerrain<'a, 'g> {
    game: &'a Game<'g>,
    cells: &'a mut HashMap<u32, Cell>,
    avoid_breaking: &'a [String],
    auto_tool: bool,
    memo: HashMap<Pos, Cell>,
    dig: HashMap<u32, Option<f64>>,
}

impl<'a, 'g> GameTerrain<'a, 'g> {
    pub fn new(game: &'a Game<'g>, cells: &'a mut HashMap<u32, Cell>, avoid_breaking: &'a [String], auto_tool: bool) -> Self {
        Self { game, cells, avoid_breaking, auto_tool, memo: HashMap::new(), dig: HashMap::new() }
    }
}

impl Terrain for GameTerrain<'_, '_> {
    fn cell(&mut self, p: Pos) -> Cell {
        if let Some(&c) = self.memo.get(&p) {
            return c;
        }
        let c = if p[1] < horizon_client::chunk::MIN_Y {
            // Below the world: the void.
            Cell::Lava
        } else if !loaded(self.game, p) {
            Cell::Unloaded
        } else {
            let state = self.game.world.block(p[0], p[1], p[2]);
            let game = self.game;
            *self.cells.entry(state.0 as u32).or_insert_with(|| classify(game, state))
        };
        self.memo.insert(p, c);
        c
    }

    fn dig_cost(&mut self, p: Pos) -> Option<f64> {
        let state = self.game.world.block(p[0], p[1], p[2]);
        let game = self.game;
        let (avoid, auto) = (self.avoid_breaking, self.auto_tool);
        *self.dig.entry(state.0 as u32).or_insert_with(|| {
            let name = game.res.facts().get(state).name.to_string();
            let bare = name.strip_prefix("minecraft:").unwrap_or(&name);
            if avoid.iter().any(|a| a == bare) || bare.ends_with("_bed") || bare.ends_with("shulker_box") {
                return None;
            }
            best_tool(game, state, auto).map(|(ticks, _)| ticks)
        })
    }
}

// ---------------------------------------------------------------------------------------
// The hotbar.

/// The first hotbar slot holding an item `wanted` accepts.
pub fn hotbar_slot(game: &Game<'_>, wanted: impl Fn(&str) -> bool) -> Option<usize> {
    (0..9).find(|&i| game.inventory.get(i).and_then(Option::as_ref).is_some_and(|s| s.count > 0 && wanted(&s.item)))
}

/// How many of an item the whole inventory holds.
pub fn count_items(game: &Game<'_>, wanted: impl Fn(&str) -> bool) -> u32 {
    game.inventory.iter().flatten().filter(|s| wanted(&s.item)).map(|s| s.count).sum()
}

pub fn bare(name: &str) -> &str {
    name.strip_prefix("minecraft:").unwrap_or(name)
}

// ---------------------------------------------------------------------------------------
// Entities.

pub fn player_pos(game: &Game<'_>) -> Option<[f64; 3]> {
    game.player.as_ref().map(|p| [p.position.x, p.position.y, p.position.z])
}

/// A mob (or another player) the follow process can walk after.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub id: u32,
    pub pos: [f64; 3],
    pub yaw: f32,
    pub kind: String,
    pub name: Option<String>,
}

pub fn targets(game: &Game<'_>) -> Vec<Target> {
    game.mobs
        .iter()
        .filter(|m| m.alive())
        .map(|m| Target {
            id: m.id,
            pos: [m.position.x, m.position.y, m.position.z],
            yaw: m.yaw,
            kind: m.kind.id.to_string(),
            name: m.custom_name.clone(),
        })
        .collect()
}

pub fn nearest<'t>(from: [f64; 3], candidates: impl Iterator<Item = &'t Target>) -> Option<&'t Target> {
    candidates.min_by(|a, b| {
        let d = |p: &[f64; 3]| (p[0] - from[0]).powi(2) + (p[1] - from[1]).powi(2) + (p[2] - from[2]).powi(2);
        d(&a.pos).total_cmp(&d(&b.pos))
    })
}

/// Items lying on the ground: where, and what.
pub fn dropped_items(game: &Game<'_>) -> Vec<([f64; 3], String)> {
    game.dropped
        .iter()
        .map(|i| ([i.position.x, i.position.y, i.position.z], i.stack.item.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------------------
// Where blocks are.

/// Looks for blocks without reading the world twice: the chunks seen so far, and the block states a
/// name stands for.
#[derive(Default)]
pub struct WorldCache {
    /// Every chunk column that has been loaded since the game started.
    pub explored: HashSet<(i32, i32)>,
    revision: u64,
    ids: HashMap<String, Rc<HashSet<u32>>>,
}

impl WorldCache {
    /// Remember which chunks are loaded now (only when the set changed).
    pub fn note_chunks(&mut self, game: &Game<'_>) {
        let revision = game.world.chunks.membership_revision();
        if revision != self.revision {
            self.revision = revision;
            for (k, _) in &game.world.chunks {
                self.explored.insert(*k);
            }
        }
    }

    pub fn forget(&mut self) {
        self.explored.clear();
        self.revision = 0;
        self.ids.clear();
    }

    /// Every state id of the blocks called `names`.
    pub fn state_ids(&mut self, game: &Game<'_>, names: &HashSet<String>) -> Rc<HashSet<u32>> {
        let mut key: Vec<&str> = names.iter().map(String::as_str).collect();
        key.sort_unstable();
        let key = key.join(",");
        if let Some(ids) = self.ids.get(&key) {
            return ids.clone();
        }
        let ids: HashSet<u32> = game
            .res
            .facts()
            .states
            .iter()
            .enumerate()
            .filter(|(_, s)| names.contains(&*s.name))
            .map(|(i, _)| i as u32)
            .collect();
        let ids = Rc::new(ids);
        self.ids.insert(key, ids.clone());
        ids
    }

    /// The positions of blocks called `names` within `radius` blocks (horizontally) of `center` and
    /// within `y_range`, nearest first, at most `max`.
    pub fn locations_of(
        &mut self,
        game: &Game<'_>,
        names: &HashSet<String>,
        center: Pos,
        radius: i32,
        y_range: Option<(i32, i32)>,
        max: usize,
    ) -> Vec<Pos> {
        let ids = self.state_ids(game, names);
        if ids.is_empty() {
            return Vec::new();
        }
        let (y_lo, y_hi) = y_range.unwrap_or((center[1] - 48, center[1] + 48));
        let y_lo = y_lo.max(horizon_client::chunk::MIN_Y);
        let mut found: Vec<(i64, Pos)> = Vec::new();
        let (cx0, cx1) = ((center[0] - radius) >> 4, (center[0] + radius) >> 4);
        let (cz0, cz1) = ((center[2] - radius) >> 4, (center[2] + radius) >> 4);
        for cx in cx0..=cx1 {
            for cz in cz0..=cz1 {
                let Some(chunk) = game.world.chunks.get(&(cx, cz)) else {
                    continue;
                };
                for (s, section) in chunk.sections.iter().enumerate() {
                    let base = horizon_client::chunk::MIN_Y + s as i32 * 16;
                    if base + 15 < y_lo || base > y_hi {
                        continue;
                    }
                    if !section.blocks.maybe_has(|v| ids.contains(&v)) {
                        continue;
                    }
                    for y in base.max(y_lo)..=(base + 15).min(y_hi) {
                        for lz in 0..16 {
                            for lx in 0..16 {
                                let (x, z) = (cx * 16 + lx, cz * 16 + lz);
                                if (x - center[0]).abs() > radius || (z - center[2]).abs() > radius {
                                    continue;
                                }
                                if ids.contains(&chunk.get(lx, y, lz)) {
                                    let d = (i64::from(x - center[0])).pow(2) + (i64::from(y - center[1])).pow(2) + (i64::from(z - center[2])).pow(2);
                                    found.push((d, [x, y, z]));
                                }
                            }
                        }
                    }
                }
            }
        }
        found.sort_unstable_by_key(|f| f.0);
        found.truncate(max);
        found.into_iter().map(|f| f.1).collect()
    }
}

/// The highest block in the column at (x, z) at or below `from`, or None when nothing is there or the
/// column is not loaded.
pub fn column_top(game: &Game<'_>, x: i32, z: i32, from: i32) -> Option<i32> {
    if !game.world.chunks.contains_key(&(x >> 4, z >> 4)) {
        return None;
    }
    let low = (from - 120).max(horizon_client::chunk::MIN_Y);
    (low..=from).rev().find(|&y| !game.res.facts().is_air(game.world.block(x, y, z)))
}

/// An ore a player could see without X-ray: one of its faces (or, with `diagonals`, its edges and
/// corners) touches air, water or lava.
pub fn exposed(t: &mut dyn Terrain, p: Pos, diagonals: bool) -> bool {
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let faces = (dx != 0) as i32 + (dy != 0) as i32 + (dz != 0) as i32;
                if faces == 0 || (faces > 1 && !diagonals) {
                    continue;
                }
                let c = t.cell([p[0] + dx, p[1] + dy, p[2] + dz]);
                if matches!(c, Cell::Air | Cell::Water | Cell::Lava | Cell::Climb | Cell::Low | Cell::Danger | Cell::Door) {
                    return true;
                }
            }
        }
    }
    false
}

/// Whether the block above `p` is open to the sky's side: used to find the way out of a cave.
pub fn open_above(t: &mut dyn Terrain, p: Pos) -> bool {
    t.cell(up(p, 1)).free()
}
