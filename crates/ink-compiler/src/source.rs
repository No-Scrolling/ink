use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use oxc::{allocator::Allocator, parser::Parser, semantic::SemanticBuilder, span::SourceType};

use crate::{
    ir::{Action, App, Condition, Node, Route, State, StateId, Tab, TextPart, Value},
    lower::{self, ModuleKind},
};

pub fn compile(project_root: &Path, entry: &Path) -> Result<App> {
    let entry = entry
        .canonicalize()
        .with_context(|| format!("could not find {}", entry.display()))?;
    let mut compiler = Compiler {
        project_root,
        stack: Vec::new(),
    };
    let app = compiler.module(&entry, ModuleKind::App)?;
    lower::validate_navigation(&app.root).map_err(|error| anyhow::anyhow!(error.render()))?;
    Ok(app)
}

struct Compiler<'a> {
    project_root: &'a Path,
    stack: Vec<PathBuf>,
}

impl Compiler<'_> {
    fn module(&mut self, path: &Path, kind: ModuleKind) -> Result<App> {
        if let Some(start) = self.stack.iter().position(|active| active == path) {
            let mut cycle = self.stack[start..]
                .iter()
                .map(|path| display_path(self.project_root, path))
                .collect::<Vec<_>>();
            cycle.push(display_path(self.project_root, path));
            bail!("screen module import cycle: {}", cycle.join(" -> "));
        }

        self.stack.push(path.to_owned());
        let result = self.lower_and_expand(path, kind);
        self.stack.pop();
        result
    }

    fn lower_and_expand(&mut self, path: &Path, kind: ModuleKind) -> Result<App> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?;
        let source_type = SourceType::from_path(path)
            .with_context(|| format!("{} is not a TypeScript/TSX source path", path.display()))?;
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &source, source_type).parse();

        if !parsed.diagnostics.is_empty() {
            let diagnostics = parsed
                .diagnostics
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            bail!("{}:\n{diagnostics}", path.display());
        }

        let semantic = SemanticBuilder::new_compiler().build(&parsed.program);
        if !semantic.diagnostics.is_empty() {
            let diagnostics = semantic
                .diagnostics
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n");
            bail!("{}:\n{diagnostics}", path.display());
        }

        let app = lower::lower(&parsed.program, self.project_root, path, kind)
            .map_err(|error| anyhow::anyhow!(error.render(path, &source)))?;
        self.expand(app)
    }

    fn expand(&mut self, app: App) -> Result<App> {
        let mut states = app.states;
        let root = self.expand_node(app.root, &mut states)?;
        Ok(App { states, root })
    }

    fn expand_node(&mut self, node: Node, states: &mut Vec<State>) -> Result<Node> {
        Ok(match node {
            Node::Screen {
                children,
                title,
                centered,
            } => Node::Screen {
                children: self.expand_nodes(children, states)?,
                title,
                centered,
            },
            Node::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            } => Node::Stack {
                children: self.expand_nodes(children, states)?,
                axis,
                gap,
                align,
                justify,
            },
            Node::Tabs { state, tabs } => Node::Tabs {
                state,
                tabs: tabs
                    .into_iter()
                    .map(|tab| {
                        Ok(Tab {
                            icon: tab.icon,
                            action: tab.action,
                            screen: Box::new(self.expand_node(*tab.screen, states)?),
                        })
                    })
                    .collect::<Result<_>>()?,
            },
            Node::Navigator { routes } => Node::Navigator {
                routes: routes
                    .into_iter()
                    .map(|route| {
                        Ok(Route {
                            path: route.path,
                            screen: Box::new(self.expand_node(*route.screen, states)?),
                        })
                    })
                    .collect::<Result<_>>()?,
            },
            Node::Conditional {
                condition,
                consequent,
                alternate,
            } => Node::Conditional {
                condition,
                consequent: Box::new(self.expand_node(*consequent, states)?),
                alternate: alternate
                    .map(|node| self.expand_node(*node, states).map(Box::new))
                    .transpose()?,
            },
            Node::ForEach { state, template } => Node::ForEach {
                state,
                template: Box::new(self.expand_node(*template, states)?),
            },
            Node::ScreenModule { path } => {
                let mut screen = self.module(&path, ModuleKind::Screen)?;
                rebase_node(&mut screen.root, states.len());
                states.extend(screen.states);
                screen.root
            }
            node @ (Node::Text { .. }
            | Node::TextInput { .. }
            | Node::Button { .. }
            | Node::Icon { .. }
            | Node::Image { .. }
            | Node::Toggle { .. }) => node,
        })
    }

    fn expand_nodes(&mut self, nodes: Vec<Node>, states: &mut Vec<State>) -> Result<Vec<Node>> {
        nodes
            .into_iter()
            .map(|node| self.expand_node(node, states))
            .collect()
    }
}

fn rebase_node(node: &mut Node, offset: usize) {
    match node {
        Node::Screen { children, .. } | Node::Stack { children, .. } => {
            for child in children {
                rebase_node(child, offset);
            }
        }
        Node::Text { parts, .. } => {
            for part in parts {
                match part {
                    TextPart::State(state) | TextPart::ListLength(state) => rebase(state, offset),
                    TextPart::Literal(_) | TextPart::Item(_) => {}
                }
            }
        }
        Node::TextInput { state, .. } | Node::Toggle { state, .. } => rebase(state, offset),
        Node::Button { action, .. } => {
            if let Some(action) = action {
                rebase_action(action, offset);
            }
        }
        Node::Tabs { state, tabs } => {
            rebase(state, offset);
            for tab in tabs {
                rebase_action(&mut tab.action, offset);
                rebase_node(&mut tab.screen, offset);
            }
        }
        Node::Navigator { routes } => {
            for route in routes {
                rebase_node(&mut route.screen, offset);
            }
        }
        Node::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            match condition {
                Condition::Bool { state, .. } | Condition::ListEmpty { state, .. } => {
                    rebase(state, offset);
                }
            }
            rebase_node(consequent, offset);
            if let Some(alternate) = alternate {
                rebase_node(alternate, offset);
            }
        }
        Node::ForEach { state, template } => {
            rebase(state, offset);
            rebase_node(template, offset);
        }
        Node::Icon { .. } | Node::Image { .. } => {}
        Node::ScreenModule { .. } => unreachable!("nested screen modules are expanded first"),
    }
}

fn rebase_action(action: &mut Action, offset: usize) {
    match action {
        Action::Increment { state, .. }
        | Action::SetInt { state, .. }
        | Action::SetBool { state, .. }
        | Action::Toggle { state }
        | Action::ClearList { state }
        | Action::RemoveListItem { state } => rebase(state, offset),
        Action::SetList { state, value }
        | Action::AppendList { state, value }
        | Action::ReplaceListItem { state, value } => {
            rebase(state, offset);
            rebase_value(value, offset);
        }
        Action::Navigate { .. } => {}
    }
}

fn rebase_value(value: &mut Value, offset: usize) {
    match value {
        Value::State(state) => rebase(state, offset),
        Value::List(values) => {
            for value in values {
                rebase_value(value, offset);
            }
        }
        Value::Object(fields) => {
            for (_, value) in fields {
                rebase_value(value, offset);
            }
        }
        Value::Int(_) | Value::Bool(_) | Value::String(_) | Value::Item(_) => {}
    }
}

fn rebase(state: &mut StateId, offset: usize) {
    state.0 += offset;
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
