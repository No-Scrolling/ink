//! A compiled app with `theme.ts`, run through QuickJS, `ReactTree` and layout. Theme fonts are
//! process-wide, so these contracts run in their own test binary.

use ink_core::{Colour, Rect, TextAlign, TextRun};
use ink_test::SceneApp;

const WIDTH: f32 = 1080.0;
// Values from test/fixtures/scene/themes/theme.ts, in pixels of the 1080-pixel-wide viewport.
const INSET: f32 = 90.0;
const HEADER_TOP: f32 = 50.0;

fn settings() -> SceneApp {
    let mut app = SceneApp::new("themes");
    app.pump_until(|app| app.labels().contains(&"Settings".into()));
    app
}

fn run<'a>(app: &'a SceneApp, text: &str) -> &'a TextRun {
    app.engine.scene().text.iter().find(|run| run.text == text)
        .unwrap_or_else(|| panic!("no text run {text:?} in {:?}", app.labels()))
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = value as f32 / 255.0;
    if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
}

fn foreground() -> Colour {
    Colour { red: srgb_to_linear(0xc8), green: srgb_to_linear(0xff), blue: srgb_to_linear(0x00), alpha: 1.0 }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

fn dispatch(app: &mut SceneApp) {
    let request = app.engine.take_native_request().expect("interaction did not dispatch");
    let event = app.tree.route_native_message(request.payload().to_owned()).unwrap();
    app.runtime.send(event).unwrap();
}

#[test]
fn large_titles_align_with_content_and_back_sits_at_the_end() {
    let mut app = settings();
    let title = run(&app, "Settings");
    assert!(close(title.rect.x, INSET), "title starts at the content inset, got {}", title.rect.x);
    assert!(close(title.rect.y, HEADER_TOP), "header.top sits above the title, got {}", title.rect.y);

    let (_, x, y) = app.visible_label("Spelling");
    assert!(app.engine.tap(x, y));
    dispatch(&mut app);
    app.pump_until(|app| app.labels().contains(&"Detail page".into()));
    assert!(close(run(&app, "Detail").rect.x, INSET), "an end Back button leaves the title at the inset");

    let header_bottom = run(&app, "Detail page").rect.y;
    let back: Rect = app.engine.scene().masks.iter().map(|mask| mask.rect)
        .find(|rect| rect.y < header_bottom)
        .expect("Back draws an icon in the header");
    assert!(back.x > WIDTH / 2.0, "Back is at the end of the header, got x = {}", back.x);
    assert!(app.engine.tap(back.x + back.width / 2.0, back.y + back.height / 2.0), "the end Back icon is pressable");
    dispatch(&mut app);
    app.pump_until(|app| app.labels().contains(&"Settings".into()));
}

#[test]
fn cutout_below_starts_the_screen_under_a_top_cutout() {
    let mut app = settings();
    let camera = Rect { x: 40.0, y: 0.0, width: 100.0, height: 120.0 };
    assert!(app.engine.set_display_cutouts(vec![camera]));
    let title = run(&app, "Settings");
    assert!(close(title.rect.y, camera.height + HEADER_TOP), "title starts header.top below the cutout, got {}", title.rect.y);
    assert!(close(title.rect.x, INSET), "the title keeps the content inset beside a left cutout");
}

#[test]
fn inline_fields_share_a_row_and_wrap_values_that_do_not_fit() {
    let app = settings();
    let label = run(&app, "Spelling");
    let value = run(&app, "British");
    assert!(close(label.rect.y, value.rect.y), "label and value share a row");
    assert_eq!(value.align, TextAlign::End, "the value sits at the end of the row");
    let end = value.rect.x + value.rect.width;
    assert!(end > WIDTH / 2.0 && end < WIDTH - INSET, "the value ends before the chevron, got {end}");

    let label = run(&app, "Keyboard");
    let first_value_line = app.engine.scene().text.iter()
        .find(|run| run.text.starts_with("A value"))
        .expect("long values are shown, not dropped");
    assert!(first_value_line.rect.y > label.rect.y, "a value that does not fit wraps under its label");
    assert!(!app.labels().iter().any(|label| label.ends_with('…')), "inline values are not truncated");
}

#[test]
fn section_titles_group_with_the_rows_below_them() {
    let app = settings();
    let swiping = run(&app, "SWIPING");
    let keyboard = run(&app, "Keyboard").rect.y;
    // The bottom of the Keyboard row, including its wrapped value.
    let rows_above = app.engine.scene().text.iter()
        .filter(|run| run.rect.y >= keyboard && run.rect.y < swiping.rect.y)
        .map(bottom)
        .fold(0.0, f32::max);
    let above = swiping.rect.y - rows_above;
    let below = run(&app, "Enabled toggle").rect.y - bottom(swiping);
    assert!(below < above, "a section title sits closer to its rows ({below}) than to the rows above ({above})");
}

fn bottom(run: &TextRun) -> f32 {
    run.rect.y + run.rect.height
}

#[test]
fn theme_colours_replace_ink_defaults_but_not_colours_the_app_chose() {
    let app = settings();
    assert_eq!(run(&app, "Spelling").colour, foreground(), "Ink text uses the theme foreground");
    assert_eq!(run(&app, "Canvas default").colour, foreground(), "canvas text without a colour uses the theme foreground");
    assert!(
        app.engine.scene().quads.iter().any(|quad| quad.colour == Colour::WHITE),
        "an explicit #ffffff canvas fill stays white"
    );
}

#[test]
fn disabled_custom_toggles_draw_their_disabled_state() {
    let app = settings();
    let images = &app.engine.scene().images;
    assert_eq!(images.len(), 2, "each custom toggle draws one image");
    assert_ne!(images[0].image.id(), images[1].image.id(), "enabled and disabled toggles differ");
}

fn rows() -> SceneApp {
    let mut app = settings();
    let (_, x, y) = app.visible_label("Keyboard");
    assert!(app.engine.tap(x, y));
    dispatch(&mut app);
    app.pump_until(|app| app.labels().contains(&"Item 0".into()));
    app
}

fn muted() -> Colour {
    Colour { red: srgb_to_linear(0x7f), green: srgb_to_linear(0x7f), blue: srgb_to_linear(0x84), alpha: 1.0 }
}

#[test]
fn a_row_end_action_is_a_separate_button() {
    let mut app = rows();
    let (_, x, y) = app.visible_label("Item 1");
    assert_eq!(app.tap(x, y)["value"], "row 1", "the row keeps its own action");
    let icon = app.engine.scene().masks.iter().map(|mask| mask.rect)
        .find(|rect| (rect.y + rect.height / 2.0 - y).abs() < 60.0 && rect.x > WIDTH / 2.0)
        .expect("the end action draws its icon at the end of the row");
    assert!(close(icon.x + icon.width, WIDTH - INSET), "the icon ends at the content edge, got {}", icon.x + icon.width);
    assert_eq!(app.tap(icon.x + icon.width / 2.0, icon.y + icon.height / 2.0)["value"], "remove 1");
}

#[test]
fn a_screen_note_stays_at_the_bottom_in_the_muted_colour() {
    let mut app = rows();
    let note = run(&app, "Stored on this phone only").clone();
    assert_eq!(note.colour, muted());
    assert!(note.rect.y > 1000.0 && note.rect.y + note.rect.height <= 1240.0, "the note is at the bottom, got {}", note.rect.y);
    assert!(app.engine.scroll_by(300.0), "the rows scroll");
    let moved = run(&app, "Stored on this phone only");
    assert!(!moved.scrolling && moved.rect.y == note.rect.y, "the note does not scroll with the content");
}

#[test]
fn a_scrolling_scroll_bar_appears_while_scrolling_then_fades() {
    let mut app = rows();
    let bar = app.engine.scene().scroll_bar.expect("the rows overflow the screen");
    assert_eq!(bar.opacity, 0.0, "the bar is hidden before scrolling");
    assert!(!bar.track_visible);
    assert!(close(bar.thumb_width, 8.0), "scrollBar.width sets the thumb, got {}", bar.thumb_width);
    assert!(app.engine.scroll_by(200.0));
    assert_eq!(app.engine.scene().scroll_bar.unwrap().opacity, 1.0, "scrolling shows the bar");
    assert!(app.engine.has_scene_animations(), "the host keeps drawing until the bar fades");
    std::thread::sleep(std::time::Duration::from_millis(1200));
    app.engine.animate_scene();
    assert_eq!(app.engine.scene().scroll_bar.unwrap().opacity, 0.0, "the bar fades after scrolling stops");
    assert!(!app.engine.has_scene_animations(), "nothing keeps animating once the bar is hidden");
}

#[test]
fn checked_options_can_be_pressed_across_the_whole_row() {
    let mut app = settings();
    let (_, x, y) = app.visible_label("Spelling");
    assert!(app.engine.tap(x, y));
    dispatch(&mut app);
    app.pump_until(|app| app.labels().contains(&"Second".into()));
    let (_, _, y) = app.visible_label("Second");
    // Far past the end of the label's text, near the end of the row.
    assert_eq!(app.tap(WIDTH - INSET - 10.0, y)["value"], "second");
}

#[test]
fn the_scroll_bar_uses_the_theme_colour() {
    let app = rows();
    let orange = Colour { red: 1.0, green: srgb_to_linear(0x66), blue: 0.0, alpha: 1.0 };
    assert_eq!(app.engine.scene().scroll_bar_colour(1.0), orange);
}

#[test]
fn a_screen_theme_variant_applies_to_its_screen_only() {
    let mut app = settings();
    let base = run(&app, "Spelling").clone();
    let base_background = app.engine.scene().background();
    let (_, x, y) = app.visible_label("Reader");
    assert!(app.engine.tap(x, y));
    dispatch(&mut app);
    app.pump_until(|app| app.labels().contains(&"Reader page".into()));

    let page = run(&app, "Reader page").clone();
    assert_ne!(page.font, base.font, "the variant's body font replaces the app font");
    assert!(page.font_size > base.font_size * 1.2, "text.size 60 is larger than the app's 45");
    let background = Colour { red: srgb_to_linear(0x10), green: srgb_to_linear(0x18), blue: srgb_to_linear(0x20), alpha: 1.0 };
    assert_eq!(app.engine.scene().background(), background);
    let title = run(&app, "Chapter");
    assert_eq!(title.align, TextAlign::Centre, "the variant's centred header replaces the large header");
    assert_eq!(page.colour, foreground(), "settings the variant leaves out come from the app theme");

    assert!(app.engine.back());
    dispatch(&mut app);
    app.pump_until(|app| app.labels().contains(&"Spelling".into()));
    let restored = run(&app, "Spelling");
    assert_eq!((restored.font, restored.font_size), (base.font, base.font_size), "other screens keep the app theme");
    assert_eq!(app.engine.scene().background(), base_background);
}
