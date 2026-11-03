// baritone.Baritone and BaritoneProvider: the instance. One `Baritone` per client, holding the shared
// state (`Core`) and the processes. The hooks in tick.rs, chat.rs, command.rs, list.rs, settings.rs and
// click.rs call the functions at the bottom; each takes the instance out of its cell for the length of
// the call, so nothing is ever borrowed twice (the first version borrowed its state while logging and
// brought the game down on the first command).
use super::baritone_aim::AimProcessor;
use super::baritone_command as commands;
use super::baritone_gworld::{on_server, player_pos, WorldCache};
use super::baritone_legacy_helper::Cell;
use super::baritone_input::{player_took_over, Input};
use super::baritone_inventory as inv;
use super::baritone_p_build::BuilderProcess;
use super::baritone_p_elytra::ElytraProcess;
use super::baritone_p_explore::ExploreProcess;
use super::baritone_p_farm::FarmProcess;
use super::baritone_p_follow::FollowProcess;
use super::baritone_p_getto::GetToBlockProcess;
use super::baritone_p_goal::CustomGoalProcess;
use super::baritone_p_mine::MineProcess;
use super::baritone_pathing::{DigState, PathingCommand, Process};
use super::baritone_pathing_behavior::PathingBehavior;
use super::baritone_selection::SelectionManager;
use super::baritone_settings::{self as settings, Settings};
use super::baritone_utils::{feet_of, Pos};
use super::baritone_waypoints::{Tag, Waypoint, Waypoints};
use super::Screen;
use super::Game;
use horizon_client::{
    chat::ChatSource,
    text::{self, Text},
};
use std::{
    cell::{Cell as FlagCell, RefCell},
    collections::HashMap,
};

pub const ID: &str = "baritone";
pub const VERSION: &str = "1.1.15";
pub const DESCRIPTION: &str =
    "A Rust port of Baritone for Horizon: #goto, #mine, #farm, #build, #explore, #follow, #tunnel, #elytra and the rest, with bridging, pillaring, parkour, doors and water bucket landings.";

/// A water bucket landing in progress.
pub struct Mlg {
    ground: Pos,
    placed: bool,
    since: u64,
    next: u64,
    tries: u32,
    /// The tick of the last click: the world takes a moment to show what it did.
    clicked: u64,
}

/// Everything the processes and commands share.
pub struct Core {
    pub settings: Settings,
    pub input: Input,
    pub pathing: PathingBehavior,
    /// LookBehavior's aim processor: the noise and the mouse steps of every aim.
    pub aim: AimProcessor,
    /// LookBehavior: how a rotation a movement asked for is shown, and what the server was last sent.
    pub look: super::baritone_look::LookBehavior,
    /// The cached world of the world and dimension being played in.
    pub world_data: super::baritone_world_data::WorldProvider,
    /// The game's block states as the pathfinder's, kept between ticks.
    pub states: std::rc::Rc<super::baritone_state_factory::StateFactory>,
    /// What the process asking for a path overrides of the calculation context (PathingCommandContext).
    pub path_hooks: Option<std::rc::Rc<dyn super::baritone_calculation_context::CostHooks>>,
    pub dig: DigState,
    pub cache: WorldCache,
    pub selections: SelectionManager,
    /// The first corner of a selection being made (#sel pos1).
    pub sel_pos1: Option<Pos>,
    /// What #sel copy took, and where the copy's corner lies from the place it was taken at.
    pub clipboard: Option<std::rc::Rc<dyn super::baritone_schematic::Schematic>>,
    pub clipboard_offset: Pos,
    /// Waypoints that were deleted or cleared, for #wp restore.
    pub deleted_waypoints: Vec<Waypoint>,
    pub waypoints: Waypoints,
    /// The world the waypoints in `waypoints` belong to.
    pub waypoints_world: Option<String>,
    pub cells: HashMap<u32, Cell>,
    pub avoid_breaking: Vec<String>,
    pub throwaway: Vec<String>,
    pub mlg: Option<Mlg>,
    /// `#pause`.
    pub user_paused: bool,
    was_dead: bool,
    block_names: Option<Vec<String>>,
}

impl Core {
    fn new(game: &Game<'_>) -> Self {
        let settings = Settings::load(|name| game.native_setting(ID, name));
        let waypoints = Waypoints::from_text("");
        let mut core = Self {
            settings,
            input: Input::default(),
            pathing: PathingBehavior::new(),
            aim: AimProcessor::new(super::baritone_command::now_ms()),
            look: Default::default(),
            world_data: Default::default(),
            states: std::rc::Rc::new(super::baritone_state_factory::StateFactory::new(game)),
            path_hooks: None,
            dig: DigState::default(),
            cache: WorldCache::default(),
            selections: SelectionManager::default(),
            sel_pos1: None,
            clipboard: None,
            clipboard_offset: [0, 0, 0],
            deleted_waypoints: Vec::new(),
            waypoints,
            waypoints_world: None,
            cells: HashMap::new(),
            avoid_breaking: Vec::new(),
            throwaway: Vec::new(),
            mlg: None,
            user_paused: false,
            was_dead: false,
            block_names: None,
        };
        core.refresh();
        core
    }

    /// The values worked out from settings, after any of them changes.
    pub fn refresh(&mut self) {
        super::baritone_goals::set_cost_heuristic(self.settings.float("costHeuristic"));
        self.avoid_breaking = self.settings.list("blocksToAvoidBreaking");
        self.throwaway = self.settings.list("acceptableThrowawayItems");
    }

    /// Set a setting and keep it for the next game.
    pub fn set_setting(&mut self, game: &Game<'_>, name: &str, value: &str) -> Result<(&'static settings::Def, String), String> {
        let (def, canonical) = self.settings.set(name, value)?;
        game.native_set_setting(ID, def.name, &canonical);
        self.refresh();
        Ok((def, canonical))
    }

    pub fn reset_setting(&mut self, game: &Game<'_>, def: &'static settings::Def) {
        self.settings.reset(def);
        game.native_set_setting(ID, def.name, def.default);
        self.refresh();
    }

    /// The waypoints file of the world being played in. The Java keeps them with each world; one global list would show a
    /// bed or a death of another world, and #home or #wp goto would walk to coordinates that mean nothing here.
    fn waypoints_file(game: &Game<'_>) -> std::path::PathBuf {
        game.native_data_dir(ID).join("cache").join(super::baritone_world_data::world_name(game)).join("waypoints.txt")
    }

    pub fn save_waypoints(&self, game: &Game<'_>) {
        let file = Self::waypoints_file(game);
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&file, self.waypoints.to_text());
    }

    /// Switch to the waypoints of the world now being played in. The one list older versions kept for every world goes to
    /// the first world opened after the change, once, so nothing is lost.
    pub fn follow_waypoints(&mut self, game: &Game<'_>) {
        let world = super::baritone_world_data::world_name(game);
        if self.waypoints_world.as_deref() == Some(world.as_str()) {
            return;
        }
        self.waypoints_world = Some(world);
        let file = Self::waypoints_file(game);
        let text = match std::fs::read_to_string(&file) {
            Ok(t) => t,
            Err(_) => {
                // A file marks the move as done: a setting that is not one of Baritone's own is not kept between runs.
                let legacy = game.native_setting(ID, "waypoints").unwrap_or_default();
                let marker = game.native_data_dir(ID).join("waypoints_moved");
                if !legacy.trim().is_empty() && !marker.exists() {
                    let _ = std::fs::create_dir_all(game.native_data_dir(ID));
                    let _ = std::fs::write(&marker, "The waypoints older versions kept for every world were moved to the first world opened after that.
");
                    legacy
                } else {
                    String::new()
                }
            }
        };
        self.waypoints = Waypoints::from_text(&text);
        self.deleted_waypoints.clear();
        self.save_waypoints(game);
    }

    /// Every block name the game knows, sorted, for commands to check a name against.
    pub fn known_blocks(&mut self, game: &Game<'_>) -> &Vec<String> {
        self.block_names.get_or_insert_with(|| {
            let mut names: Vec<String> = game.res.facts().states.iter().map(|s| s.name.to_string()).collect();
            names.sort();
            names.dedup();
            names
        })
    }

    /// `stone`, `minecraft:stone` -> `minecraft:stone`, if there is such a block.
    pub fn block_named(&mut self, game: &Game<'_>, word: &str) -> Option<String> {
        let full = if word.contains(':') { word.to_ascii_lowercase() } else { format!("minecraft:{}", word.to_ascii_lowercase()) };
        self.known_blocks(game).binary_search(&full).ok().map(|_| full)
    }

    // -----------------------------------------------------------------------------------
    // Chat: Baritone's look. `[` and `]` dark purple, `Baritone` light purple, the message gray,
    // a failure red.

    fn prefix_text(&self) -> Text {
        let short = self.settings.flag("shortBaritonePrefix");
        let name = if april_first_days() { "Baritoe" } else if short { "B" } else { "Baritone" };
        Text::colored("[", text::DARK_PURPLE)
            .append(Text::colored(name, text::LIGHT_PURPLE))
            .append(Text::colored("]", text::DARK_PURPLE))
            .append(" ")
    }

    pub fn say_text(&self, game: &mut Game<'_>, body: Text) {
        // useMessageTag: the messages are marked by a tag instead of the prefix (Horizon's chat has no tag to draw, so the
        // lines are plain).
        let line = if self.settings.flag("useMessageTag") { body } else { self.prefix_text().append(body) };
        let focused = game.screen == Screen::Chat;
        game.ui.chat.add(&game.gui.atlas, line, None, ChatSource::Client, game.ticks, focused);
    }

    pub fn say(&self, game: &mut Game<'_>, message: &str) {
        self.say_text(game, Text::colored(message, text::GRAY));
    }

    pub fn say_red(&self, game: &mut Game<'_>, message: &str) {
        self.say_text(game, Text::colored(message, text::RED));
    }

    pub fn say_green(&self, game: &mut Game<'_>, message: &str) {
        self.say_text(game, Text::colored(message, text::GREEN));
    }

    /// A line with no prefix, for lists.
    pub fn say_plain(&self, game: &mut Game<'_>, line: Text) {
        let focused = game.screen == Screen::Chat;
        game.ui.chat.add(&game.gui.atlas, line, None, ChatSource::Client, game.ticks, focused);
    }

    // -----------------------------------------------------------------------------------
    // Landing long falls with a water bucket (MLG).

    fn mlg_tick(&mut self, game: &mut Game<'_>, active: bool) {
        let enabled = active && self.settings.flag("allowWaterBucketFall");
        let tick = game.ticks;
        let Some((pos, vy, on_ground, gliding, fell)) = game.player.as_ref().map(|p| {
            ([p.position.x, p.position.y, p.position.z], p.velocity.y, p.ground.on_ground, p.fall_flying || p.flying, p.fall_distance)
        }) else {
            self.mlg = None;
            return;
        };
        if !enabled {
            self.mlg = None;
            return;
        }
        let feet = feet_of(pos);
        if self.mlg.is_none() {
            if gliding || on_ground || fell < 3.5 || vy > -0.5 {
                return;
            }
            let Some(slot) = inv::bucket_slot(game) else {
                return;
            };
            let mut ground = None;
            {
                let g = &*game;
                let mut terrain = super::baritone_gworld::GameTerrain::new(g, &mut self.cells, &self.avoid_breaking, super::baritone_toolset::ToolSettings::of(&self.settings));
                for k in 0..=14 {
                    let c = [feet[0], feet[1] - k, feet[2]];
                    match super::baritone_legacy_helper::Terrain::cell(&mut terrain, c) {
                        Cell::Solid => {
                            ground = Some(c);
                            break;
                        }
                        Cell::Air | Cell::Door => {}
                        // Water, a ladder, lava, a slab...: nothing a bucket would help with.
                        _ => return,
                    }
                }
            }
            let Some(ground) = ground else {
                return;
            };
            if pos[1] - f64::from(ground[1] + 1) > 6.0 {
                return;
            }
            inv::select(game, slot);
            self.mlg = Some(Mlg { ground, placed: false, since: tick, next: tick, tries: 0, clicked: 0 });
        }
        let Some(m) = self.mlg.as_ref() else {
            return;
        };
        if tick.saturating_sub(m.since) > 160 || m.tries > 8 {
            self.mlg = None;
            return;
        }
        let (ground, placed, next, clicked) = (m.ground, m.placed, m.next, m.clicked);
        let above = [ground[0], ground[1] + 1, ground[2]];
        let water_there = {
            let g = &*game;
            let mut terrain = super::baritone_gworld::GameTerrain::new(g, &mut self.cells, &self.avoid_breaking, super::baritone_toolset::ToolSettings::of(&self.settings));
            super::baritone_legacy_helper::Terrain::cell(&mut terrain, above) == Cell::Water
        };
        if !placed {
            if water_there {
                if let Some(m) = self.mlg.as_mut() {
                    m.placed = true;
                    m.next = tick + 1;
                    m.tries = 0;
                }
            } else if on_ground && tick.saturating_sub(clicked) > 15 {
                self.mlg = None;
            } else if tick >= next && !on_ground {
                if let Some(slot) = inv::bucket_slot(game) {
                    inv::select(game, slot);
                }
                let did = game.bar_place(ground, [0, 1, 0]);
                if let Some(m) = self.mlg.as_mut() {
                    m.next = tick + 2;
                    if did {
                        m.tries += 1;
                        m.clicked = tick;
                    }
                }
            }
        } else if !water_there {
            self.mlg = None;
        } else if tick >= next && (on_ground || vy.abs() < 0.2) {
            match inv::empty_bucket_slot(game) {
                Some(slot) => {
                    inv::select(game, slot);
                    let did = game.bar_place(ground, [0, 1, 0]);
                    if let Some(m) = self.mlg.as_mut() {
                        m.next = tick + 3;
                        if did {
                            m.tries += 1;
                            m.clicked = tick;
                        }
                    }
                }
                None => self.mlg = None,
            }
        }
    }
}

/// Every process the instance has.
pub struct Procs {
    pub goal: CustomGoalProcess,
    pub get_to_block: GetToBlockProcess,
    pub mine: MineProcess,
    pub follow: FollowProcess,
    pub explore: ExploreProcess,
    pub farm: FarmProcess,
    pub build: BuilderProcess,
    pub elytra: ElytraProcess,
    pub backfill: super::baritone_p_backfill::BackfillProcess,
    pub pauser: super::baritone_p_backfill::InventoryPauserProcess,
}

impl Procs {
    fn new() -> Self {
        Self {
            goal: CustomGoalProcess::new(),
            get_to_block: GetToBlockProcess::new(),
            mine: MineProcess::new(),
            follow: FollowProcess::new(),
            explore: ExploreProcess::new(),
            farm: FarmProcess::new(),
            build: BuilderProcess::new(),
            elytra: ElytraProcess::new(),
            backfill: super::baritone_p_backfill::BackfillProcess::new(),
            pauser: super::baritone_p_backfill::InventoryPauserProcess::new(),
        }
    }

    /// Every process, in the order they were registered.
    pub fn ordered(&mut self) -> Vec<&mut dyn Process> {
        vec![
            &mut self.goal,
            &mut self.get_to_block,
            &mut self.follow,
            &mut self.elytra,
            &mut self.farm,
            &mut self.explore,
            &mut self.mine,
            &mut self.build,
            &mut self.backfill,
            &mut self.pauser,
        ]
    }

    /// The process with this name.
    pub fn by_name(&mut self, name: &str) -> Option<&mut dyn Process> {
        self.ordered().into_iter().find(|p| p.name() == name)
    }

    pub fn cancel_all(&mut self) {
        for p in self.ordered() {
            p.cancel();
        }
    }

    /// Whether a process is running.
    pub fn any_active(&mut self) -> bool {
        self.ordered().iter().any(|p| p.is_active())
    }
}

pub struct Baritone {
    pub core: Core,
    pub procs: Procs,
    pub in_control: Option<&'static str>,
    /// What the process in control last asked for, for #proc.
    pub last_command: Option<String>,
    /// The active processes, the one that became active last first, then by priority (`active` of the control
    /// manager: ties are broken by which was added to the beginning of the list first).
    active_order: Vec<&'static str>,
}

impl Baritone {
    fn new(game: &Game<'_>) -> Self {
        Self { core: Core::new(game), procs: Procs::new(), in_control: None, last_command: None, active_order: Vec::new() }
    }

    /// Stop everything: every process, the path and the keys.
    pub fn cancel_everything(&mut self, game: &mut Game<'_>) {
        self.procs.cancel_all();
        self.core.pathing_reset(game);
        self.core.mlg = None;
        self.in_control = None;
    }

    /// The name and description of the process in control.
    pub fn describe(&mut self) -> Option<(&'static str, f64, String)> {
        let in_control = self.in_control?;
        self.procs.ordered().into_iter().find(|p| p.name() == in_control).map(|p| (p.name(), p.priority(), p.display()))
    }

    fn tick(&mut self, game: &mut Game<'_>) {
        if game.player.is_none() || game.title.is_some() {
            if self.procs.any_active() || self.core.pathing_busy() {
                self.cancel_everything(game);
            }
            return;
        }
        self.core.cache.note_chunks(game);
        self.core.cache_tick(game);
        self.track_death(game);
        let allow_servers = self.core.settings.flag("allowOnServers");
        if on_server(game) && !allow_servers {
            if self.procs.any_active() {
                self.cancel_everything(game);
                self.core.say_red(game, "Stopped: Baritone is off on servers. #set allowOnServers true turns it on, if the server allows it.");
            }
            return;
        }
        if game.title.is_some() || game.screen == Screen::Dead || game.sign_edit.is_some() || game.book.is_some() {
            // The title, the death screen, a sign or a book: hold still, keep the plan.
            self.core.input.release(game);
            return;
        }
        let active = self.procs.any_active();
        // Typing in the chat presses W, A, S and D: only on the playing screen is that steering.
        if active && self.core.pathing_busy() && game.screen == Screen::Playing && player_took_over(game) {
            self.cancel_everything(game);
            self.core.say_red(game, "Stopped: you took the controls.");
            return;
        }
        self.core.mlg_tick(game, active);
        self.control_tick(game);
    }

    fn track_death(&mut self, game: &mut Game<'_>) {
        let dead = game.screen == Screen::Dead;
        if dead && !self.core.was_dead && self.core.settings.flag("doDeathWaypoints") {
            if let Some(p) = player_pos(game) {
                let wp = Waypoint {
                    tag: Tag::Death,
                    name: "death".into(),
                    pos: feet_of(p),
                    dimension: dimension_of(game),
                    created: super::baritone_command::now_ms(),
                };
                self.core.waypoints.add(wp);
                self.core.save_waypoints(game);
            }
        }
        self.core.was_dead = dead;
    }

    /// baritone.utils.PathingControlManager.executeProcesses and preTick/postTick: ask the active
    /// processes, highest priority first; the first with something to say is in control and the rest are
    /// told they lost it. The pathing behavior then does what that process asked.
    fn control_tick(&mut self, game: &mut Game<'_>) {
        let Baritone { core, procs, in_control, last_command, active_order } = self;
        let last_tick = *in_control;
        let mut priorities: Vec<(&'static str, f64, bool, bool)> = Vec::new();
        for p in procs.ordered() {
            p.refresh(core, game);
            priorities.push((p.name(), p.priority(), p.is_active(), p.is_temporary()));
        }
        // executeProcesses: a newly active process goes to the very front of the queue, a process that is not
        // active leaves it; the queue is then sorted by priority (a stable sort).
        for (name, _, active, _) in &priorities {
            if *active {
                if !active_order.contains(name) {
                    active_order.insert(0, name);
                }
            } else {
                active_order.retain(|n| n != name);
            }
        }
        let priority_of = |name: &str| priorities.iter().find(|p| p.0 == name).map_or(0.0, |p| p.1);
        active_order.sort_by(|a, b| priority_of(b).total_cmp(&priority_of(a)));
        let temporary_of = |name: &str| priorities.iter().find(|p| p.0 == name).is_some_and(|p| p.3);

        let calc_failed = core.pathing.calc_failed_last_tick();
        let safe = core.pathing_safe_to_cancel();
        let mut command: Option<PathingCommand> = None;
        let mut now: Option<&'static str> = None;
        let queue = active_order.clone();
        for (i, name) in queue.iter().enumerate() {
            let Some(p) = procs.by_name(name) else { continue };
            core.path_hooks = None;
            let cmd = p.on_tick(core, game, last_tick == Some(*name) && calc_failed, safe);
            if matches!(cmd, PathingCommand::Defer) {
                core.path_hooks = None;
                continue;
            }
            now = Some(*name);
            command = Some(cmd);
            // A temporary process leaves the others as they are; any other takes control from them.
            if !temporary_of(name) {
                for rest in &queue[i + 1..] {
                    if let Some(q) = procs.by_name(rest) {
                        q.on_lost_control(core, game);
                    }
                }
            }
            break;
        }
        *in_control = now;
        *last_command = command.as_ref().map(PathingCommand::describe);
        // Control changed from a real process to another one that wants to do something.
        let changed = now != last_tick && last_tick.is_some_and(|l| !temporary_of(l)) && command.as_ref().is_some_and(|c| !matches!(c, PathingCommand::RequestPause));
        let lines = core.path_tick(game, command.as_ref(), changed);
        for line in lines {
            core.say(game, &line);
        }
    }
}

/// `Helper.getPrefix`'s April fools: the first three days of April, by the clock of the machine.
fn april_first_days() -> bool {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let _ = era;
    month == 4 && day <= 3
}

fn dimension_of(game: &Game<'_>) -> String {
    game.native_player().map_or("overworld", |p| p.dimension).to_owned()
}

// ---------------------------------------------------------------------------------------
// The instance, and the functions the hooks call.

thread_local! {
    static INSTANCE: RefCell<Option<Box<Baritone>>> = const { RefCell::new(None) };
    static BUSY: FlagCell<bool> = const { FlagCell::new(false) };
}

/// Run `f` with the instance, taking it out of its cell for the length of the call. None when it is
/// already in use (the game called back into a hook from inside one).
pub fn with<R>(game: &mut Game<'_>, f: impl FnOnce(&mut Baritone, &mut Game<'_>) -> R) -> Option<R> {
    if BUSY.with(FlagCell::get) {
        return None;
    }
    BUSY.with(|b| b.set(true));
    let mut b = INSTANCE.with_borrow_mut(Option::take).unwrap_or_else(|| Box::new(Baritone::new(game)));
    let r = f(&mut b, game);
    INSTANCE.with_borrow_mut(|slot| *slot = Some(b));
    BUSY.with(|b| b.set(false));
    Some(r)
}

/// `onPlayerUpdate` PRE, just before the player's motion reads its rotation.
pub fn look_pre(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    with(game, |b, game| {
        let Core { look, aim, settings, .. } = &mut b.core;
        let aim_settings = super::baritone_aim::AimSettings::of(settings, f64::from(game.settings.sensitivity));
        let current = super::baritone_rotation::Rotation::new(game.yaw, game.pitch);
        if let Some(actual) = look.pre(current, aim, &aim_settings, settings) {
            game.yaw = actual.yaw;
            game.pitch = actual.pitch.clamp(-90.0, 90.0);
        }
    });
}

/// `onPlayerUpdate` POST, once the tick has been sent to the server.
pub fn look_post(game: &mut Game<'_>) {
    with(game, |b, game| {
        let fall_flying = game.player.as_ref().is_some_and(|p| p.fall_flying);
        let current = super::baritone_rotation::Rotation::new(game.yaw, game.pitch);
        if let Some(shown) = b.core.look.post(current, fall_flying, &b.core.settings) {
            game.yaw = shown.yaw;
            game.pitch = shown.pitch.clamp(-90.0, 90.0);
        }
    });
}

pub fn tick(game: &mut Game<'_>) {
    look_post(game);
    super::baritone_input::back(game);
    if !game.native_window() {
        return;
    }
    with(game, |b, game| {
        b.tick(game);
        publish(b, game);
        publish_tab(b, game);
    });
}

/// What TAB completes: the prefix, the waypoints, the block and mob names, and the function the chat
/// screen asks (registered once).
fn publish_tab(b: &mut Baritone, game: &mut Game<'_>) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| horizon_client::chat::baritone_suggest::set(super::baritone_tab::suggest));
    let names = b.core.waypoints.all.iter().filter(|w| !w.name.is_empty()).map(|w| w.name.clone()).collect();
    super::baritone_tab::publish(&b.core.settings.text("prefix"), names);
    if super::baritone_tab::needs_blocks() {
        let blocks = b.core.known_blocks(game).clone();
        let mobs = game.res.mob_types().keys().cloned().collect();
        super::baritone_tab::publish_names(blocks, mobs);
    }
}

/// The mod was turned off: stop and draw nothing.
pub fn stop(game: &mut Game<'_>) {
    look_post(game);
    if !game.native_window() {
        return;
    }
    with(game, |b, game| {
        if b.procs.any_active() || b.core.pathing_busy() {
            b.cancel_everything(game);
        }
        // The cached world goes to disk when the game stops.
        let Baritone { core, .. } = b;
        core.world_data.save(game, &core.settings);
    });
    horizon_client::gizmos::baritone_lines::set(Vec::new(), Vec::new());
}

fn publish(b: &mut Baritone, game: &Game<'_>) {
    let mut out = super::baritone_render::Canvas::default();
    super::baritone_render::lines(b, game, &mut out);
    horizon_client::gizmos::baritone_lines::set(out.depth, out.top);
}

/// A line typed in chat: true when it was a Baritone command, so the server never sees it.
pub fn chat(game: &mut Game<'_>, message: &str) -> bool {
    with(game, |b, game| {
        let s = &b.core.settings;
        let prefix = s.text("prefix");
        let on = s.flag("chatControl") || (s.flag("chatControlAnyway") && s.flag("prefixControl"));
        if !on || !s.flag("prefixControl") || prefix.is_empty() {
            return false;
        }
        let Some(line) = message.strip_prefix(prefix.as_str()) else {
            return false;
        };
        // A doubled prefix is a way to say the character itself.
        if line.starts_with(prefix.as_str()) {
            return false;
        }
        game.ui.chat.add_recent(message);
        commands::run(b, game, line.trim());
        true
    })
    .unwrap_or(false)
}

/// `@baritone <line>` from a clicked chat line, and `/baritone <line>` typed with a slash.
pub fn command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    let mut words = command.splitn(2, char::is_whitespace);
    let first = words.next().unwrap_or("");
    if first != commands::CLICK && !first.eq_ignore_ascii_case("baritone") && !first.eq_ignore_ascii_case("b") {
        return;
    }
    *ran = true;
    let line = words.next().unwrap_or("").trim().to_owned();
    with(game, |b, game| commands::run(b, game, &line));
}

// ---------------------------------------------------------------------------------------
// Options > Mods.

const PAGE_TOGGLES: &[(&str, &str)] = &[
    ("Allow on servers", "allowOnServers"),
    ("Place blocks (bridge, pillar)", "allowPlace"),
    ("Parkour jumps", "allowParkour"),
    ("Water bucket landings", "allowWaterBucketFall"),
    ("Draw the path", "renderPath"),
];

fn toggle_on(game: &Game<'_>, name: &str) -> bool {
    let default = settings::find(name).map_or("false", |d| d.default);
    game.native_setting(ID, name).unwrap_or_else(|| default.to_owned()) == "true"
}

pub fn page_labels(game: &Game<'_>) -> Vec<String> {
    PAGE_TOGGLES
        .iter()
        .map(|(label, name)| format!("{label}: {}", if toggle_on(game, name) { "ON" } else { "OFF" }))
        .collect()
}

pub fn page_click(game: &mut Game<'_>, button: usize) {
    let Some((_, name)) = PAGE_TOGGLES.get(button) else {
        return;
    };
    let now = if toggle_on(game, name) { "false" } else { "true" };
    game.native_set_setting(ID, name, now);
    // The running instance reads its own copy.
    with(game, |b, game| {
        let _ = b.core.set_setting(game, name, now);
    });
}
