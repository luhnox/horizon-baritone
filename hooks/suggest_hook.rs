fn baritone_suggest(screen: &mut ChatScreen, m: &Measure) -> std::ops::ControlFlow<()> {
    let text = screen.input.value.clone();
    if text.starts_with('/') || text.chars().all(char::is_whitespace) {
        return std::ops::ControlFlow::Continue(());
    }
    let Some(provide) = baritone_suggest::get() else {
        return std::ops::ControlFlow::Continue(());
    };
    let cursor_b = byte_at(&text, screen.input.cursor);
    let partial = &text[..cursor_b];
    let Some((start, names)) = provide(partial) else {
        return std::ops::ControlFlow::Continue(());
    };
    if screen.keep_suggestions && screen.suggestions.is_some() {
        return std::ops::ControlFlow::Break(());
    }
    screen.parse = None;
    screen.input.suggestion = None;
    screen.suggestions = None;
    screen.usage.clear();
    let mut builder = SuggestionsBuilder::new(partial, start);
    builder.suggest_matching(names.iter().map(String::as_str));
    let (range, list) = merge_suggestions(partial, builder.out);
    screen.awaiting = None;
    screen.pending = Some((range.0, list));
    screen.usage_width = 0;
    screen.usage_position = 0;
    if screen.allow_suggestions && screen.auto_suggestions {
        screen.show_suggestions(m);
    }
    std::ops::ControlFlow::Break(())
}
