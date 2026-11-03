// baritone.command.defaults: the commands that start a job: mine, pickup, farm, follow, explore,
// explorefilter, blacklist, find, elytra. Names, aliases, descriptions and
// messages are Baritone 1.21.4's.
use super::baritone::Baritone;
use super::baritone_argument::{Args, CommandError, CommandResult};
use super::baritone_cmd_build::resolve_file;
use super::baritone_command::{begin, block_word, blocks, clickable, dimension, feet, here, relative, run_event, Command};
use super::baritone_goals::Goal;
use super::baritone_gworld::{bare, targets};
use super::baritone_p_explore::read_filter;
use super::baritone_p_follow::Filter;
use super::baritone_utils::pos_label;
use super::Game;
use horizon_client::text::{self, Text};
use std::collections::HashSet;

pub const COMMANDS: &[Command] = &[
    Command {
        names: &["mine"],
        short: "Mine some blocks",
        long: &[
            "The mine command allows you to tell Baritone to search for and mine individual blocks.",
            "",
            "The specified blocks can be ores, or any other block.",
            "",
            "Also see the legitMine settings (see #set l legitMine).",
            "",
            "Usage:",
            "> mine <blocks...> - Mine blocks.",
            "> mine <n> <blocks...> - Mine blocks until you have n of their drops.",
            "",
            "Examples:",
            "> mine diamond_ore deepslate_diamond_ore - Mines all diamonds it can find.",
            "> mine 64 diamond_ore - Mines diamond ore until you have 5 diamonds.",
        ],
        alias: "",
        run: mine,
    },
    Command {
        names: &["pickup"],
        short: "Pickup items",
        long: &["Usage:", "> pickup - Pickup anything", "> pickup <item1> <item2> <...> - Pickup certain items"],
        alias: "",
        run: pickup,
    },
    Command {
        names: &["farm"],
        short: "Farm nearby crops",
        long: &[
            "The farm command starts farming nearby plants. It harvests mature crops and plants new ones.",
            "",
            "Usage:",
            "> farm - farms every crop it can find.",
            "> farm <range> - farm crops within range from the starting position.",
            "> farm <range> <waypoint> - farm crops within range from waypoint.",
        ],
        alias: "",
        run: farm,
    },
    Command {
        names: &["follow"],
        short: "Follow entity things",
        long: &[
            "The follow command tells Baritone to follow certain kinds of entities.",
            "",
            "Usage:",
            "> follow entities - Follows all entities.",
            "> follow entity <entity1> <entity2> <...> - Follow certain entities (for example 'skeleton', 'horse' etc.)",
            "> follow players - Follow players",
            "> follow player <username1> <username2> <...> - Follow certain players",
        ],
        alias: "",
        run: follow,
    },
    Command {
        names: &["explore"],
        short: "Explore things",
        long: &[
            "Tell Baritone to explore chunks it does not have cached yet by distance.",
            "If you used explorefilter before this, it will be applied.",
            "",
            "Usage:",
            "> explore - Explore from your current position.",
            "> explore <x> <z> - Explore from the specified X and Z position.",
        ],
        alias: "",
        run: explore,
    },
    Command {
        names: &["explorefilter"],
        short: "Explore chunks from a json",
        long: &[
            "Apply an explore filter before using explore, which tells the explore process which chunks have been explored/not explored.",
            "",
            "The JSON file will follow this format: [{\"x\":0,\"z\":0},...]",
            "",
            "If 'invert' is specified, the chunks listed will be considered NOT explored, rather than explored.",
            "",
            "Usage:",
            "> explorefilter <path> [invert] - Load the JSON file referenced by the specified path. If invert is specified, it must be the literal word 'invert'.",
        ],
        alias: "",
        run: explore_filter,
    },
    Command {
        names: &["blacklist"],
        short: "Blacklist closest block",
        long: &[
            "While going to a block this command blacklists the closest block so that Baritone won't attempt to get to it.",
            "",
            "Usage:",
            "> blacklist",
        ],
        alias: "",
        run: blacklist,
    },
    Command {
        names: &["find"],
        short: "Find positions of a certain block",
        long: &[
            "The find command searches through the loaded world and attempts to find the location of the block.",
            "",
            "Usage:",
            "> find <block> [...] - Try finding the listed blocks",
        ],
        alias: "",
        run: find,
    },
    Command {
        names: &["elytra"],
        short: "elytra time",
        long: &[
            "The elytra command takes off, steers to the goal, fires a firework rocket when it slows down, keeps above the highest thing ahead, glides in, and walks the last stretch.",
            "It needs an elytra worn and, to go far, firework rockets in the hotbar.",
            "",
            "Usage:",
            "> elytra - fly to the most recent goal (set one with #goal, or #goto)",
            "> elytra reset - start again towards the same goal",
            "> elytra repack - forget which chunks were seen",
            "> elytra supported - say whether it can run",
        ],
        alias: "",
        run: elytra,
    },
];

fn mine(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    here(game)?;
    let quantity = args.try_int().unwrap_or(0);
    args.require_min(1)?;
    let mut boms = Vec::new();
    for word in args.rest() {
        boms.push(super::baritone_command::block_optional_meta(b, game, &word)?);
    }
    while args.optional_string().is_some() {}
    begin(b, game)?;
    b.core.say(game, &format!("Mining [{}]", boms.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")));
    let Baritone { core, procs, .. } = b;
    procs.mine.mine(core, game, quantity, Some(super::baritone_bom::BlockOptionalMetaLookup::new(boms)));
    Ok(())
}

fn pickup(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    here(game)?;
    let mut items: Vec<String> = Vec::new();
    while let Some(w) = args.optional_string() {
        let full = if w.contains(':') { w.to_ascii_lowercase() } else { format!("minecraft:{}", w.to_ascii_lowercase()) };
        if game.res.items().get(&full).is_none() {
            return Err(CommandError::new(format!("Expected a valid item, but got \"{w}\".")));
        }
        if !items.contains(&full) {
            items.push(full);
        }
    }
    begin(b, game)?;
    if items.is_empty() {
        b.core.say(game, "Picking up all items");
        b.procs.follow.pickup(Vec::new());
    } else {
        b.core.say(game, "Picking up these items:");
        for i in &items {
            b.core.say(game, i);
        }
        b.procs.follow.pickup(items.iter().map(|i| bare(i).to_owned()).collect());
    }
    Ok(())
}

fn farm(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    args.require_max(2)?;
    let mut range = 0;
    let mut origin = feet(game)?;
    if args.has_at_least(1) {
        range = args.int("a range")?;
    }
    if args.has_at_least(1) {
        let word = args.string("a waypoint")?;
        let dim = dimension(game);
        let found: Vec<_> = b.core.waypoints.lookup(&word, &dim).into_iter().map(|w| w.pos).collect();
        origin = match found.as_slice() {
            [] => return Err(CommandError::new("No waypoints found")),
            [one] => *one,
            _ => return Err(CommandError::new("Multiple waypoints were found")),
        };
    }
    begin(b, game)?;
    b.procs.farm.farm(origin, range);
    b.core.say(game, "Farming");
    Ok(())
}

fn follow(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    here(game)?;
    args.require_min(1)?;
    if args.has_exactly(1) {
        let group = args.one_of(&["entities", "players"], "a group (entities or players)")?;
        begin(b, game)?;
        b.procs.follow.follow(if group == "entities" { Filter::Entities } else { Filter::Players });
        b.core.say(game, &format!("Following all {group}"));
        return Ok(());
    }
    args.require_min(2)?;
    let list = args.one_of(&["entity", "player"], "entity or player")?;
    let words = args.rest();
    while args.optional_string().is_some() {}
    if list == "entity" {
        let mut kinds = Vec::new();
        for w in &words {
            let id = if w.contains(':') { w.to_ascii_lowercase() } else { format!("minecraft:{}", w.to_ascii_lowercase()) };
            if !game.res.mob_types().contains_key(&id) {
                return Err(CommandError::new(format!("Expected a valid entity type, but got \"{w}\".")));
            }
            kinds.push(id);
        }
        begin(b, game)?;
        b.core.say(game, "Following these types of entities:");
        for k in &kinds {
            b.core.say(game, k);
        }
        b.procs.follow.follow(Filter::Kinds(kinds));
    } else {
        // Players, by name: the ones in range now.
        let here_players: Vec<String> = targets(game).into_iter().filter(|t| t.kind == "minecraft:player").filter_map(|t| t.name).collect();
        let named: Vec<String> = words.iter().filter(|w| here_players.iter().any(|p| p.eq_ignore_ascii_case(w))).cloned().collect();
        if named.is_empty() {
            return Err(CommandError::new("No valid entities in range!"));
        }
        begin(b, game)?;
        b.core.say(game, "Following these entities:");
        for n in &named {
            b.core.say(game, n);
        }
        b.procs.follow.follow(Filter::Named(named));
    }
    Ok(())
}

fn explore(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    if args.has_any() {
        args.require_exactly(2)?;
    } else {
        args.require_max(0)?;
    }
    let at = feet(game)?;
    let (x, z) = if args.has_any() {
        let (wx, wz) = (args.string("x")?, args.string("z")?);
        let bad = |w: &str| CommandError::new(format!("Expected a valid coordinate, but got \"{w}\"."));
        (relative(&wx, at[0]).ok_or_else(|| bad(&wx))?, relative(&wz, at[2]).ok_or_else(|| bad(&wz))?)
    } else {
        (at[0], at[2])
    };
    begin(b, game)?;
    b.procs.explore.explore(x, z);
    b.core.say(game, &format!("Exploring from {}", super::baritone_goals::GoalXZ(x, z).label()));
    Ok(())
}

fn explore_filter(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    args.require_max(2)?;
    let file = args.string("a file")?;
    let mut invert = false;
    if args.has_any() {
        let word = args.string("invert")?;
        if word.eq_ignore_ascii_case("invert") {
            invert = true;
        } else {
            return Err(CommandError::new("Expected either \"invert\" or nothing"));
        }
    }
    let chunks = read_filter(&resolve_file(game, &file)).map_err(CommandError)?;
    b.procs.explore.set_filter(chunks, invert);
    b.core.say(game, &format!("Explore filter applied. Inverted: {invert}"));
    Ok(())
}

fn blacklist(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    args.require_max(0)?;
    if !b.procs.get_to_block.running() {
        return Err(CommandError::new("GetToBlockProcess is not currently active"));
    }
    let Baritone { core, procs, .. } = b;
    if procs.get_to_block.blacklist_closest(core, game) {
        b.core.say(game, "Blacklisted closest instances");
        Ok(())
    } else {
        Err(CommandError::new("No known locations, unable to blacklist"))
    }
}

fn find(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    args.require_min(1)?;
    let at = feet(game)?;
    let mut found = Vec::new();
    for w in args.rest() {
        let name = block_word(b, game, &w)?;
        let set: HashSet<String> = [name].into_iter().collect();
        found.extend(b.core.cache.locations_of(game, &set, at, 160, Some((horizon_client::chunk::MIN_Y, 320)), 16));
    }
    while args.optional_string().is_some() {}
    if found.is_empty() {
        b.core.say(game, "No positions known, are you sure the blocks are in the loaded world?");
        return Ok(());
    }
    for p in found {
        let text = format!("{} {} {}", p[0], p[1], p[2]);
        let line = clickable(&pos_label(p), text::GRAY, run_event(&format!("goal {text}")), Text::colored("Click to set goal to this position", text::GRAY));
        b.core.say_plain(game, line);
    }
    Ok(())
}

fn elytra(b: &mut Baritone, game: &mut Game<'_>, args: &mut Args) -> CommandResult {
    if args.has_exactly(1) && args.peek() == Some("supported") {
        let ok = game.glider_worn();
        b.core.say(game, if ok { "yes" } else { "no: wear an elytra first" });
        return Ok(());
    }
    here(game)?;
    if !args.has_any() {
        if !game.glider_worn() {
            return Err(CommandError::new("Wear an elytra first."));
        }
        let goal = b.procs.goal.most_recent_goal().cloned().ok_or_else(|| CommandError::new("No goal has been set"))?;
        let (x, z) = goal.column().ok_or_else(|| CommandError::new("The goal has no place to fly to."))?;
        if !b.core.settings.flag("elytraTermsAccepted") {
            b.core.say(game, "This elytra is simple: a straight flight at a safe height with firework rockets for speed, then a glide in. It is not Baritone's Nether pathfinder, so it will not thread a canyon or know the seed. #set elytraTermsAccepted true hides this message.");
        }
        begin(b, game)?;
        if super::baritone_inventory::firework_slot(game).is_none() {
            b.core.say(game, "No firework rockets in the hotbar: it will only glide.");
        }
        b.procs.elytra.fly_to(x, z, game.ticks);
        return Ok(());
    }
    let action = args.string("an action")?;
    match action.as_str() {
        "reset" => {
            if !b.procs.elytra.restart(game.ticks) {
                return Err(CommandError::new("Not flying anywhere"));
            }
            b.core.say(game, "Reset state but still flying to same goal");
        }
        "repack" => {
            b.core.cache.forget();
            b.core.say(game, "Queued all loaded chunks for repacking");
        }
        _ => return Err(CommandError::new("Invalid action")),
    }
    Ok(())
}
