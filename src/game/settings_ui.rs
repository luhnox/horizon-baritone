// Baritone's settings screen as the game draws it: the page Options > Mods > Baritone, in the style of the Punchy settings screen,
// drawn in place of the options screen's own page for it (the hooks are hooks/page_*.rs). What it shows, where everything
// stands and what a click does is baritone_screen; this draws it with the game's own HUD pieces.
use super::baritone::Baritone;
use super::baritone_screen::{self as screen, Control, Entry, Rect, CATEGORIES};
use super::Game;
use horizon_client::{
    hud::{rgba, HudAtlas, HudBatch},
    text::Text,
    ui, widgets,
};

fn c(rgb: u32) -> u32 {
    rgba((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255)
}

const YELLOW: u32 = 0xF0B323;
const YELLOW_HOVER: u32 = 0xFFC83D;
const YELLOW_DARK: u32 = 0xA87608;
const YELLOW_LIGHT: u32 = 0xFFD769;
const WHITE: u32 = 0xFFFFFF;
const GRAY: u32 = 0xA8A8A8;
const BLACK: u32 = 0x101010;

fn fill(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, color: u32) {
    batch.fill(atlas, r[0], r[1], r[2], r[3], color);
}

fn inset(r: Rect, by: f32) -> Rect {
    [r[0] + by, r[1] + by, (r[2] - 2.0 * by).max(0.0), (r[3] - 2.0 * by).max(0.0)]
}

fn text_of(s: &str, bold: bool) -> Text {
    let t = Text::literal(s);
    if bold { t.bold() } else { t }
}

/// How wide `s` is at `scale`; bold glyphs are a pixel wider each.
fn width(atlas: &HudAtlas, s: &str, scale: f32, bold: bool) -> f32 {
    (HudBatch::text_width(atlas, s) + if bold { s.chars().count() as f32 } else { 0.0 }) * scale
}

/// Text with its top left at `(x, y)`, `scale` times the size of the font.
fn put(batch: &mut HudBatch, atlas: &HudAtlas, s: &str, x: f32, y: f32, scale: f32, color: u32, shadow: bool, bold: bool) {
    let start = batch.vertices.len();
    batch.text_styled(atlas, &text_of(s, bold), 0.0, 0.0, color, shadow);
    for v in &mut batch.vertices[start..] {
        v.pos = [(x + v.pos[0] * scale).round(), (y + v.pos[1] * scale).round()];
    }
}

/// Text centred in `r`.
fn centered(batch: &mut HudBatch, atlas: &HudAtlas, s: &str, r: Rect, scale: f32, color: u32, shadow: bool, bold: bool) {
    let w = width(atlas, s, scale, bold);
    put(batch, atlas, s, r[0] + (r[2] - w) / 2.0, r[1] + (r[3] - 8.0 * scale) / 2.0, scale, color, shadow, bold);
}

/// `s` cut with "..." to fit in `room` pixels at `scale`.
fn fitted(atlas: &HudAtlas, s: &str, room: f32, scale: f32) -> String {
    if width(atlas, s, scale, false) <= room {
        return s.to_owned();
    }
    let mut out = String::new();
    for ch in s.chars() {
        let next = format!("{out}{ch}...");
        if width(atlas, &next, scale, false) > room {
            break;
        }
        out.push(ch);
    }
    format!("{}...", out.trim_end())
}

/// A yellow button with bold black text, raised as the Punchy ones are.
fn yellow_button(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, label: &str, hover: bool, f: f32, scale: f32) {
    let line = f.max(1.0).floor();
    fill(batch, atlas, r, c(YELLOW_DARK));
    fill(batch, atlas, [r[0] + line, r[1] + line, r[2] - 2.0 * line, r[3] - 2.0 * line], c(if hover { YELLOW_HOVER } else { YELLOW }));
    fill(batch, atlas, [r[0] + line, r[1] + line, r[2] - 2.0 * line, line], c(YELLOW_LIGHT));
    centered(batch, atlas, label, [r[0], r[1] - line * 0.5, r[2], r[3]], scale, c(BLACK), false, true);
}

/// The mark before a category's name: a colour square with a letter.
fn icon(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, glyph: &str, color: u32, f: f32, scale: f32) {
    let line = f.max(1.0).floor();
    fill(batch, atlas, r, c(0x151515));
    fill(batch, atlas, inset(r, line), c(color));
    fill(batch, atlas, [r[0] + line, r[1] + line, r[2] - 2.0 * line, line], rgba(255, 255, 255, 70));
    centered(batch, atlas, glyph, r, scale, c(WHITE), true, true);
}

/// The switch: the lit half yellow with an I, the knob on the other.
fn switch(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, on: bool, f: f32, scale: f32) {
    let line = f.max(1.0).floor();
    fill(batch, atlas, r, c(0x141414));
    let inner = inset(r, line);
    let half = (inner[2] / 2.0).floor();
    let (left, right) = ([inner[0], inner[1], half, inner[3]], [inner[0] + half, inner[1], inner[2] - half, inner[3]]);
    let (lit, knob) = if on { (left, right) } else { (right, left) };
    if on {
        fill(batch, atlas, lit, c(YELLOW));
        fill(batch, atlas, [lit[0], lit[1], lit[2], line], c(YELLOW_LIGHT));
        centered(batch, atlas, "I", lit, scale, c(BLACK), false, true);
    } else {
        fill(batch, atlas, lit, c(0x2B2B2B));
        centered(batch, atlas, "O", lit, scale, c(0xB0B0B0), false, true);
    }
    fill(batch, atlas, knob, c(0x8A8A8A));
    fill(batch, atlas, [knob[0], knob[1], knob[2], knob[3] - line], c(0xD8D8D8));
    fill(batch, atlas, [knob[0], knob[1], knob[2], line], c(0xF2F2F2));
}

/// The slider: a dark track with a yellow part up to the knob, a notch at each step, and the knob.
fn slider(batch: &mut HudBatch, atlas: &HudAtlas, t: Rect, steps: usize, at: Option<f32>, f: f32) {
    let line = f.max(1.0).floor();
    fill(batch, atlas, t, c(0x141414));
    let bar = [t[0] + line, t[1] + 2.0 * f, t[2] - 2.0 * line, t[3] - 4.0 * f];
    fill(batch, atlas, bar, c(0x2B2B2B));
    let pos = at.unwrap_or(0.0);
    fill(batch, atlas, [bar[0], bar[1], bar[2] * pos, bar[3]], c(YELLOW));
    for i in 1..steps.saturating_sub(1) {
        let x = bar[0] + bar[2] * i as f32 / (steps - 1) as f32;
        fill(batch, atlas, [x.floor(), bar[1], line, bar[3]], c(0x141414));
    }
    let (kw, kh) = (7.0 * f, 12.0 * f);
    let knob = [(bar[0] + bar[2] * pos - kw / 2.0).round().max(t[0]), t[1] + t[3] / 2.0 - kh / 2.0, kw, kh];
    fill(batch, atlas, knob, c(0x141414));
    fill(batch, atlas, inset(knob, line), c(0xC8C8C8));
    fill(batch, atlas, [knob[0] + line, knob[1] + line, knob[2] - 2.0 * line, line], c(0xF2F2F2));
}

/// A thin scrollbar thumb at the right edge of `r`, when the content is taller than `r`.
fn scrollbar(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, content: f32, scroll: f32, f: f32) {
    if content <= r[3] {
        return;
    }
    let bar = (r[3] * r[3] / content).clamp(8.0 * f, r[3] - 4.0 * f);
    let y = r[1] + (scroll / (content - r[3])) * (r[3] - bar);
    fill(batch, atlas, [r[0] + r[2] - 4.0 * f, y, 3.0 * f, bar], c(0xBDBDBD));
}

/// One frame of the screen, in place of the options screen's own page.
pub fn draw(b: &mut Baritone, game: &Game<'_>, batch: &mut HudBatch) {
    let atlas = &game.gui.atlas;
    let cursor = game.cursor;
    let (w, h) = (batch.width, batch.height);
    let l = screen::layout(w, h);
    let (f, t) = (l.scale, l.text);
    let line = f.max(1.0).floor();
    b.core.ui.clamp(&l);
    let (state, settings) = (&b.core.ui, &b.core.settings);
    let hover = |r: Rect| ui::hovered(cursor, r[0], r[1], r[2], r[3]);

    // The window with its shadow.
    fill(batch, atlas, [l.window[0] + 3.0 * f, l.window[1] + 3.0 * f, l.window[2], l.window[3]], rgba(0, 0, 0, 90));
    fill(batch, atlas, l.window, c(0x161616));
    fill(batch, atlas, inset(l.window, line), c(0x313131));
    fill(batch, atlas, [l.pane[0], l.pane[1], l.pane[2] - line, l.pane[3] - line], c(0x404040));

    // The title bar: light, with the name in bold black and the close button.
    fill(batch, atlas, l.title_bar, c(0xE4E4E4));
    fill(batch, atlas, [l.title_bar[0], l.title_bar[1] + l.title_bar[3] - 2.0 * f, l.title_bar[2], 2.0 * f], c(0xA0A0A0));
    centered(batch, atlas, "BARITONE", [l.title_bar[0], l.title_bar[1], l.title_bar[2], l.title_bar[3] - 2.0 * f], t * 1.25, c(BLACK), false, true);
    let close = hover(l.close);
    if close {
        fill(batch, atlas, [l.close[0], l.close[1] + line, l.close[2], l.close[3] - 3.0 * f], c(0xCFCFCF));
    }
    centered(batch, atlas, "x", [l.close[0], l.close[1], l.close[2], l.close[3] - 2.0 * f], t * 1.4, c(BLACK), false, true);
    widgets::record("Close", l.close, true);
    widgets::widget_cursor(close, true);

    // The brand box, Done and what has changed.
    let bx = l.brand;
    fill(batch, atlas, bx, c(0x1F1F1F));
    fill(batch, atlas, [bx[0], bx[1] + bx[3] / 2.0, bx[2], bx[3] / 2.0], c(0x2A2A2A));
    let name_w = width(atlas, "BARITONE", t * 2.0, true);
    put(batch, atlas, "BARITONE", bx[0] + (bx[2] - name_w) / 2.0, bx[1] + 17.0 * f, t * 2.0, c(0xD9A441), true, true);
    let tag_w = width(atlas, "for Horizon", t * 0.9, false);
    put(batch, atlas, "for Horizon", bx[0] + (bx[2] - tag_w) / 2.0, bx[1] + 17.0 * f + 16.0 * t + 6.0 * f, t * 0.9, c(GRAY), false, false);
    let done = hover(l.done);
    yellow_button(batch, atlas, l.done, "DONE", done, f, t);
    widgets::record("Done", l.done, true);
    widgets::widget_cursor(done, true);
    let n = screen::changed(settings);
    let info = if n == 0 { "Nothing changed".to_owned() } else { format!("{n} changed from the defaults") };
    put(batch, atlas, &info, l.info[0], l.info[1], t * 0.85, c(GRAY), false, false);

    // The categories, scrolled and clipped under the Done button.
    let start = batch.vertices.len();
    fill(batch, atlas, l.side_list, c(0x2A2A2A));
    let side_hover = hover(l.side_list);
    for (entry, r) in screen::side_entries(&l, state.side_scroll) {
        if r[1] + r[3] < l.side_list[1] || r[1] > l.side_list[1] + l.side_list[3] {
            continue;
        }
        match entry {
            Entry::Heading(text) => put(batch, atlas, text, r[0] + 7.0 * f, r[1] + 3.0 * f, t * 0.85, c(0x9A9A9A), false, false),
            Entry::Category(i) => {
                let over = side_hover && hover(r);
                if i == state.category {
                    fill(batch, atlas, r, c(0x4A4A4A));
                    fill(batch, atlas, [r[0], r[1], 3.0 * f, r[3]], c(YELLOW));
                } else if over {
                    fill(batch, atlas, r, c(0x383838));
                }
                let cat = &CATEGORIES[i];
                let sq = 14.0 * f;
                icon(batch, atlas, [r[0] + 10.0 * f, r[1] + (r[3] - sq) / 2.0, sq, sq], cat.glyph, cat.color, f, t * 0.9);
                put(batch, atlas, cat.label, r[0] + 30.0 * f, r[1] + (r[3] - 8.0 * t) / 2.0, t, c(WHITE), true, false);
                if r[1] >= l.side_list[1] && r[1] + r[3] <= l.side_list[1] + l.side_list[3] {
                    widgets::record(cat.label, r, true);
                }
                widgets::widget_cursor(over, true);
            }
        }
    }
    scrollbar(batch, atlas, l.side_list, screen::side_content_height(&l), state.side_scroll, f);
    batch.clip_since(start, l.side_list[0], l.side_list[1], l.side_list[0] + l.side_list[2], l.side_list[1] + l.side_list[3]);

    // The header of the page: its name and Reset page.
    let cat = &CATEGORIES[state.category.min(CATEGORIES.len() - 1)];
    fill(batch, atlas, l.head, c(0x383838));
    fill(batch, atlas, [l.head[0], l.head[1] + l.head[3] - line, l.head[2], line], c(0x262626));
    put(batch, atlas, &cat.label.to_uppercase(), l.head[0] + 8.0 * f, l.head[1] + (l.head[3] - 8.0 * t * 1.6) / 2.0, t * 1.6, c(WHITE), true, true);
    if cat.items.iter().any(|i| !matches!(i.control, Control::Link(_))) {
        let reset = hover(l.reset);
        yellow_button(batch, atlas, l.reset, "Reset page", reset, f, t * 0.9);
        widgets::record("Reset page", l.reset, true);
        widgets::widget_cursor(reset, true);
    }

    // The rows of the page, scrolled and clipped to the list.
    let start = batch.vertices.len();
    let items = cat.items;
    let rects = screen::rows(&l, items, state.scroll);
    let list_hover = hover(l.list);
    for (i, (item, r)) in items.iter().zip(&rects).enumerate() {
        let r = *r;
        if r[1] + r[3] < l.list[1] || r[1] > l.list[1] + l.list[3] {
            continue;
        }
        let over = list_hover && hover(r);
        fill(batch, atlas, r, c(if over { 0x4B4B4B } else if i % 2 == 0 { 0x454545 } else { 0x404040 }));
        fill(batch, atlas, [r[0], r[1] + r[3] - line, r[2], line], c(0x2E2E2E));
        put(batch, atlas, item.label, r[0] + 8.0 * f, r[1] + 5.0 * f, t, c(WHITE), true, false);
        let desc = fitted(atlas, &screen::describe(item), r[2] - 70.0 * f, t * 0.85);
        put(batch, atlas, &desc, r[0] + 8.0 * f, r[1] + 5.0 * f + 9.5 * t, t * 0.85, c(GRAY), false, false);
        match item.control {
            Control::Toggle(_) | Control::Block(_) => {
                switch(batch, atlas, screen::switch(&l, r), screen::is_on(item.control, settings) == Some(true), f, t * 0.9);
            }
            Control::Steps(_, values) => {
                let shown = screen::value(item, settings);
                let vw = width(atlas, &shown, t, false);
                put(batch, atlas, &shown, r[0] + r[2] - 10.0 * f - vw, r[1] + 5.0 * f, t, c(WHITE), true, false);
                slider(batch, atlas, screen::track(&l, r), values.len(), screen::position(item.control, settings), f);
            }
            Control::Link(_) => {
                let button = screen::open_button(&l, r);
                yellow_button(batch, atlas, button, "OPEN", hover(button), f, t * 0.9);
            }
        }
        if r[1] >= l.list[1] && r[1] + r[3] <= l.list[1] + l.list[3] {
            widgets::record(item.label, r, true);
        }
        widgets::widget_cursor(over, true);
    }
    scrollbar(batch, atlas, l.list, screen::content_height(&l, items), state.scroll, f);
    batch.clip_since(start, l.list[0], l.list[1], l.list[0] + l.list[2] - line, l.list[1] + l.list[3]);
}
