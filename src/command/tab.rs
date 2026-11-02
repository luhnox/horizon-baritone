// baritone.api.command.argument.TabCompleteHelper and each command's tabComplete: what the chat box
// offers when TAB is pressed on a `#line`. The game's chat screen asks `suggest` through the bridge.
use super::baritone_command::registry;
use super::baritone_settings::{Kind, DEFS};
use super::baritone_waypoints::Tag;
use std::cell::RefCell;

#[derive(Default)]
struct Data {
    prefix: String,
    blocks: Vec<String>,
    mobs: Vec<String>,
    waypoints: Vec<String>,
}

thread_local! {
    static DATA: RefCell<Data> = RefCell::new(Data::default());
}

/// What the completions read, refreshed by the tick.
pub fn publish(prefix: &str, waypoints: Vec<String>) {
    DATA.with_borrow_mut(|d| {
        d.prefix = prefix.to_owned();
        d.waypoints = waypoints;
    });
}

pub fn needs_blocks() -> bool {
    DATA.with_borrow(|d| d.blocks.is_empty())
}

pub fn publish_names(blocks: Vec<String>, mobs: Vec<String>) {
    DATA.with_borrow_mut(|d| {
        d.blocks = blocks;
        d.mobs = mobs;
    });
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// The text after the prefix up to the cursor in, the byte where its last word starts and the
/// candidates for it out; None when the line is not for Baritone.
pub fn suggest(line: &str) -> Option<(usize, Vec<String>)> {
    let (prefix, blocks, mobs, waypoints) = DATA.with_borrow(|d| (d.prefix.clone(), d.blocks.clone(), d.mobs.clone(), d.waypoints.clone()));
    if prefix.is_empty() {
        return None;
    }
    let rest = line.strip_prefix(prefix.as_str())?;
    let words: Vec<&str> = rest.split(' ').collect();
    let last = words.len() - 1;
    let start = line.len() - words[last].len();
    if last == 0 {
        let mut names: Vec<String> = registry().iter().flat_map(|c| c.names.iter().map(|n| (*n).to_owned())).collect();
        names.sort();
        names.dedup();
        return Some((start, names));
    }
    let command = words[0].to_ascii_lowercase();
    let command = registry().iter().find(|c| c.names.iter().any(|n| *n == command)).map_or(command.clone(), |c| if c.alias.is_empty() { c.names[0].to_owned() } else { c.alias.split(' ').next().unwrap_or("").to_owned() });
    let args = &words[1..];
    let n = args.len();
    let found = match command.as_str() {
        "help" if n == 1 => registry().iter().flat_map(|c| c.names.iter().map(|s| (*s).to_owned())).collect(),
        "set" => set_candidates(args),
        "mine" | "find" | "goto" | "blacklist" if n >= 1 && (command != "goto" || n == 1) => blocks,
        "sel" => sel_candidates(args, &blocks),
        "waypoints" => waypoint_candidates(args, &waypoints),
        "follow" => match args[0].to_ascii_lowercase().as_str() {
            _ if n == 1 => strings(&["entities", "players", "entity", "player"]),
            "entity" => mobs,
            _ => Vec::new(),
        },
        "elytra" if n == 1 => strings(&["reset", "repack", "supported"]),
        "explorefilter" if n == 2 => strings(&["invert"]),
        _ => Vec::new(),
    };
    Some((start, found))
}

fn set_candidates(args: &[&str]) -> Vec<String> {
    let n = args.len();
    if n == 1 {
        let mut all: Vec<String> = DEFS.iter().map(|d| d.name.to_owned()).collect();
        all.extend(strings(&["list", "modified", "reset", "toggle", "save"]));
        return all;
    }
    match args[0].to_ascii_lowercase().as_str() {
        "reset" if n == 2 => {
            let mut all: Vec<String> = DEFS.iter().map(|d| d.name.to_owned()).collect();
            all.push("all".to_owned());
            all
        }
        "toggle" if n == 2 => DEFS.iter().filter(|d| d.kind == Kind::Bool).map(|d| d.name.to_owned()).collect(),
        first if n == 2 => match DEFS.iter().find(|d| d.name.eq_ignore_ascii_case(first)) {
            Some(d) if d.kind == Kind::Bool => strings(&["true", "false"]),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn sel_candidates(args: &[&str], blocks: &[String]) -> Vec<String> {
    let n = args.len();
    if n == 1 {
        return strings(&[
            "pos1", "p1", "1", "pos2", "p2", "2", "clear", "c", "undo", "u", "set", "fill", "s", "f", "walls", "w", "shell", "shl", "sphere", "sph",
            "hsphere", "hsph", "cylinder", "cyl", "hcylinder", "hcyl", "cleararea", "ca", "replace", "r", "expand", "ex", "copy", "cp", "paste", "p",
            "contract", "ct", "shift", "sh",
        ]);
    }
    match args[0].to_ascii_lowercase().as_str() {
        "set" | "fill" | "s" | "f" | "walls" | "w" | "shell" | "shl" | "sphere" | "sph" | "hsphere" | "hsph" if n == 2 => blocks.to_vec(),
        "cylinder" | "cyl" | "hcylinder" | "hcyl" => match n {
            2 => blocks.to_vec(),
            3 => strings(&["x", "y", "z"]),
            _ => Vec::new(),
        },
        "replace" | "r" => blocks.to_vec(),
        "expand" | "ex" | "contract" | "ct" | "shift" | "sh" => match n {
            2 => strings(&["all", "a", "newest", "n", "oldest", "o"]),
            3 => strings(&["down", "up", "north", "south", "west", "east"]),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn waypoint_candidates(args: &[&str], waypoints: &[String]) -> Vec<String> {
    let n = args.len();
    if n == 1 {
        return strings(&["list", "get", "l", "clear", "c", "save", "s", "info", "show", "i", "delete", "d", "restore", "goal", "g", "goto"]);
    }
    let tags: Vec<String> = Tag::ALL.iter().flat_map(|t| t.names().iter().map(|s| (*s).to_owned())).collect();
    match args[0].to_ascii_lowercase().as_str() {
        "list" | "get" | "l" | "clear" | "c" | "save" | "s" if n == 2 => tags,
        "restore" => Vec::new(),
        _ if n == 2 => {
            let mut all = tags;
            all.extend(waypoints.iter().cloned());
            all
        }
        _ => Vec::new(),
    }
}
