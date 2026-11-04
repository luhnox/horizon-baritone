// Baritone's settings screen: what each page shows, where everything stands, and what a press or a key does.
// It takes the place of the page Options > Mods > Baritone (hooks/page_*.rs). The parts it is made of are those of a Bedrock
// Edition screen: a header with a back button and a search field, tabs with an item picture, rows with an item picture, a title,
// a description and a switch, a slider, a dropdown, a button or a text field, section labels, a scrollbar and a tooltip on hover.
// Nothing here touches a game type, so it is tested in the plain crate; the drawing is in game/settings_ui.rs.
use super::baritone_settings::{self as settings, Settings};

pub const THROWAWAY: &str = "acceptableThrowawayItems";
pub const SEARCH_MAX: usize = 32;
pub const BLOCK_MAX: usize = 40;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Control {
    /// A switch.
    Toggle(&'static str),
    /// A number picked from a fixed list, on a slider.
    Steps(&'static str, &'static [&'static str]),
    /// One of a few named values, from a dropdown.
    Choice(&'static str, &'static [&'static str]),
    /// One block in the list of blocks Baritone may place, as a switch.
    Block(&'static str),
    /// A text field and a button that add a block of the player's choice to that list.
    BlockInput,
    /// An address opened in the browser, from a button.
    Link(&'static str),
    /// A button that puts every setting of the screen back.
    ResetAll,
}

pub struct Item {
    pub label: &'static str,
    /// What the row says under its title; empty takes the first sentence of the setting's own description.
    pub desc: &'static str,
    /// The item whose picture the row shows (`dirt`); one the game does not know shows none.
    pub icon: &'static str,
    pub control: Control,
}

pub enum Row {
    Heading(&'static str),
    Item(Item),
}

pub struct Category {
    pub label: &'static str,
    pub icon: &'static str,
    pub rows: &'static [Row],
}

const fn item(icon: &'static str, label: &'static str, desc: &'static str, control: Control) -> Row {
    Row::Item(Item { label, desc, icon, control })
}
const fn toggle(icon: &'static str, label: &'static str, name: &'static str) -> Row {
    item(icon, label, "", Control::Toggle(name))
}
const fn steps(icon: &'static str, label: &'static str, name: &'static str, values: &'static [&'static str]) -> Row {
    item(icon, label, "", Control::Steps(name, values))
}
const fn choice(icon: &'static str, label: &'static str, name: &'static str, values: &'static [&'static str]) -> Row {
    item(icon, label, "", Control::Choice(name, values))
}
const fn block(label: &'static str, name: &'static str) -> Row {
    item(name, label, "", Control::Block(name))
}
const fn link(icon: &'static str, label: &'static str, desc: &'static str, url: &'static str) -> Row {
    item(icon, label, desc, Control::Link(url))
}

pub const CATEGORIES: &[Category] = &[
    Category {
        label: "Movement",
        icon: "leather_boots",
        rows: &[
            Row::Heading("Basics"),
            toggle("cobblestone", "Place blocks", "allowPlace"),
            toggle("iron_pickaxe", "Break blocks", "allowBreak"),
            toggle("leather_boots", "Sprint", "allowSprint"),
            toggle("water_bucket", "Water bucket landings", "allowWaterBucketFall"),
            steps("feather", "Fall without water", "maxFallHeightNoWater", &["1", "2", "3", "4", "5", "10", "20"]),
            Row::Heading("Advanced"),
            toggle("rabbit_foot", "Parkour jumps", "allowParkour"),
            toggle("scaffolding", "Parkour places blocks", "allowParkourPlace"),
            toggle("ladder", "Parkour ascends", "allowParkourAscend"),
            toggle("stone_stairs", "Diagonal ascend", "allowDiagonalAscend"),
            toggle("oak_stairs", "Diagonal descend", "allowDiagonalDescend"),
            toggle("iron_shovel", "Dig straight down", "allowDownward"),
            toggle("vine", "Climb vines", "allowVines"),
        ],
    },
    Category {
        label: "Blocks",
        icon: "dirt",
        rows: &[
            Row::Heading("Blocks Baritone may place"),
            block("Place dirt", "dirt"),
            block("Place grass blocks", "grass_block"),
            block("Place cobblestone", "cobblestone"),
            block("Place stone", "stone"),
            block("Place netherrack", "netherrack"),
            block("Place cobbled deepslate", "cobbled_deepslate"),
            block("Place andesite", "andesite"),
            block("Place sand", "sand"),
            block("Place gravel", "gravel"),
            item("name_tag", "Another block", "Type a block's name, like sandstone, and press Add.", Control::BlockInput),
            Row::Heading("Inventory"),
            toggle("chest", "Take blocks from the inventory", "allowInventory"),
        ],
    },
    Category {
        label: "Mining",
        icon: "diamond_pickaxe",
        rows: &[
            Row::Heading("Collecting"),
            toggle("hopper", "Collect dropped items", "mineScanDroppedItems"),
            steps("clock", "Wait for drops (ms)", "mineDropLoiterDurationMSThanksLouca", &["0", "100", "250", "500", "1000"]),
            Row::Heading("Searching"),
            toggle("compass", "Explore to find blocks", "exploreForBlocks"),
            toggle("diamond_ore", "Only exposed ores", "allowOnlyExposedOres"),
            toggle("spyglass", "Legit mine", "legitMine"),
            steps("paper", "Known ore spots", "mineMaxOreLocationsCount", &["16", "32", "64", "128", "256"]),
            Row::Heading("Tools and care"),
            toggle("diamond_pickaxe", "Pick the best tool", "autoTool"),
            toggle("iron_boots", "Walk while breaking", "walkWhileBreaking"),
            toggle("gravel", "Pause for falling blocks", "pauseMiningForFallingBlocks"),
            toggle("anvil", "Keep damaged tools", "itemSaver"),
        ],
    },
    Category {
        label: "Build & Farm",
        icon: "wheat",
        rows: &[
            Row::Heading("Building"),
            toggle("bricks", "Build in layers", "buildInLayers"),
            toggle("wooden_axe", "Build only the selection", "buildOnlySelection"),
            toggle("oak_planks", "Skip what is already there", "buildIgnoreExisting"),
            toggle("filled_map", "Map art mode", "mapArtMode"),
            toggle("iron_shovel", "Backfill tunnels", "backfill"),
            choice("compass", "Schematic rotation", "buildSchematicRotation", &["NONE", "CLOCKWISE_90", "CLOCKWISE_180", "COUNTERCLOCKWISE_90"]),
            choice("ender_pearl", "Schematic mirror", "buildSchematicMirror", &["NONE", "LEFT_RIGHT", "FRONT_BACK"]),
            Row::Heading("Farming"),
            toggle("wheat_seeds", "Replant crops", "replantCrops"),
            toggle("nether_wart", "Replant nether wart", "replantNetherWart"),
            toggle("bone_meal", "Wait for crops to grow", "farmWaitForGrowth"),
        ],
    },
    Category {
        label: "Pathing",
        icon: "compass",
        rows: &[
            Row::Heading("Planning time"),
            steps("clock", "Plan time (ms)", "primaryTimeoutMS", &["250", "500", "1000", "2000"]),
            steps("clock", "Plan ahead time (ms)", "planAheadPrimaryTimeoutMS", &["1000", "2000", "4000", "8000"]),
            Row::Heading("Costs"),
            steps("stone", "Block placing cost", "blockPlacementPenalty", &["5", "10", "20", "40", "100"]),
            steps("iron_pickaxe", "Block breaking cost", "blockBreakAdditionalPenalty", &["0", "2", "5", "10"]),
            steps("slime_ball", "Jump cost", "jumpPenalty", &["0", "1", "2", "4", "8"]),
            Row::Heading("Behaviour"),
            toggle("iron_sword", "Avoid mobs", "avoidance"),
            toggle("barrier", "Cancel when the goal is gone", "cancelOnGoalInvalidation"),
            toggle("paper", "Walk on cached chunks only", "pathThroughCachedOnly"),
            toggle("cobweb", "Slow path", "slowPath"),
        ],
    },
    Category {
        label: "Rendering",
        icon: "spyglass",
        rows: &[
            Row::Heading("The path"),
            toggle("filled_map", "Draw the path", "renderPath"),
            toggle("string", "Path as a line", "renderPathAsLine"),
            toggle("glass", "Path through walls", "renderPathIgnoreDepth"),
            steps("string", "Path line width", "pathRenderLineWidthPixels", &["1", "2", "3", "5", "8"]),
            Row::Heading("The goal"),
            toggle("beacon", "Draw the goal", "renderGoal"),
            toggle("firework_rocket", "Animate the goal", "renderGoalAnimated"),
            toggle("lightning_rod", "Goal beacon", "renderGoalXZBeacon"),
            steps("string", "Goal line width", "goalRenderLineWidthPixels", &["1", "2", "3", "5", "8"]),
            Row::Heading("More"),
            toggle("wooden_axe", "Draw selections", "renderSelectionBoxes"),
            toggle("grass_block", "Show cached chunks", "renderCachedChunks"),
        ],
    },
    Category {
        label: "Chat",
        icon: "writable_book",
        rows: &[
            Row::Heading("Commands"),
            toggle("command_block", "Commands in chat", "chatControl"),
            toggle("name_tag", "Commands with the prefix", "prefixControl"),
            toggle("repeater", "Commands in chat anyway", "chatControlAnyway"),
            toggle("bell", "Echo commands", "echoCommands"),
            Row::Heading("Messages"),
            toggle("writable_book", "Short prefix", "shortBaritonePrefix"),
            toggle("name_tag", "Message tag", "useMessageTag"),
            toggle("oak_sign", "Messages as toasts", "logAsToast"),
            toggle("bell", "Desktop notifications", "desktopNotifications"),
            toggle("ender_eye", "Hide coordinates", "censorCoordinates"),
            toggle("redstone_torch", "Debug messages", "chatDebug"),
        ],
    },
    Category {
        label: "Camera",
        icon: "ender_eye",
        rows: &[
            toggle("ender_eye", "Free look", "freeLook"),
            toggle("elytra", "Free look in flight", "elytraFreeLook"),
            toggle("feather", "Smooth look in flight", "elytraSmoothLook"),
            toggle("shield", "Anti-cheat compatibility", "antiCheatCompatibility"),
        ],
    },
    Category {
        label: "Help & Support",
        icon: "book",
        rows: &[
            Row::Heading("Links"),
            link("book", "Baritone Wiki", "How every command and setting works.", "https://github.com/cabaletta/baritone/blob/master/USAGE.md"),
            link("nether_star", "Releases", "Newer versions of Baritone for Horizon.", "https://github.com/luhnox/horizon-baritone/releases"),
            link("spider_eye", "Bug Report", "Tell us what went wrong.", "https://github.com/luhnox/horizon-baritone/issues"),
            Row::Heading("Settings"),
            item("tnt", "Reset everything", "Puts every setting on these pages back to its default.", Control::ResetAll),
        ],
    },
];

// ---------------------------------------------------------------------------------------------
// What a control shows.

fn bare(name: &str) -> &str {
    name.trim().trim_start_matches("minecraft:")
}

fn same_number(a: &str, b: &str) -> bool {
    a == b || matches!((a.trim().parse::<f64>(), b.trim().parse::<f64>()), (Ok(x), Ok(y)) if (x - y).abs() < 1e-9)
}

/// The name of the setting a control sets, when it sets one.
pub fn setting_of(control: Control) -> Option<&'static str> {
    match control {
        Control::Toggle(n) | Control::Steps(n, _) | Control::Choice(n, _) => Some(n),
        Control::Block(_) | Control::BlockInput => Some(THROWAWAY),
        Control::Link(_) | Control::ResetAll => None,
    }
}

/// On or off, for the controls that are switches.
pub fn is_on(control: Control, s: &Settings) -> Option<bool> {
    match control {
        Control::Toggle(name) => Some(s.flag(name)),
        Control::Block(block) => Some(s.list(THROWAWAY).iter().any(|b| bare(b) == block)),
        _ => None,
    }
}

/// `CLOCKWISE_90` as `Clockwise 90`.
pub fn pretty(value: &str) -> String {
    let lower = value.replace('_', " ").to_lowercase();
    let mut chars = lower.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

/// The value a slider or a dropdown shows.
pub fn value(item: &Item, s: &Settings) -> String {
    match item.control {
        Control::Steps(name, _) => s.get(name).to_owned(),
        Control::Choice(name, _) => pretty(s.get(name)),
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
        Control::Toggle(name) | Control::Steps(name, _) | Control::Choice(name, _) => {
            settings::find(name).map_or_else(String::new, |d| d.desc.to_owned())
        }
        Control::Block(block) => format!("Baritone may place {} when it bridges, pillars and builds.", block.replace('_', " ")),
        Control::BlockInput | Control::Link(_) | Control::ResetAll => String::new(),
    }
}

/// What a tooltip says of the setting: its default.
pub fn default_of(item: &Item) -> Option<String> {
    match item.control {
        Control::Toggle(name) => settings::find(name).map(|d| if d.default == "true" { "on".to_owned() } else { "off".to_owned() }),
        Control::Steps(name, _) => settings::find(name).map(|d| d.default.to_owned()),
        Control::Choice(name, _) => settings::find(name).map(|d| pretty(d.default)),
        _ => None,
    }
}

/// What a press on a switch sets: the setting and its new value.
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
        _ => None,
    }
}

/// The step a slider is set to by a press at `at` (0 to 1) along its track.
pub fn step_at(control: Control, at: f32) -> Option<(&'static str, &'static str)> {
    let Control::Steps(name, values) = control else { return None };
    let last = values.len().checked_sub(1)?;
    let i = (at.clamp(0.0, 1.0) * last as f32).round() as usize;
    Some((name, values[i.min(last)]))
}

/// The settings a category's reset puts back (all of them when `category` is None).
pub fn reset_names(category: Option<usize>) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    let all = CATEGORIES.iter().enumerate().filter(|(i, _)| category.is_none_or(|c| c == *i));
    for (_, cat) in all {
        for row in cat.rows {
            if let Row::Item(i) = row
                && let Some(name) = setting_of(i.control)
                && !names.contains(&name)
            {
                names.push(name);
            }
        }
    }
    names
}

// ---------------------------------------------------------------------------------------------
// What the list shows: the page, or what a search found on every page.

pub enum View {
    /// "12 settings", first in the list.
    Count(usize),
    Heading(&'static str),
    Item(&'static Item),
}

fn matches(item: &Item, query: &str) -> bool {
    let name = setting_of(item.control).unwrap_or("");
    let block = if let Control::Block(b) = item.control { b } else { "" };
    [item.label, &describe(item), name, block].iter().any(|text| text.to_lowercase().contains(query))
}

pub fn view(category: usize, query: &str) -> Vec<View> {
    let query = query.trim().to_lowercase();
    let mut out = Vec::new();
    if query.is_empty() {
        for row in CATEGORIES.get(category).map_or(&[][..], |c| c.rows) {
            out.push(match row {
                Row::Heading(text) => View::Heading(text),
                Row::Item(i) => View::Item(i),
            });
        }
    } else {
        for cat in CATEGORIES {
            for row in cat.rows {
                if let Row::Item(i) = row
                    && matches(i, &query)
                {
                    out.push(View::Item(i));
                }
            }
        }
    }
    let count = out.iter().filter(|r| matches!(r, View::Item(_))).count();
    out.insert(0, View::Count(count));
    out
}

// ---------------------------------------------------------------------------------------------
// Where everything stands, in GUI pixels. The screen is drawn on a design of 480 by 270 (the screens of a 1080p window at
// the largest GUI scale) and scaled to the window, so it looks the same however large the GUI is.

pub type Rect = [f32; 4];

pub const DESIGN: (f32, f32) = (480.0, 270.0);
pub const ROW_H: f32 = 28.0;
pub const SLIDER_ROW_H: f32 = 40.0;
pub const HEADING_H: f32 = 16.0;
pub const COUNT_H: f32 = 14.0;
pub const TAB_H: f32 = 16.0;
pub const TAB_PITCH: f32 = 19.0;
pub const CONTROL_H: f32 = 14.0;

pub struct Layout {
    /// GUI pixels for each pixel of the 480 by 270 design.
    pub scale: f32,
    /// The scale of the text: the font is eight pixels tall, so it follows the screen in half steps.
    pub text: f32,
    pub panel: Rect,
    pub header: Rect,
    pub back: Rect,
    pub search: Rect,
    pub tabs: Vec<Rect>,
    pub frame: Rect,
    /// Where the rows scroll.
    pub list: Rect,
    pub reset: Rect,
    pub done: Rect,
}

pub fn layout(width: f32, height: f32) -> Layout {
    let f = (width / DESIGN.0).min(height / DESIGN.1).max(0.5);
    let (ox, oy) = ((width - DESIGN.0 * f) / 2.0, (height - DESIGN.1 * f) / 2.0);
    let r = |x: f32, y: f32, w: f32, h: f32| -> Rect { [ox + x * f, oy + y * f, w * f, h * f] };
    Layout {
        scale: f,
        text: ((0.75 * f * 2.0).round() / 2.0).max(0.75),
        panel: r(20.0, 14.0, 440.0, 242.0),
        header: r(20.0, 14.0, 440.0, 26.0),
        back: r(27.0, 19.0, 16.0, 16.0),
        search: r(322.0, 19.0, 130.0, 16.0),
        tabs: (0..CATEGORIES.len()).map(|i| r(28.0, 46.0 + i as f32 * TAB_PITCH, 92.0, TAB_H)).collect(),
        frame: r(126.0, 44.0, 327.0, 182.0),
        list: r(128.0, 46.0, 323.0, 178.0),
        reset: r(274.0, 231.0, 86.0, 16.0),
        done: r(366.0, 231.0, 86.0, 16.0),
    }
}

pub fn inside(cursor: (f32, f32), r: Rect) -> bool {
    cursor.0 >= r[0] && cursor.0 < r[0] + r[2] && cursor.1 >= r[1] && cursor.1 < r[1] + r[3]
}

pub fn row_height(row: &View) -> f32 {
    match row {
        View::Count(_) => COUNT_H,
        View::Heading(_) => HEADING_H,
        View::Item(i) if matches!(i.control, Control::Steps(..)) => SLIDER_ROW_H,
        View::Item(_) => ROW_H,
    }
}

/// How tall the rows together are.
pub fn content_height(l: &Layout, rows: &[View]) -> f32 {
    rows.iter().map(row_height).sum::<f32>() * l.scale
}

/// Each row's rectangle with the list scrolled by `scroll`; rows outside the list are still given (the drawing clips them).
pub fn rows(l: &Layout, rows: &[View], scroll: f32) -> Vec<Rect> {
    let mut y = l.list[1] - scroll;
    rows.iter()
        .map(|row| {
            let h = row_height(row) * l.scale;
            let rect = [l.list[0], y, l.list[2], h];
            y += h;
            rect
        })
        .collect()
}

/// A button, a field or a switch of `w` design pixels at the right of a row, centred in it.
pub fn at_right(l: &Layout, row: Rect, w: f32) -> Rect {
    let f = l.scale;
    [row[0] + row[2] - 14.0 * f - w * f, row[1] + (row[3] - CONTROL_H * f) / 2.0, w * f, CONTROL_H * f]
}

pub fn switch(l: &Layout, row: Rect) -> Rect {
    at_right(l, row, 30.0)
}

pub fn open_button(l: &Layout, row: Rect) -> Rect {
    at_right(l, row, 50.0)
}

pub fn dropdown(l: &Layout, row: Rect) -> Rect {
    at_right(l, row, 104.0)
}

/// The Add button of a block field.
pub fn add_button(l: &Layout, row: Rect) -> Rect {
    at_right(l, row, 40.0)
}

/// The text field of a block field: from the title's column to the Add button.
pub fn block_field(l: &Layout, row: Rect) -> Rect {
    let f = l.scale;
    let add = add_button(l, row);
    let x = row[0] + 8.0 * f + 22.0 * f + 70.0 * f;
    [x, add[1], add[0] - 4.0 * f - x, add[3]]
}

/// The slider's track inside its row: where the knob runs.
pub fn track(l: &Layout, row: Rect) -> Rect {
    let f = l.scale;
    [row[0] + 30.0 * f, row[1] + row[3] - 13.0 * f, row[2] - 30.0 * f - 14.0 * f, 6.0 * f]
}

/// The options of an open dropdown: one row each, under its button.
pub fn dropdown_options(button: Rect, count: usize) -> Vec<Rect> {
    (0..count).map(|i| [button[0], button[1] + button[3] * (i + 1) as f32, button[2], button[3]]).collect()
}

// ---------------------------------------------------------------------------------------------
// The screen's own state, and what it does with a press and with a key.

#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Set(&'static str, String),
    Open(&'static str),
    /// A category's own settings back to their defaults (None: all of them).
    Reset(Option<usize>),
    /// A block typed into the field.
    AddBlock(String),
    /// The back button or Done: back to Options > Mods.
    Back,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Field {
    Search,
    Block,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Key {
    Backspace,
    Enter,
    Escape,
    Other,
}

#[derive(Debug, Default)]
pub struct SettingsScreen {
    pub category: usize,
    pub scroll: f32,
    /// The row (of the list as shown) whose slider is being dragged.
    pub dragging: Option<usize>,
    /// The row whose dropdown is open.
    pub open: Option<usize>,
    pub focus: Option<Field>,
    pub search: String,
    pub block: String,
}

impl SettingsScreen {
    pub fn rows(&self) -> Vec<View> {
        view(self.category, &self.search)
    }

    pub fn clamp(&mut self, l: &Layout) {
        self.scroll = self.scroll.clamp(0.0, (content_height(l, &self.rows()) - l.list[3]).max(0.0));
    }

    /// A turn of the wheel, `steps` notches (up is positive).
    pub fn wheel(&mut self, steps: f32, l: &Layout) {
        self.open = None;
        self.scroll -= steps * 18.0 * l.scale;
        self.clamp(l);
    }

    /// A press (or, with `drag`, a movement with the button held) at `cursor`.
    pub fn click(&mut self, cursor: (f32, f32), size: (f32, f32), drag: bool, s: &Settings) -> Action {
        let l = layout(size.0, size.1);
        let view = self.rows();
        let rects = rows(&l, &view, self.scroll);
        if drag {
            return match self.dragging {
                Some(i) => slider_action(&l, &view, &rects, i, cursor),
                None => Action::None,
            };
        }
        self.dragging = None;
        // An open dropdown takes the press first: an option sets, anything else closes it.
        if let Some(i) = self.open.take()
            && let (Some(View::Item(it)), Some(row)) = (view.get(i), rects.get(i))
            && let Control::Choice(name, values) = it.control
        {
            let button = dropdown(&l, *row);
            for (value, rect) in values.iter().zip(dropdown_options(button, values.len())) {
                if inside(cursor, rect) {
                    return Action::Set(name, (*value).to_owned());
                }
            }
            if inside(cursor, button) {
                return Action::None;
            }
        }
        if inside(cursor, l.back) || inside(cursor, l.done) {
            return Action::Back;
        }
        if inside(cursor, l.reset) {
            return Action::Reset(Some(self.category));
        }
        if inside(cursor, l.search) {
            self.focus = Some(Field::Search);
            return Action::None;
        }
        if let Some(i) = l.tabs.iter().position(|r| inside(cursor, *r)) {
            self.category = i;
            self.scroll = 0.0;
            self.search.clear();
            self.focus = None;
            return Action::None;
        }
        self.focus = None;
        if !inside(cursor, l.list) {
            return Action::None;
        }
        for (i, (row, rect)) in view.iter().zip(&rects).enumerate() {
            let View::Item(it) = row else { continue };
            if !inside(cursor, *rect) {
                continue;
            }
            return match it.control {
                Control::Link(url) => {
                    if inside(cursor, open_button(&l, *rect)) { Action::Open(url) } else { Action::None }
                }
                Control::ResetAll => {
                    if inside(cursor, open_button(&l, *rect)) { Action::Reset(None) } else { Action::None }
                }
                Control::Steps(..) => {
                    if inside(cursor, grab(&l, track(&l, *rect))) {
                        self.dragging = Some(i);
                        slider_action(&l, &view, &rects, i, cursor)
                    } else {
                        Action::None
                    }
                }
                Control::Choice(..) => {
                    if inside(cursor, dropdown(&l, *rect)) {
                        self.open = Some(i);
                    }
                    Action::None
                }
                Control::BlockInput => {
                    if inside(cursor, block_field(&l, *rect)) {
                        self.focus = Some(Field::Block);
                        Action::None
                    } else if inside(cursor, add_button(&l, *rect)) {
                        self.submit_block()
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

    fn submit_block(&mut self) -> Action {
        let lower = self.block.trim().to_lowercase();
        let name = lower.trim_start_matches("minecraft:").replace(' ', "_");
        self.block.clear();
        if name.is_empty() { Action::None } else { Action::AddBlock(name) }
    }

    /// A key while the screen is up: None when it is not for a field (Escape then leaves the screen as it always does).
    pub fn key(&mut self, key: Key, text: Option<&str>) -> Option<Action> {
        let field = self.focus?;
        match key {
            Key::Escape => self.focus = None,
            Key::Enter => match field {
                Field::Search => self.focus = None,
                Field::Block => return Some(self.submit_block()),
            },
            Key::Backspace => {
                match field {
                    Field::Search => {
                        self.search.pop();
                        self.scroll = 0.0;
                    }
                    Field::Block => {
                        self.block.pop();
                    }
                }
            }
            Key::Other => {
                let (target, max) = match field {
                    Field::Search => (&mut self.search, SEARCH_MAX),
                    Field::Block => (&mut self.block, BLOCK_MAX),
                };
                for ch in text.unwrap_or("").chars().filter(|c| !c.is_control()) {
                    if target.chars().count() < max {
                        target.push(ch);
                    }
                }
                // A new search starts at the top; a block being typed leaves the list where it is.
                if field == Field::Search {
                    self.scroll = 0.0;
                }
            }
        }
        Some(Action::None)
    }
}

/// The track with room above and below it for a knob to be grabbed by.
fn grab(l: &Layout, t: Rect) -> Rect {
    let pad = 4.0 * l.scale;
    [t[0] - pad, t[1] - pad, t[2] + 2.0 * pad, t[3] + 2.0 * pad]
}

fn slider_action(l: &Layout, view: &[View], rects: &[Rect], i: usize, cursor: (f32, f32)) -> Action {
    let (Some(View::Item(it)), Some(rect)) = (view.get(i), rects.get(i)) else { return Action::None };
    let t = track(l, *rect);
    match step_at(it.control, (cursor.0 - t[0]) / t[2]) {
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

    fn items(category: usize) -> Vec<&'static Item> {
        CATEGORIES[category].rows.iter().filter_map(|r| if let Row::Item(i) = r { Some(i) } else { None }).collect()
    }

    /// Scrolls the list so the row at `row` is at its top, as far as the page goes.
    fn scroll_to(screen: &mut SettingsScreen, l: &Layout, row: usize) {
        screen.scroll = rows(l, &screen.rows(), 0.0)[row][1] - l.list[1];
        screen.clamp(l);
    }

    /// The list index of the row with this label, on the page open.
    fn row_of(screen: &SettingsScreen, label: &str) -> usize {
        screen.rows().iter().position(|r| matches!(r, View::Item(i) if i.label == label)).unwrap()
    }

    #[test]
    fn every_control_names_a_setting_of_the_right_kind_and_has_a_description() {
        for c in CATEGORIES {
            assert!(!c.icon.is_empty());
            for row in c.rows {
                let Row::Item(item) = row else { continue };
                assert!(!item.icon.is_empty(), "{} has no picture", item.label);
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
                    Control::Choice(name, values) => {
                        let def = settings::find(name).unwrap();
                        assert!(values.contains(&def.default), "{name}: {} is not one of the choices", def.default);
                        for v in values {
                            assert!(defaults().clone().set(name, v).is_ok(), "{name} {v}");
                        }
                    }
                    Control::Block(_) | Control::BlockInput => assert_eq!(settings::find(THROWAWAY).map(|d| d.kind), Some(Kind::List)),
                    Control::Link(url) => assert!(url.starts_with("https://")),
                    Control::ResetAll => {}
                }
            }
        }
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
        assert_eq!(change(Control::Link("https://x"), &s), None);
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
    fn values_read_well_and_the_default_shows_for_a_tooltip() {
        assert_eq!(pretty("CLOCKWISE_90"), "Clockwise 90");
        assert_eq!(pretty("NONE"), "None");
        assert_eq!(pretty(""), "");
        let s = defaults();
        let rotation = items(3).into_iter().find(|i| i.label == "Schematic rotation").unwrap();
        assert_eq!(value(rotation, &s), "None");
        assert_eq!(default_of(rotation).as_deref(), Some("None"));
        let place = items(0).into_iter().find(|i| i.label == "Place blocks").unwrap();
        assert_eq!(default_of(place).as_deref(), Some("on"));
        let parkour = items(0).into_iter().find(|i| i.label == "Parkour jumps").unwrap();
        assert_eq!(default_of(parkour).as_deref(), Some("off"));
        assert_eq!(default_of(items(8)[0]), None, "a link has no default");
    }

    #[test]
    fn a_reset_covers_the_settings_of_the_page_once_each_or_of_every_page() {
        let blocks = reset_names(Some(1));
        assert_eq!(blocks.iter().filter(|n| **n == THROWAWAY).count(), 1);
        assert!(blocks.contains(&"allowInventory"));
        assert!(reset_names(Some(8)).is_empty());
        assert!(reset_names(Some(99)).is_empty());
        let all = reset_names(None);
        assert!(all.contains(&"allowPlace") && all.contains(&"renderPath") && all.contains(&"buildSchematicMirror"));
        let mut once = all.clone();
        once.sort_unstable();
        once.dedup();
        assert_eq!(all.len(), once.len(), "no setting twice");
    }

    #[test]
    fn the_list_is_the_page_or_what_a_search_finds_on_every_page() {
        let page = view(1, "");
        assert!(matches!(page[0], View::Count(11)), "nine blocks, the field and the inventory switch");
        assert!(matches!(page[1], View::Heading("Blocks Baritone may place")));
        let found = view(0, "PARKOUR");
        let labels: Vec<&str> = found.iter().filter_map(|r| if let View::Item(i) = r { Some(i.label) } else { None }).collect();
        assert!(labels.contains(&"Parkour jumps") && labels.contains(&"Parkour ascends"));
        assert!(matches!(found[0], View::Count(n) if n == labels.len()));
        assert!(found.iter().all(|r| !matches!(r, View::Heading(_))), "a search lists settings, not headings");
        assert!(matches!(view(0, "mineScanDropped")[0], View::Count(1)), "the setting's own name finds it");
        assert!(matches!(view(0, "deepslate")[0], View::Count(1)), "and so does a block's");
        assert!(matches!(view(0, "zzzzzz")[0], View::Count(0)));
        assert!(matches!(view(0, "   ")[0], View::Count(n) if n > 5), "a blank search is no search");
    }

    #[test]
    fn the_screen_fits_its_window_at_every_size_and_keeps_the_design_proportions() {
        for (w, h) in [(480.0, 270.0), (960.0, 540.0), (1280.0, 480.0), (640.0, 360.0), (320.0, 180.0)] {
            let l = layout(w, h);
            let mut inner = vec![l.header, l.back, l.search, l.frame, l.list, l.reset, l.done];
            inner.extend(l.tabs.iter().copied());
            for r in inner {
                assert!(r[0] >= l.panel[0] - 0.01 && r[1] >= l.panel[1] - 0.01, "{r:?} at {w}x{h}");
                assert!(r[0] + r[2] <= l.panel[0] + l.panel[2] + 0.01 && r[1] + r[3] <= l.panel[1] + l.panel[3] + 0.01, "{r:?} at {w}x{h}");
            }
            assert!(l.back[0] + l.back[2] < l.search[0]);
            assert!(l.tabs[0][0] + l.tabs[0][2] <= l.frame[0]);
            assert!(l.frame[1] + l.frame[3] <= l.reset[1]);
            assert!(l.reset[0] + l.reset[2] < l.done[0]);
            assert!(l.header[1] + l.header[3] <= l.tabs[0][1] && l.header[1] + l.header[3] <= l.frame[1]);
            for pair in l.tabs.windows(2) {
                assert!(pair[0][1] + pair[0][3] <= pair[1][1]);
            }
            assert!(l.panel[0] + l.panel[2] <= w + 0.01 && l.panel[1] + l.panel[3] <= h + 0.01);
        }
        let l = layout(480.0, 270.0);
        assert_eq!(l.panel, [20.0, 14.0, 440.0, 242.0]);
        assert_eq!(l.text, 1.0);
        let big = layout(960.0, 540.0);
        assert_eq!((big.scale, big.text, big.panel[2]), (2.0, 1.5, 880.0));
        assert!(layout(1280.0, 480.0).panel[0] > 100.0, "a window wider than the design is centred");
    }

    #[test]
    fn rows_stack_with_their_own_heights_and_scroll() {
        let l = layout(480.0, 270.0);
        let mut screen = SettingsScreen::default();
        let view = screen.rows();
        let rects = rows(&l, &view, 0.0);
        assert_eq!(rects[0][1], l.list[1]);
        assert_eq!(rects[1][1], l.list[1] + COUNT_H, "the count first, then the heading");
        assert_eq!(rects[2][1], l.list[1] + COUNT_H + HEADING_H);
        let fall = row_of(&screen, "Fall without water");
        assert_eq!(rects[fall][3], SLIDER_ROW_H, "the slider row is taller");
        assert!(content_height(&l, &view) > l.list[3]);
        assert!(content_height(&l, &SettingsScreen { category: 7, ..Default::default() }.rows()) < l.list[3]);
        assert_eq!(rows(&l, &view, 30.0)[0][1], l.list[1] - 30.0);
        screen.wheel(-50.0, &l);
        assert_eq!(screen.scroll, content_height(&l, &view) - l.list[3]);
        screen.wheel(50.0, &l);
        assert_eq!(screen.scroll, 0.0);
    }

    #[test]
    fn presses_choose_a_tab_flip_a_switch_drag_a_slider_and_press_the_buttons() {
        let s = defaults();
        let size = (480.0, 270.0);
        let l = layout(size.0, size.1);
        let mut screen = SettingsScreen::default();
        let rects = rows(&l, &screen.rows(), 0.0);
        // The first setting of Movement is Place blocks: a press anywhere on the row flips it.
        let place = row_of(&screen, "Place blocks");
        assert_eq!(screen.click(center(rects[place]), size, false, &s), Action::Set("allowPlace", "false".into()));
        // A heading and the count are not buttons.
        assert_eq!(screen.click(center(rects[1]), size, false, &s), Action::None);
        assert_eq!(screen.click(center(rects[0]), size, false, &s), Action::None);
        // The slider row: scroll to it, press at the start of the track, drag past the end.
        let fall = row_of(&screen, "Fall without water");
        scroll_to(&mut screen, &l, fall);
        let rects = rows(&l, &screen.rows(), screen.scroll);
        let t = track(&l, rects[fall]);
        assert_eq!(screen.click((t[0], t[1] + t[3] / 2.0), size, false, &s), Action::Set("maxFallHeightNoWater", "1".into()));
        assert_eq!(screen.dragging, Some(fall));
        assert_eq!(screen.click((t[0] + t[2] + 40.0, 3.0), size, true, &s), Action::Set("maxFallHeightNoWater", "20".into()));
        assert_eq!(screen.click((rects[fall][0] + 3.0, rects[fall][1] + 3.0), size, false, &s), Action::None);
        assert_eq!(screen.dragging, None);
        assert_eq!(screen.click((t[0], t[1]), size, true, &s), Action::None, "a drag that did not start on a slider moves nothing");
        // A tab opens its page from the top and drops a search.
        screen.search = "park".into();
        assert_eq!(screen.click(center(l.tabs[1]), size, false, &s), Action::None);
        assert_eq!((screen.category, screen.scroll, screen.search.as_str()), (1, 0.0, ""));
        // The footer, the back button, and a press on nothing.
        assert_eq!(screen.click(center(l.reset), size, false, &s), Action::Reset(Some(1)));
        assert_eq!(screen.click(center(l.done), size, false, &s), Action::Back);
        assert_eq!(screen.click(center(l.back), size, false, &s), Action::Back);
        assert_eq!(screen.click((0.0, 0.0), size, false, &s), Action::None);
        // A link opens from its button only; Reset everything too.
        screen.category = 8;
        let rects = rows(&l, &screen.rows(), 0.0);
        let bug = row_of(&screen, "Bug Report");
        assert!(matches!(screen.click(center(open_button(&l, rects[bug])), size, false, &s), Action::Open(url) if url.ends_with("/issues")));
        assert_eq!(screen.click((rects[bug][0] + 3.0, rects[bug][1] + 3.0), size, false, &s), Action::None);
        let all = row_of(&screen, "Reset everything");
        assert_eq!(screen.click(center(open_button(&l, rects[all])), size, false, &s), Action::Reset(None));
    }

    #[test]
    fn a_dropdown_opens_picks_an_option_and_closes_on_any_other_press() {
        let s = defaults();
        let size = (480.0, 270.0);
        let l = layout(size.0, size.1);
        let mut screen = SettingsScreen { category: 3, ..Default::default() };
        let row = row_of(&screen, "Schematic mirror");
        scroll_to(&mut screen, &l, row);
        let rects = rows(&l, &screen.rows(), screen.scroll);
        let button = dropdown(&l, rects[row]);
        assert_eq!(screen.click(center(button), size, false, &s), Action::None);
        assert_eq!(screen.open, Some(row));
        let options = dropdown_options(button, 3);
        assert!(options[0][1] >= button[1] + button[3], "the options hang under the button");
        assert_eq!(screen.click(center(options[2]), size, false, &s), Action::Set("buildSchematicMirror", "FRONT_BACK".into()));
        assert_eq!(screen.open, None);
        // Opened again, a press on its own button closes it, and a press elsewhere closes it and does what it says.
        screen.click(center(button), size, false, &s);
        assert_eq!(screen.click(center(button), size, false, &s), Action::None);
        assert_eq!(screen.open, None);
        screen.click(center(button), size, false, &s);
        assert_eq!(screen.click(center(l.done), size, false, &s), Action::Back);
        assert_eq!(screen.open, None);
        // The wheel closes it too.
        screen.click(center(button), size, false, &s);
        screen.wheel(1.0, &l);
        assert_eq!(screen.open, None);
    }

    #[test]
    fn the_search_field_filters_and_the_block_field_adds_a_block() {
        let s = defaults();
        let size = (480.0, 270.0);
        let l = layout(size.0, size.1);
        let mut screen = SettingsScreen::default();
        assert_eq!(screen.key(Key::Other, Some("a")), None, "nothing is focused: the key is the screen's");
        assert_eq!(screen.click(center(l.search), size, false, &s), Action::None);
        assert_eq!(screen.focus, Some(Field::Search));
        assert_eq!(screen.key(Key::Other, Some("Pa\u{1b}rk")), Some(Action::None), "a control character is not typed");
        assert_eq!(screen.search, "Park");
        assert!(matches!(screen.rows()[0], View::Count(n) if n >= 3));
        screen.key(Key::Backspace, None);
        assert_eq!(screen.search, "Par");
        assert_eq!(screen.key(Key::Escape, None), Some(Action::None));
        assert_eq!(screen.focus, None);
        assert_eq!(screen.search, "Par", "Escape leaves the field and keeps the search");
        screen.focus = Some(Field::Search);
        screen.key(Key::Enter, None);
        assert_eq!(screen.focus, None);
        screen.search = "x".repeat(SEARCH_MAX);
        screen.focus = Some(Field::Search);
        screen.key(Key::Other, Some("yz"));
        assert_eq!(screen.search.len(), SEARCH_MAX, "a search has a length limit");
        // The block field: Blocks page, press the field, type, press Add or Enter.
        screen.search.clear();
        screen.category = 1;
        let row = row_of(&screen, "Another block");
        scroll_to(&mut screen, &l, row);
        let rects = rows(&l, &screen.rows(), screen.scroll);
        assert_eq!(screen.click(center(block_field(&l, rects[row])), size, false, &s), Action::None);
        assert_eq!(screen.focus, Some(Field::Block));
        let before = screen.scroll;
        screen.key(Key::Other, Some(" Minecraft:Sand Stone "));
        assert_eq!(screen.scroll, before, "typing a block does not move the list");
        assert_eq!(screen.click(center(add_button(&l, rects[row])), size, false, &s), Action::AddBlock("sand_stone".into()));
        assert_eq!(screen.block, "", "the field is emptied");
        assert_eq!(screen.click(center(add_button(&l, rects[row])), size, false, &s), Action::None, "an empty field adds nothing");
        screen.click(center(block_field(&l, rects[row])), size, false, &s);
        screen.key(Key::Other, Some("clay"));
        assert_eq!(screen.key(Key::Enter, None), Some(Action::AddBlock("clay".into())));
        // A press away from the field leaves it.
        screen.click(center(block_field(&l, rects[row])), size, false, &s);
        screen.click((rects[row][0] + 3.0, rects[row][1] + 3.0), size, false, &s);
        assert_eq!(screen.focus, None);
        // Where the block field and its button stand does not overlap.
        let (f, a) = (block_field(&l, rects[row]), add_button(&l, rects[row]));
        assert!(f[0] + f[2] < a[0] && f[2] > 40.0);
    }

    #[test]
    fn presses_land_the_same_on_a_larger_window() {
        let s = defaults();
        let size = (1920.0, 1080.0);
        let l = layout(size.0, size.1);
        assert_eq!(l.scale, 4.0);
        let mut screen = SettingsScreen::default();
        let rects = rows(&l, &screen.rows(), 0.0);
        let sprint = row_of(&screen, "Sprint");
        assert_eq!(screen.click(center(rects[sprint]), size, false, &s), Action::Set("allowSprint", "false".into()));
    }
}
