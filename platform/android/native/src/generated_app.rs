use ink_core::{Action, AppDefinition, Node, StateId, StateValue, Style, TextPart};
#[rustfmt::skip]
pub fn app() -> ink_core::AppDefinition {
    let node_0 = Node::text(
            vec![TextPart::literal("Count: "), TextPart::state(StateId::new(0usize))],
        )
        .with_style(Style {
            font_size: Some(32f32),
            ..Default::default()
        });
    let node_1 = Node::button(
        "Increase",
        Action::Increment {
            state: StateId::new(0usize),
            by: 1i64,
        },
    );
    let node_2 = Node::column(vec![node_0, node_1])
        .with_style(Style {
            gap: Some(16f32),
            ..Default::default()
        });
    AppDefinition::new(vec![StateValue::Int(0i64)], node_2)
}
