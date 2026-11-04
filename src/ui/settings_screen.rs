// Baritone's settings screen: the categories and what each one shows, where everything stands, and what a click does.
// It takes the place of the page Options > Mods > Baritone (the hooks are in hooks/page_*.rs) and is laid out like the
// Bedrock-style settings screens of Punchy: a title bar with a close button, a column of categories on the left under a
// Done button, and on the right a header and a list of rows, each a title, a description and a switch or a slider.
// Nothing here touches a game type, so it is tested in the plain crate; the drawing is in ui/settings_page.rs.
use super::baritone_settings::{self as settings, Settings};

pub const THROWAWAY: &str = "acceptableThrowawayItems";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Control {
    /// A switch.
    Toggle(&'static str),
    /// A number picked from a fixed list, on a slider.
    Steps(&'static str, &'static [&'static str]),
    /// One block in the list of blocks Baritone may place.
    Block(&'static str),
    /// An address opened in the browser.
    Link(&'static str),
}

pub struct Item {
    pub label: &'static str,
    /// What the row says under its title; empty takes the first sentence of the setting's own description.
    pub desc: &'static str,
    pub control: Control,
}

pub struct Category {
    pub label: &'static str,
    /// The letter on the colour square before its name, and the square's colour (0xRRGGBB).
    pub glyph: &'static str,
    pub color: u32,
    pub items: &'static [Item],
}

const fn toggle(label: &'static str, name: &'static str) -> Item {
    Item { label, desc: "", control: Control::Toggle(name) }
}
const fn steps(label: &'static str, name: &'static str, values: &'static [&'static str]) -> Item {
    Item { label, desc: "", control: Control::Steps(name, values) }
}
const fn block(label: &'static str, name: &'static str) -> Item {
    Item { label, desc: "", control: Control::Block(name) }
}
const fn link(label: &'static str, desc: &'static str, url: &'static str) -> Item {
    Item { label, desc, control: Control::Link(url) }
}

pub const CATEGORIES: &[Category] = &[
    Category {
        label: "Movement",
        glyph: ">",
        color: 0x2E6DB4,
        items: &[
            toggle("Place blocks", "allowPlace"),
            toggle("Break blocks", "allowBreak"),
            toggle("Sprint", "allowSprint"),
            toggle("Parkour jumps", "allowParkour"),
            toggle("Parkour places blocks", "allowParkourPlace"),
            toggle("Parkour ascends", "allowParkourAscend"),
            toggle("Diagonal ascend", "allowDiagonalAscend"),
            toggle("Diagonal descend", "allowDiagonalDescend"),
            toggle("Dig straight down", "allowDownward"),
            toggle("Climb vines", "allowVines"),
            toggle("Water bucket landings", "allowWaterBucketFall"),
            steps("Fall without water", "maxFallHeightNoWater", &["1", "2", "3", "4", "5", "10", "20"]),
        ],
    },
    Category {
        label: "Blocks",
        glyph: "#",
        color: 0x8A5A2B,
        items: &[
            block("Place dirt", "dirt"),
            block("Place grass blocks", "grass_block"),
            block("Place cobblestone", "cobblestone"),
            block("Place stone", "stone"),
            block("Place netherrack", "netherrack"),
            block("Place cobbled deepslate", "cobbled_deepslate"),
            block("Place andesite", "andesite"),
            block("Place sand", "sand"),
            block("Place gravel", "gravel"),
            toggle("Take blocks from the inventory", "allowInventory"),
        ],
    },
    Category {
        label: "Mining",
        glyph: "M",
        color: 0x6B6B6B,
        items: &[
            toggle("Collect dropped items", "mineScanDroppedItems"),
            steps("Wait for drops (ms)", "mineDropLoiterDurationMSThanksLouca", &["0", "100", "250", "500", "1000"]),
            toggle("Explore to find blocks", "exploreForBlocks"),
            toggle("Only exposed ores", "allowOnlyExposedOres"),
            toggle("Legit mine", "legitMine"),
            toggle("Pick the best tool", "autoTool"),
            toggle("Walk while breaking", "walkWhileBreaking"),
            toggle("Pause for falling blocks", "pauseMiningForFallingBlocks"),
            toggle("Keep damaged tools", "itemSaver"),
            steps("Known ore spots", "mineMaxOreLocationsCount", &["16", "32", "64", "128", "256"]),
        ],
    },
    Category {
        label: "Build & Farm",
        glyph: "B",
        color: 0x3F8F3F,
        items: &[
            toggle("Build in layers", "buildInLayers"),
            toggle("Build only the selection", "buildOnlySelection"),
            toggle("Skip what is already there", "buildIgnoreExisting"),
            toggle("Map art mode", "mapArtMode"),
            toggle("Backfill tunnels", "backfill"),
            toggle("Replant crops", "replantCrops"),
            toggle("Replant nether wart", "replantNetherWart"),
            toggle("Wait for crops to grow", "farmWaitForGrowth"),
        ],
    },
    Category {
        label: "Pathing",
        glyph: "P",
        color: 0xB5541C,
        items: &[
            steps("Plan time (ms)", "primaryTimeoutMS", &["250", "500", "1000", "2000"]),
            steps("Plan ahead time (ms)", "planAheadPrimaryTimeoutMS", &["1000", "2000", "4000", "8000"]),
            steps("Block placing cost", "blockPlacementPenalty", &["5", "10", "20", "40", "100"]),
            steps("Block breaking cost", "blockBreakAdditionalPenalty", &["0", "2", "5", "10"]),
            steps("Jump cost", "jumpPenalty", &["0", "1", "2", "4", "8"]),
            toggle("Avoid mobs", "avoidance"),
            toggle("Cancel when the goal is gone", "cancelOnGoalInvalidation"),
            toggle("Walk on cached chunks only", "pathThroughCachedOnly"),
            toggle("Slow path", "slowPath"),
        ],
    },
    Category {
        label: "Rendering",
        glyph: "R",
        color: 0x7A3FA0,
        items: &[
            toggle("Draw the path", "renderPath"),
            toggle("Path as a line", "renderPathAsLine"),
            toggle("Path through walls", "renderPathIgnoreDepth"),
            toggle("Draw the goal", "renderGoal"),
            toggle("Animate the goal", "renderGoalAnimated"),
            toggle("Goal beacon", "renderGoalXZBeacon"),
            toggle("Draw selections", "renderSelectionBoxes"),
            toggle("Show cached chunks", "renderCachedChunks"),
            steps("Path line width", "pathRenderLineWidthPixels", &["1", "2", "3", "5", "8"]),
            steps("Goal line width", "goalRenderLineWidthPixels", &["1", "2", "3", "5", "8"]),
        ],
    },
    Category {
        label: "Chat",
        glyph: "C",
        color: 0x2F8FA8,
        items: &[
            toggle("Commands in chat", "chatControl"),
            toggle("Commands with the prefix", "prefixControl"),
            toggle("Commands in chat anyway", "chatControlAnyway"),
            toggle("Echo commands", "echoCommands"),
            toggle("Short prefix", "shortBaritonePrefix"),
            toggle("Message tag", "useMessageTag"),
            toggle("Messages as toasts", "logAsToast"),
            toggle("Desktop notifications", "desktopNotifications"),
            toggle("Hide coordinates", "censorCoordinates"),
            toggle("Debug messages", "chatDebug"),
        ],
    },
    Category {
        label: "Camera",
        glyph: "O",
        color: 0x1F3F8F,
        items: &[
            toggle("Free look", "freeLook"),
            toggle("Free look in flight", "elytraFreeLook"),
            toggle("Smooth look in flight", "elytraSmoothLook"),
            toggle("Anti-cheat compatibility", "antiCheatCompatibility"),
        ],
    },
    Category {
        label: "Help & Support",
        glyph: "?",
        color: 0xA03F6F,
        items: &[
            link("Baritone Wiki", "How every command and setting works.", "https://github.com/cabaletta/baritone/blob/master/USAGE.md"),
            link("Releases", "Newer versions of Baritone for Horizon.", "https://github.com/luhnox/horizon-baritone/releases"),
            link("Bug Report", "Tell us what went wrong.", "https://github.com/luhnox/horizon-baritone/issues"),
        ],
    },
];

/// The sidebar: a heading, then the categories under it, as the Punchy screen sorts its pages. Indexes into `CATEGORIES`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Entry {
    Heading(&'static str),
    Category(usize),
}

pub const SIDEBAR: &[Entry] = &[
    Entry::Heading("BARITONE"),
    Entry::Category(0),
    Entry::Category(1),
    Entry::Category(2),
    Entry::Category(3),
    Entry::Category(4),
    Entry::Heading("INTERFACE"),
    Entry::Category(5),
    Entry::Category(6),
    Entry::Category(7),
    Entry::Heading("MORE"),
    Entry::Category(8),
];

// ---------------------------------------------------------------------------------------------
// What a control shows.

fn bare(name: &str) -> &str {
    name.trim().trim_start_matches("minecraft:")
}

fn same_number(a: &str, b: &str) -> bool {
    a == b || matches!((a.trim().parse::<f64>(), b.trim().parse::<f64>()), (Ok(x), Ok(y)) if (x - y).abs() < 1e-9)
}

/// On or off, for the controls that are switches.
pub fn is_on(control: Control, s: &Settings) -> Option<bool> {
    match control {
        Control::Toggle(name) => Some(s.flag(name)),
        Control::Block(block) => Some(s.list(THROWAWAY).iter().any(|b| bare(b) == block)),
        Control::Steps(..) | Control::Link(_) => None,
    }
}

/// The value a slider shows, as the setting holds it.
pub fn value(item: &Item, s: &Settings) -> String {
    match item.control {
        Control::Steps(name, _) => s.get(name).to_owned(),
        _ => String::new(),
    }
}

/// Where the slider's knob stands, 0 to 1, or None when the value is not one of its steps.
pub fn position(control: Control, s: &Settings) -> Option<f32> {
    let Control::Steps(name, values) = control else { return None };
    let at = values.iter().position(|v| same_number(v, s.get(name)))?;
    Some(if values.len() < 2 { 0.0 } else { at as f32 / (values.len() - 1) as f32 })
}

/// The line under the title.
pub fn describe(item: &Item) -> String {
    if !item.desc.is_empty() {
        return item.desc.to_owned();
    }
    match item.control {
        Control::Toggle(name) | Control::Steps(name, _) => settings::find(name).map_or_else(String::new, |d| d.desc.to_owned()),
        Control::Block(block) => format!("Baritone may place {} when it bridges, pillars and builds.", block.replace('_', " ")),
        Control::Link(_) => String::new(),
    }
}

/// What a click on the control sets: the setting and its new value.
pub fn change(control: Control, s: &Settings) -> Option<(&'static str, String)> {
    match control {
        Control::Toggle(name) => Some((name, (!s.flag(name)).to_string())),
        Control::Block(block) => {
            let mut list = s.list(THROWAWAY);
            let before = list.len();
            list.retain(|b| bare(b) != block);
            if list.len() == before {
                list.push(block.to_owned());
            }
            Some((THROWAWAY, list.join(",")))
        }
        Control::Steps(..) | Control::Link(_) => None,
    }
}

/// The step a slider is set to by a press at `at` (0 to 1) along its track.
pub fn step_at(control: Control, at: f32) -> Option<(&'static str, &'static str)> {
    let Control::Steps(name, values) = control else { return None };
    let last = values.len().checked_sub(1)?;
    let i = (at.clamp(0.0, 1.0) * last as f32).round() as usize;
    Some((name, values[i.min(last)]))
}

/// The settings a category's reset puts back.
pub fn reset_names(category: usize) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for item in CATEGORIES.get(category).map_or(&[][..], |c| c.items) {
        let name = match item.control {
            Control::Toggle(name) | Control::Steps(name, _) => name,
            Control::Block(_) => THROWAWAY,
            Control::Link(_) => continue,
        };
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// How many of the settings on the screen are not at their defaults.
pub fn changed(s: &Settings) -> usize {
    let mut seen: Vec<&'static str> = (0..CATEGORIES.len()).flat_map(reset_names).collect();
    seen.sort_unstable();
    seen.dedup();
    seen.into_iter().filter(|n| settings::find(n).is_some_and(|d| s.modified(d))).count()
}

// ---------------------------------------------------------------------------------------------
// Where everything stands, in GUI pixels. The screen is drawn on a design of 480 by 270 (the screens of a 1080p window at
// the largest GUI scale) and scaled to the window, so it looks the same however large the GUI is.

pub type Rect = [f32; 4];

pub const DESIGN: (f32, f32) = (480.0, 270.0);
pub const ROW_H: f32 = 28.0;
pub const SLIDER_ROW_H: f32 = 40.0;
pub const HEADING_H: f32 = 12.0;
pub const SIDE_ROW_H: f32 = 18.0;

pub struct Layout {
    /// GUI pixels for each pixel of the 480 by 270 design.
    pub scale: f32,
    /// The scale of the text: the font is eight pixels tall, so it follows the screen in half steps.
    pub text: f32,
    pub window: Rect,
    pub title_bar: Rect,
    pub close: Rect,
    pub sidebar: Rect,
    pub brand: Rect,
    pub done: Rect,
    /// Where the line "N settings changed" starts.
    pub info: [f32; 2],
    /// Where the categories scroll.
    pub side_list: Rect,
    pub pane: Rect,
    pub head: Rect,
    pub reset: Rect,
    /// Where the rows of a category scroll.
    pub list: Rect,
}

pub fn layout(width: f32, height: f32) -> Layout {
    let f = (width / DESIGN.0).min(height / DESIGN.1).max(0.5);
    let (ox, oy) = ((width - DESIGN.0 * f) / 2.0, (height - DESIGN.1 * f) / 2.0);
    let r = |x: f32, y: f32, w: f32, h: f32| -> Rect { [ox + x * f, oy + y * f, w * f, h * f] };
    Layout {
        scale: f,
        text: ((0.75 * f * 2.0).round() / 2.0).max(0.75),
        window: r(12.0, 12.0, 456.0, 246.0),
        title_bar: r(12.0, 12.0, 456.0, 17.0),
        close: r(448.0, 12.0, 20.0, 17.0),
        sidebar: r(12.0, 29.0, 141.0, 229.0),
        brand: r(18.0, 34.0, 129.0, 62.0),
        done: r(18.0, 100.0, 129.0, 15.0),
        info: [ox + 18.0 * f, oy + 120.0 * f],
        side_list: r(12.0, 134.0, 141.0, 124.0),
        pane: r(153.0, 29.0, 315.0, 229.0),
        head: r(153.0, 29.0, 315.0, 28.0),
        reset: r(398.0, 36.0, 64.0, 15.0),
        list: r(153.0, 58.0, 315.0, 200.0),
    }
}

pub fn inside(cursor: (f32, f32), r: Rect) -> bool {
    cursor.0 >= r[0] && cursor.0 < r[0] + r[2] && cursor.1 >= r[1] && cursor.1 < r[1] + r[3]
}

pub fn row_height(item: &Item) -> f32 {
    match item.control {
        Control::Steps(..) => SLIDER_ROW_H,
        _ => ROW_H,
    }
}

/// How tall the rows of `items` are together.
pub fn content_height(l: &Layout, items: &[Item]) -> f32 {
    items.iter().map(row_height).sum::<f32>() * l.scale
}

/// Each row's rectangle with the list scrolled by `scroll`; rows outside the list are still given (the drawing clips them).
pub fn rows(l: &Layout, items: &[Item], scroll: f32) -> Vec<Rect> {
    let mut y = l.list[1] - scroll;
    items
        .iter()
        .map(|item| {
            let h = row_height(item) * l.scale;
            let rect = [l.list[0], y, l.list[2], h];
            y += h;
            rect
        })
        .collect()
}

fn entry_height(e: &Entry) -> f32 {
    if matches!(e, Entry::Heading(_)) { HEADING_H } else { SIDE_ROW_H }
}

/// The whole sidebar's height.
pub fn side_content_height(l: &Layout) -> f32 {
    SIDEBAR.iter().map(entry_height).sum::<f32>() * l.scale
}

/// Each sidebar entry's rectangle with the column scrolled by `scroll`.
pub fn side_entries(l: &Layout, scroll: f32) -> Vec<(Entry, Rect)> {
    let mut y = l.side_list[1] - scroll;
    SIDEBAR
        .iter()
        .map(|e| {
            let h = entry_height(e) * l.scale;
            let rect = [l.side_list[0], y, l.side_list[2], h];
            y += h;
            (*e, rect)
        })
        .collect()
}

/// The slider's track inside its row: where the knob runs.
pub fn track(l: &Layout, row: Rect) -> Rect {
    let f = l.scale;
    [row[0] + 8.0 * f, row[1] + row[3] - 13.0 * f, row[2] - 24.0 * f, 8.0 * f]
}

/// The switch at the right of a row.
pub fn switch(l: &Layout, row: Rect) -> Rect {
    let f = l.scale;
    [row[0] + row[2] - 42.0 * f, row[1] + (row[3] - 14.0 * f) / 2.0, 32.0 * f, 14.0 * f]
}

/// The button of a link row.
pub fn open_button(l: &Layout, row: Rect) -> Rect {
    let f = l.scale;
    [row[0] + row[2] - 54.0 * f, row[1] + (row[3] - 15.0 * f) / 2.0, 44.0 * f, 15.0 * f]
}

// ---------------------------------------------------------------------------------------------
// The screen's own state and what it does with a click.

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Set(&'static str, String),
    Open(&'static str),
    ResetCategory(usize),
    /// Done or the close button: back to Options > Mods.
    Back,
}

#[derive(Debug, Default)]
pub struct SettingsScreen {
    pub category: usize,
    pub scroll: f32,
    pub side_scroll: f32,
    /// The row whose slider is being dragged.
    pub dragging: Option<usize>,
    /// The category the sidebar was last scrolled to show.
    revealed: Option<usize>,
}

impl SettingsScreen {
    pub fn items(&self) -> &'static [Item] {
        CATEGORIES.get(self.category).map_or(&[], |c| c.items)
    }

    /// Keeps the sidebar showing the category that is open: when the page opens, and when the category changes.
    pub fn reveal(&mut self, l: &Layout) {
        if self.revealed == Some(self.category) {
            return;
        }
        self.revealed = Some(self.category);
        let Some((_, rect)) = side_entries(l, 0.0).into_iter().find(|(e, _)| *e == Entry::Category(self.category)) else { return };
        let (top, bottom) = (rect[1] - l.side_list[1], rect[1] + rect[3] - l.side_list[1]);
        if top < self.side_scroll {
            self.side_scroll = top;
        } else if bottom > self.side_scroll + l.side_list[3] {
            self.side_scroll = bottom - l.side_list[3];
        }
    }

    pub fn clamp(&mut self, l: &Layout) {
        self.reveal(l);
        self.scroll = self.scroll.clamp(0.0, (content_height(l, self.items()) - l.list[3]).max(0.0));
        self.side_scroll = self.side_scroll.clamp(0.0, (side_content_height(l) - l.side_list[3]).max(0.0));
    }

    /// A turn of the wheel, `steps` notches (up is positive), over the sidebar or the rows.
    pub fn wheel(&mut self, steps: f32, cursor: (f32, f32), l: &Layout) {
        self.reveal(l);
        if inside(cursor, l.sidebar) {
            self.side_scroll -= steps * SIDE_ROW_H * l.scale;
        } else {
            self.scroll -= steps * 18.0 * l.scale;
        }
        self.clamp(l);
    }

    /// A press (or, with `drag`, a movement with the button held) at `cursor`.
    pub fn click(&mut self, cursor: (f32, f32), size: (f32, f32), drag: bool, s: &Settings) -> Action {
        let l = layout(size.0, size.1);
        let items = self.items();
        let rects = rows(&l, items, self.scroll);
        if drag {
            return match self.dragging {
                Some(i) => slider_action(&l, items, &rects, i, cursor),
                None => Action::None,
            };
        }
        self.dragging = None;
        if inside(cursor, l.close) || inside(cursor, l.done) {
            return Action::Back;
        }
        if inside(cursor, l.reset) {
            return Action::ResetCategory(self.category);
        }
        if inside(cursor, l.side_list) {
            for (entry, rect) in side_entries(&l, self.side_scroll) {
                if let Entry::Category(i) = entry
                    && inside(cursor, rect)
                {
                    if i != self.category {
                        self.category = i;
                        self.scroll = 0.0;
                    }
                    return Action::None;
                }
            }
            return Action::None;
        }
        if !inside(cursor, l.list) {
            return Action::None;
        }
        for (i, (item, rect)) in items.iter().zip(&rects).enumerate() {
            if !inside(cursor, *rect) {
                continue;
            }
            return match item.control {
                Control::Link(url) => {
                    if inside(cursor, open_button(&l, *rect)) { Action::Open(url) } else { Action::None }
                }
                Control::Steps(..) => {
                    if inside(cursor, grab(&l, track(&l, *rect))) {
                        self.dragging = Some(i);
                        slider_action(&l, items, &rects, i, cursor)
                    } else {
                        Action::None
                    }
                }
                // The whole row is the switch: a press anywhere on it flips it.
                control => change(control, s).map_or(Action::None, |(name, value)| Action::Set(name, value)),
            };
        }
        Action::None
    }
}

/// The track with room above and below it for a thumb to be grabbed by.
fn grab(l: &Layout, t: Rect) -> Rect {
    let pad = 4.0 * l.scale;
    [t[0] - pad, t[1] - pad, t[2] + 2.0 * pad, t[3] + 2.0 * pad]
}

fn slider_action(l: &Layout, items: &[Item], rects: &[Rect], i: usize, cursor: (f32, f32)) -> Action {
    let (Some(item), Some(rect)) = (items.get(i), rects.get(i)) else { return Action::None };
    let t = track(l, *rect);
    match step_at(item.control, (cursor.0 - t[0]) / t[2]) {
        Some((name, value)) => Action::Set(name, value.to_owned()),
        None => Action::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use settings::Kind;

    fn defaults() -> Settings {
        Settings::load(|_| None)
    }

    fn center(r: Rect) -> (f32, f32) {
        (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)
    }

    #[test]
    fn every_control_names_a_setting_of_the_right_kind_and_has_a_description() {
        for c in CATEGORIES {
            assert!(!c.items.is_empty(), "{}", c.label);
            for item in c.items {
                assert!(!describe(item).is_empty() || matches!(item.control, Control::Link(_)), "{}", item.label);
                match item.control {
                    Control::Toggle(name) => assert_eq!(settings::find(name).map(|d| d.kind), Some(Kind::Bool), "{name}"),
                    Control::Steps(name, values) => {
                        let kind = settings::find(name).map(|d| d.kind);
                        assert!(matches!(kind, Some(Kind::Int | Kind::Float)), "{name}");
                        let default = settings::find(name).unwrap().default;
                        assert!(values.iter().any(|v| same_number(v, default)), "{name}: {default} is not one of the steps");
                        for v in values {
                            assert!(defaults().clone().set(name, v).is_ok(), "{name} {v}");
                        }
                    }
                    Control::Block(_) => assert_eq!(settings::find(THROWAWAY).map(|d| d.kind), Some(Kind::List)),
                    Control::Link(url) => assert!(url.starts_with("https://")),
                }
            }
        }
    }

    #[test]
    fn the_sidebar_lists_every_category_once_in_order() {
        let listed: Vec<usize> = SIDEBAR.iter().filter_map(|e| if let Entry::Category(i) = e { Some(*i) } else { None }).collect();
        assert_eq!(listed, (0..CATEGORIES.len()).collect::<Vec<_>>());
        assert!(matches!(SIDEBAR[0], Entry::Heading(_)));
    }

    #[test]
    fn a_toggle_flips_and_a_block_joins_and_leaves_the_list() {
        let mut s = defaults();
        assert_eq!(change(Control::Toggle("allowPlace"), &s), Some(("allowPlace", "false".to_owned())));
        let (name, value) = change(Control::Block("grass_block"), &s).unwrap();
        assert_eq!((name, value.as_str()), (THROWAWAY, "dirt,cobblestone,netherrack,stone,grass_block"));
        s.set(name, &value).unwrap();
        assert_eq!(is_on(Control::Block("grass_block"), &s), Some(true));
        assert_eq!(change(Control::Block("dirt"), &s).unwrap().1, "cobblestone,netherrack,stone,grass_block");
        s.set(THROWAWAY, "minecraft:dirt").unwrap();
        assert_eq!(is_on(Control::Block("dirt"), &s), Some(true));
    }

    #[test]
    fn a_slider_maps_its_track_to_steps_and_its_value_to_a_knob() {
        let c = Control::Steps("maxFallHeightNoWater", &["1", "2", "3", "4", "5"]);
        assert_eq!(step_at(c, 0.0), Some(("maxFallHeightNoWater", "1")));
        assert_eq!(step_at(c, 0.49), Some(("maxFallHeightNoWater", "3")));
        assert_eq!(step_at(c, 1.0), Some(("maxFallHeightNoWater", "5")));
        assert_eq!(step_at(c, -3.0), Some(("maxFallHeightNoWater", "1")));
        assert_eq!(step_at(c, 9.0), Some(("maxFallHeightNoWater", "5")));
        assert_eq!(step_at(Control::Toggle("allowPlace"), 0.5), None);
        let mut s = defaults();
        assert_eq!(position(c, &s), Some(0.5));
        s.set("maxFallHeightNoWater", "5").unwrap();
        assert_eq!(position(c, &s), Some(1.0));
        s.set("maxFallHeightNoWater", "9").unwrap();
        assert_eq!(position(c, &s), None);
        let f = Control::Steps("pathRenderLineWidthPixels", &["1", "5", "8"]);
        s.set("pathRenderLineWidthPixels", "5.0").unwrap();
        assert_eq!(position(f, &s), Some(0.5));
    }

    #[test]
    fn a_reset_covers_the_settings_of_the_category_once_each_and_changes_are_counted() {
        let blocks = reset_names(1);
        assert_eq!(blocks.iter().filter(|n| **n == THROWAWAY).count(), 1);
        assert!(blocks.contains(&"allowInventory"));
        assert!(reset_names(8).is_empty());
        assert!(reset_names(99).is_empty());
        let mut s = defaults();
        assert_eq!(changed(&s), 0);
        s.set("allowPlace", "false").unwrap();
        s.set(THROWAWAY, "dirt").unwrap();
        s.set("prefix", "!").unwrap();
        assert_eq!(changed(&s), 2, "a setting that is not on the screen is not counted");
    }

    #[test]
    fn the_screen_fits_its_window_at_every_size_and_keeps_the_design_proportions() {
        for (w, h) in [(480.0, 270.0), (960.0, 540.0), (1280.0, 480.0), (640.0, 360.0), (320.0, 180.0)] {
            let l = layout(w, h);
            let inner = [l.title_bar, l.close, l.sidebar, l.brand, l.done, l.side_list, l.pane, l.head, l.reset, l.list];
            for r in inner {
                assert!(r[0] >= l.window[0] - 0.01 && r[1] >= l.window[1] - 0.01, "{r:?} at {w}x{h}");
                assert!(r[0] + r[2] <= l.window[0] + l.window[2] + 0.01 && r[1] + r[3] <= l.window[1] + l.window[3] + 0.01, "{r:?} at {w}x{h}");
            }
            assert!(l.sidebar[0] + l.sidebar[2] <= l.pane[0] + 0.01);
            assert!(l.brand[1] + l.brand[3] <= l.done[1]);
            assert!(l.done[1] + l.done[3] <= l.side_list[1]);
            assert!(l.head[1] + l.head[3] <= l.list[1] + 0.01);
            assert!(l.window[0] + l.window[2] <= w + 0.01 && l.window[1] + l.window[3] <= h + 0.01);
        }
        let l = layout(480.0, 270.0);
        assert_eq!((l.window[0], l.window[1], l.window[2], l.window[3]), (12.0, 12.0, 456.0, 246.0));
        assert_eq!(l.text, 1.0);
        let big = layout(960.0, 540.0);
        assert_eq!(big.scale, 2.0);
        assert_eq!(big.window[2], 912.0);
        assert_eq!(big.text, 1.5);
        let wide = layout(1280.0, 480.0);
        assert!(wide.window[0] > 100.0, "a window wider than the design is centred");
    }

    #[test]
    fn rows_stack_with_their_own_heights_and_scroll() {
        let l = layout(480.0, 270.0);
        let items = CATEGORIES[0].items;
        let rects = rows(&l, items, 0.0);
        assert_eq!(rects[0][1], l.list[1]);
        assert_eq!(rects[1][1], l.list[1] + ROW_H);
        assert_eq!(rects[11][3], SLIDER_ROW_H, "the slider row is taller");
        assert_eq!(content_height(&l, items), 11.0 * ROW_H + SLIDER_ROW_H);
        assert!(content_height(&l, items) > l.list[3]);
        assert!(content_height(&l, CATEGORIES[7].items) < l.list[3]);
        let scrolled = rows(&l, items, 30.0);
        assert_eq!(scrolled[0][1], l.list[1] - 30.0);
        let mut screen = SettingsScreen::default();
        screen.wheel(-50.0, (300.0, 100.0), &l);
        assert_eq!(screen.scroll, content_height(&l, items) - l.list[3]);
        screen.wheel(50.0, (300.0, 100.0), &l);
        assert_eq!(screen.scroll, 0.0);
    }

    #[test]
    fn the_wheel_scrolls_the_sidebar_over_the_sidebar_and_the_rows_elsewhere() {
        let l = layout(480.0, 270.0);
        assert!(side_content_height(&l) > l.side_list[3], "the sidebar is longer than its room, as Punchy's is");
        let mut screen = SettingsScreen::default();
        screen.wheel(-2.0, (60.0, 200.0), &l);
        assert_eq!((screen.side_scroll, screen.scroll), (2.0 * SIDE_ROW_H, 0.0));
        screen.wheel(-100.0, (60.0, 200.0), &l);
        assert_eq!(screen.side_scroll, side_content_height(&l) - l.side_list[3]);
        screen.wheel(100.0, (60.0, 200.0), &l);
        assert_eq!(screen.side_scroll, 0.0);
        screen.wheel(-1.0, (300.0, 100.0), &l);
        assert_eq!((screen.side_scroll, screen.scroll), (0.0, 18.0));
        let entries = side_entries(&l, 0.0);
        assert_eq!(entries.len(), SIDEBAR.len());
        for pair in entries.windows(2) {
            assert_eq!(pair[0].1[1] + pair[0].1[3], pair[1].1[1], "{pair:?}");
        }
    }

    #[test]
    fn clicks_choose_a_category_flip_a_switch_drag_a_slider_and_press_the_buttons() {
        let s = defaults();
        let size = (480.0, 270.0);
        let l = layout(size.0, size.1);
        let mut screen = SettingsScreen::default();
        // Movement's first row is Place blocks: a press anywhere on the row flips it.
        let rects = rows(&l, screen.items(), 0.0);
        assert_eq!(screen.click(center(rects[0]), size, false, &s), Action::Set("allowPlace", "false".into()));
        // Between two rows there is no gap: the next row answers.
        let edge = (rects[0][0] + 5.0, rects[0][1] + rects[0][3] + 0.5);
        assert_eq!(screen.click(edge, size, false, &s), Action::Set("allowBreak", "false".into()));
        // The slider row (the last one of Movement): scroll to it, press at the start of the track, drag to the end.
        screen.scroll = content_height(&l, screen.items()) - l.list[3];
        let rects = rows(&l, screen.items(), screen.scroll);
        let t = track(&l, rects[11]);
        assert_eq!(screen.click((t[0], t[1] + t[3] / 2.0), size, false, &s), Action::Set("maxFallHeightNoWater", "1".into()));
        assert_eq!(screen.dragging, Some(11));
        assert_eq!(screen.click((t[0] + t[2] + 40.0, 3.0), size, true, &s), Action::Set("maxFallHeightNoWater", "20".into()));
        // Pressing the slider row beside the track does nothing, and a press ends the drag.
        assert_eq!(screen.click((rects[11][0] + 3.0, rects[11][1] + 3.0), size, false, &s), Action::None);
        assert_eq!(screen.dragging, None);
        assert_eq!(screen.click((t[0], t[1]), size, true, &s), Action::None, "a drag that did not start on a slider moves nothing");
        // A category: Blocks is the second.
        let entries = side_entries(&l, 0.0);
        let blocks = entries.iter().find(|(e, _)| *e == Entry::Category(1)).unwrap().1;
        assert_eq!(screen.click(center(blocks), size, false, &s), Action::None);
        assert_eq!((screen.category, screen.scroll), (1, 0.0));
        // A heading is not a button.
        let heading = entries[0].1;
        assert_eq!(screen.click(center(heading), size, false, &s), Action::None);
        assert_eq!(screen.category, 1);
        assert_eq!(screen.click(center(l.reset), size, false, &s), Action::ResetCategory(1));
        assert_eq!(screen.click(center(l.done), size, false, &s), Action::Back);
        assert_eq!(screen.click(center(l.close), size, false, &s), Action::Back);
        assert_eq!(screen.click((0.0, 0.0), size, false, &s), Action::None);
        // A link opens from its button only.
        screen.category = 8;
        let rects = rows(&l, screen.items(), 0.0);
        let button = open_button(&l, rects[2]);
        assert!(matches!(screen.click(center(button), size, false, &s), Action::Open(url) if url.ends_with("/issues")));
        assert_eq!(screen.click((rects[2][0] + 3.0, rects[2][1] + 3.0), size, false, &s), Action::None);
    }

    #[test]
    fn the_sidebar_scrolls_to_the_open_category_once_and_then_leaves_the_wheel_alone() {
        let l = layout(480.0, 270.0);
        let mut screen = SettingsScreen { category: 8, ..Default::default() };
        screen.clamp(&l);
        assert_eq!(screen.side_scroll, side_content_height(&l) - l.side_list[3], "Help & Support is the last entry");
        let last = side_entries(&l, screen.side_scroll).into_iter().last().unwrap().1;
        assert!(last[1] >= l.side_list[1] && last[1] + last[3] <= l.side_list[1] + l.side_list[3] + 0.01);
        // The user scrolls up: it stays where the wheel left it.
        screen.wheel(3.0, (60.0, 200.0), &l);
        let after_wheel = screen.side_scroll;
        screen.clamp(&l);
        assert_eq!(screen.side_scroll, after_wheel);
        // Another category above the top edge scrolls back up to it.
        screen.category = 0;
        screen.clamp(&l);
        assert_eq!(screen.side_scroll, HEADING_H * l.scale);
    }

    #[test]
    fn clicks_land_the_same_on_a_larger_window() {
        let s = defaults();
        let size = (1920.0, 1080.0);
        let l = layout(size.0, size.1);
        assert_eq!(l.scale, 4.0);
        let mut screen = SettingsScreen::default();
        let rects = rows(&l, screen.items(), 0.0);
        assert_eq!(screen.click(center(rects[2]), size, false, &s), Action::Set("allowSprint", "false".into()));
    }
}
