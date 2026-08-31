use std::collections::BTreeMap;

use ink_app_format as format;

use crate::ir;

pub fn analyse(root: &ir::Node) -> Vec<format::StateDependency> {
    let mut dependencies = BTreeMap::new();
    let mut next_node = 1_u32;
    visit_node(root, &mut next_node, &mut dependencies);
    dependencies
        .into_iter()
        .map(|((state, node), kind)| format::StateDependency {
            state: format::StateId(state as u32),
            node,
            kind,
        })
        .collect()
}

fn visit_node(
    node: &ir::Node,
    next_node: &mut u32,
    dependencies: &mut BTreeMap<(usize, u32), format::DependencyKind>,
) {
    let node_id = *next_node;
    *next_node += 1;
    match node {
        ir::Node::Screen { children, .. } | ir::Node::Stack { children, .. } => {
            for child in children {
                visit_node(child, next_node, dependencies);
            }
        }
        ir::Node::Text { parts, .. } => {
            for state in text_dependencies(parts) {
                add_dependency(dependencies, state, node_id, format::DependencyKind::Layout);
            }
        }
        ir::Node::TextInput { state, .. } => add_dependency(
            dependencies,
            *state,
            node_id,
            format::DependencyKind::Layout,
        ),
        ir::Node::Button { label, .. } => {
            for state in text_dependencies(label) {
                add_dependency(dependencies, state, node_id, format::DependencyKind::Layout);
            }
        }
        ir::Node::Field { value, .. } => {
            for state in text_dependencies(value) {
                add_dependency(dependencies, state, node_id, format::DependencyKind::Layout);
            }
        }
        ir::Node::Image { source, .. } => match source {
            ir::ImageSource::Remote(parts) | ir::ImageSource::Camera(parts) => {
                for state in text_dependencies(parts) {
                    add_dependency(dependencies, state, node_id, format::DependencyKind::Layout);
                }
            }
            ir::ImageSource::Local(_) => {}
        },
        ir::Node::Toggle { state, .. } => {
            add_dependency(
                dependencies,
                *state,
                node_id,
                format::DependencyKind::Layout,
            );
        }
        ir::Node::Tabs { state, tabs } => {
            add_dependency(
                dependencies,
                *state,
                node_id,
                format::DependencyKind::Structure,
            );
            for tab in tabs {
                visit_node(&tab.screen, next_node, dependencies);
            }
        }
        ir::Node::Navigator { routes } => {
            for route in routes {
                visit_node(&route.screen, next_node, dependencies);
            }
        }
        ir::Node::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            for state in condition_dependencies(condition) {
                add_dependency(
                    dependencies,
                    state,
                    node_id,
                    format::DependencyKind::Structure,
                );
            }
            visit_node(consequent, next_node, dependencies);
            if let Some(alternate) = alternate {
                visit_node(alternate, next_node, dependencies);
            }
        }
        ir::Node::ForEach {
            collection,
            template,
        } => {
            if let ir::Collection::State(state) = collection {
                add_dependency(
                    dependencies,
                    *state,
                    node_id,
                    format::DependencyKind::Structure,
                );
            }
            visit_node(template, next_node, dependencies);
        }
        ir::Node::Icon { .. } | ir::Node::CameraPreview { .. } => {}
        ir::Node::ScreenModule { .. } => {
            unreachable!("screen modules are resolved before analysis")
        }
    }
}

fn text_dependencies(parts: &[ir::TextPart]) -> Vec<ir::StateId> {
    let mut states = Vec::new();
    for part in parts {
        match part {
            ir::TextPart::State(state) | ir::TextPart::ListLength(state) => states.push(*state),
            ir::TextPart::Value(value) => value_dependencies(value, &mut states),
            ir::TextPart::Literal(_)
            | ir::TextPart::Resource(_, _)
            | ir::TextPart::Controller(_, _)
            | ir::TextPart::Item(_) => {}
        }
    }
    states
}

fn condition_dependencies(condition: &ir::Condition) -> Vec<ir::StateId> {
    match condition {
        ir::Condition::ValueEquals { value, .. } => {
            let mut states = Vec::new();
            value_dependencies(value, &mut states);
            states
        }
        ir::Condition::Bool { state, .. }
        | ir::Condition::ListEmpty { state, .. }
        | ir::Condition::Equals { state, .. } => vec![*state],
        ir::Condition::ResourceEquals { .. } | ir::Condition::ControllerEquals { .. } => Vec::new(),
    }
}

fn value_dependencies(value: &ir::Value, states: &mut Vec<ir::StateId>) {
    match value {
        ir::Value::State(state) | ir::Value::ListLength(state) => states.push(*state),
        ir::Value::Binary { left, right, .. } => {
            value_dependencies(left, states);
            value_dependencies(right, states);
        }
        ir::Value::List(values) => {
            for value in values {
                value_dependencies(value, states);
            }
        }
        ir::Value::Object(fields) => {
            for (_, value) in fields {
                value_dependencies(value, states);
            }
        }
        ir::Value::Null
        | ir::Value::Number(_)
        | ir::Value::Bool(_)
        | ir::Value::String(_)
        | ir::Value::Item(_)
        | ir::Value::Resource(_, _)
        | ir::Value::Controller(_, _)
        | ir::Value::CombinedStatus(_)
        | ir::Value::CombinedErrorResource(_)
        | ir::Value::CombinedErrorField(_, _)
        | ir::Value::RouteParam(_) => {}
    }
}

fn strongest(
    left: format::DependencyKind,
    right: format::DependencyKind,
) -> format::DependencyKind {
    use format::DependencyKind::{Layout, Structure};
    match (left, right) {
        (Structure, _) | (_, Structure) => Structure,
        (Layout, Layout) => Layout,
    }
}

fn add_dependency(
    dependencies: &mut BTreeMap<(usize, u32), format::DependencyKind>,
    state: ir::StateId,
    node: u32,
    kind: format::DependencyKind,
) {
    dependencies
        .entry((state.0, node))
        .and_modify(|current| *current = strongest(*current, kind))
        .or_insert(kind);
}
