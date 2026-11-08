use ink_core::{Engine, ReactTree, Scene};
use serde_json::json;

fn text_scene(strikethrough: bool) -> (Engine, ReactTree) {
    let mut engine = Engine::new();
    engine.set_viewport(1080, 1240);
    engine.set_viewport_scale(1.0);
    let mut tree = ReactTree::default();
    tree.apply(serde_json::from_value(json!([
        {"op":"create","id":1,"type":"Screen","props":{}},
        {"op":"insert","id":1,"parent":0},
        {"op":"create","id":5,"type":"Stack","props":{"align":"start"}},
        {"op":"insert","id":5,"parent":1},
        {"op":"create","id":2,"type":"Text","props":{"text":"xxxx xxxx xxxx","width":100,"align":"center","strikethrough":strikethrough}},
        {"op":"insert","id":2,"parent":5},
        {"op":"create","id":3,"type":"Text","props":{"text":"xxxx xxxx xxxx","width":100,"align":"end","maxLines":2,"strikethrough":strikethrough}},
        {"op":"insert","id":3,"parent":5},
        {"op":"create","id":4,"type":"Text","props":{"text":"xxxx xxxx xxxx","width":100,"align":"end","maxLines":1,"size":24,"strikethrough":strikethrough}},
        {"op":"insert","id":4,"parent":5}
    ])).unwrap(), &mut engine).unwrap();
    (engine, tree)
}

fn strikes<'a>(plain: &Scene, decorated: &'a Scene) -> Vec<&'a ink_core::Quad> {
    decorated
        .quads
        .iter()
        .filter(|quad| !plain.quads.contains(quad))
        .collect()
}

#[test]
fn strikethrough_follows_wrapped_and_truncated_lines_without_changing_layout() {
    let (plain, _) = text_scene(false);
    let (decorated, _) = text_scene(true);
    let plain = plain.scene();
    let decorated = decorated.scene();
    assert_eq!(plain.text, decorated.text);
    assert_eq!(
        decorated
            .text
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["xxxx", "xxxx", "xxxx", "xxxx", "xxxx…", "xxxx…"]
    );
    let lines = strikes(plain, decorated);
    assert_eq!(lines.len(), 6);
    // Public Sans at scale 1: three wrapped lines, two truncated lines, then a smaller line.
    let expected = [
        (53.370, 37.0, 67.260),
        (53.370, 77.0, 67.260),
        (53.370, 117.0, 67.260),
        (69.740, 157.0, 67.260),
        (46.879, 197.0, 90.121),
        (64.903, 231.0, 72.097),
    ];
    for (line, (x, y, width)) in lines.iter().zip(expected) {
        assert!((line.rect.x - x).abs() < 0.001);
        assert_eq!(line.rect.y, y);
        assert!((line.rect.width - width).abs() < 0.001);
        assert_eq!(line.rect.height, 1.0);
    }
}

#[test]
fn strikethrough_updates_with_fixed_width_text_and_can_be_removed() {
    let (mut engine, mut tree) = text_scene(true);
    tree.apply(
        serde_json::from_value(json!([
            {"op":"text","ids":[4],"values":["xx"]}
        ]))
        .unwrap(),
        &mut engine,
    )
    .unwrap();
    let line = engine.scene().quads.last().unwrap();
    assert!((line.rect.x - 110.096).abs() < 0.001);
    assert!((line.rect.width - 26.904).abs() < 0.001);
    assert_eq!(line.rect.y, 231.0);
    let count = engine.scene().quads.len();
    tree.apply(serde_json::from_value(json!([
        {"op":"update","id":4,"props":{"text":"xx","width":100,"align":"end","maxLines":1,"size":24,"strikethrough":false}}
    ])).unwrap(), &mut engine).unwrap();
    assert_eq!(engine.scene().quads.len(), count - 1);
}
