use std::{
    collections::{BTreeSet, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use oxc::{allocator::Allocator, parser::Parser, semantic::SemanticBuilder, span::SourceType};

use crate::{
    ir::{
        Action, App, Collection, Condition, Controller, ControllerId, ImageSource, Node,
        PayloadPart, Resource, ResourceId, Route, State, StateId, StateLifetime, Tab, TextPart,
        Value,
    },
    lower::{self, ModuleKind},
    resolver::ModuleResolver,
};

pub fn compile(project_root: &Path, entry: &Path) -> Result<App> {
    let entry = entry
        .canonicalize()
        .with_context(|| format!("could not find {}", entry.display()))?;
    let mut compiler = Compiler {
        project_root,
        stack: Vec::new(),
        extensions: BTreeSet::new(),
        android_permissions: BTreeSet::new(),
        states: Vec::new(),
        resources: Vec::new(),
        application_resources: Vec::new(),
        controllers: Vec::new(),
        application_controllers: Vec::new(),
        keyed_states: HashMap::new(),
        resolver: ModuleResolver::new(project_root),
    };
    let root = compiler.module(&entry, ModuleKind::App)?;
    let mut app = App {
        extensions: compiler.extensions,
        android_permissions: compiler.android_permissions,
        states: compiler.states,
        resources: compiler.resources,
        application_resources: compiler.application_resources,
        controllers: compiler.controllers,
        application_controllers: compiler.application_controllers,
        root,
    };
    lower::validate_navigation(&app.root).map_err(|error| anyhow::anyhow!(error.render()))?;
    validate_audio(&app)?;
    validate_background(&app)?;
    validate_notifications(&app)?;
    bundle_audio_assets(project_root, &mut app.root)?;
    Ok(app)
}

fn validate_background(app: &App) -> Result<()> {
    let mut jobs = HashMap::<String, serde_json::Value>::new();
    let mut ids = HashMap::<u32, String>::new();
    for resource in app
        .resources
        .iter()
        .filter(|resource| resource.module == "background")
    {
        let [PayloadPart::Literal(payload)] = resource.payload.as_slice() else {
            bail!("background resource configuration must be literal");
        };
        let value: serde_json::Value = serde_json::from_str(payload)
            .context("background resource configuration was invalid")?;
        let key = value["key"]
            .as_str()
            .context("background resource configuration had no key")?
            .to_owned();
        if let Some(existing) = jobs.get(&key) {
            if existing != &value {
                bail!("background key {key:?} is declared with different configurations");
            }
            continue;
        }
        if jobs.len() == 16 {
            bail!("an Ink app can declare at most 16 background jobs");
        }
        for kind in ["bootstrap", "periodic"] {
            let id = background_job_id(&key, kind);
            if let Some(existing) = ids.insert(id, format!("{key}:{kind}")) {
                bail!(
                    "background jobs {existing:?} and {:?} have the same scheduler ID; rename one key",
                    format!("{key}:{kind}"),
                );
            }
        }
        jobs.insert(key, value);
    }
    Ok(())
}

pub(crate) fn background_job_id(key: &str, kind: &str) -> u32 {
    format!("ink.background:{kind}:{key}")
        .bytes()
        .fold(0x811c9dc5_u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x01000193)
        })
        & 0x7fff_ffff
}

fn validate_audio(app: &App) -> Result<()> {
    for (kind, name) in [("player", "audio player"), ("recorder", "audio recorder")] {
        let application = app
            .application_controllers
            .iter()
            .filter(|controller| app.controllers[controller.0].kind == kind)
            .count();
        if application + max_active_controllers(&app.root, &app.controllers, kind) > 1 {
            bail!("an Ink screen can activate only one {name}");
        }
    }
    Ok(())
}

fn validate_notifications(app: &App) -> Result<()> {
    if app
        .application_controllers
        .iter()
        .filter(|controller| app.controllers[controller.0].kind == "notification-tap")
        .count()
        > 1
    {
        bail!("an Ink application can declare notificationTap() only once");
    }
    Ok(())
}

fn max_active_controllers(node: &Node, controllers: &[crate::ir::Controller], kind: &str) -> usize {
    match node {
        Node::Screen {
            children,
            controllers: screen_controllers,
            ..
        } => {
            screen_controllers
                .iter()
                .filter(|controller| controllers[controller.0].kind == kind)
                .count()
                + children
                    .iter()
                    .map(|child| max_active_controllers(child, controllers, kind))
                    .sum::<usize>()
        }
        Node::Stack { children, .. } => children
            .iter()
            .map(|child| max_active_controllers(child, controllers, kind))
            .sum(),
        Node::Tabs { tabs, .. } => tabs
            .iter()
            .map(|tab| max_active_controllers(&tab.screen, controllers, kind))
            .max()
            .unwrap_or_default(),
        Node::Navigator { routes } => routes
            .iter()
            .map(|route| max_active_controllers(&route.screen, controllers, kind))
            .max()
            .unwrap_or_default(),
        Node::Conditional {
            consequent,
            alternate,
            ..
        } => max_active_controllers(consequent, controllers, kind).max(
            alternate
                .as_deref()
                .map(|node| max_active_controllers(node, controllers, kind))
                .unwrap_or_default(),
        ),
        Node::ForEach { template, .. } => max_active_controllers(template, controllers, kind),
        Node::Text { .. }
        | Node::TextInput { .. }
        | Node::Button { .. }
        | Node::SelectorButton { .. }
        | Node::Icon { .. }
        | Node::Image { .. }
        | Node::Toggle { .. }
        | Node::ScreenModule { .. } => 0,
    }
}

fn bundle_audio_assets(project_root: &Path, root: &mut Node) -> Result<()> {
    let directory = project_root.join(".ink/android/assets/ink");
    if directory.exists() {
        std::fs::remove_dir_all(&directory)
            .with_context(|| format!("could not clean {}", directory.display()))?;
    }
    bundle_node_assets(root, &directory)
}

fn bundle_node_assets(node: &mut Node, directory: &Path) -> Result<()> {
    match node {
        Node::Screen { children, .. } | Node::Stack { children, .. } => {
            for child in children {
                bundle_node_assets(child, directory)?;
            }
        }
        Node::Button { action, .. } | Node::SelectorButton { action, .. } => {
            if let Some(action) = action {
                bundle_action_assets(action, directory)?;
            }
        }
        Node::Toggle { action, .. } => bundle_action_assets(action, directory)?,
        Node::Tabs { tabs, .. } => {
            for tab in tabs {
                bundle_action_assets(&mut tab.action, directory)?;
                bundle_node_assets(&mut tab.screen, directory)?;
            }
        }
        Node::Navigator { routes } => {
            for route in routes {
                bundle_node_assets(&mut route.screen, directory)?;
            }
        }
        Node::Conditional {
            consequent,
            alternate,
            ..
        } => {
            bundle_node_assets(consequent, directory)?;
            if let Some(alternate) = alternate {
                bundle_node_assets(alternate, directory)?;
            }
        }
        Node::ForEach { template, .. } => bundle_node_assets(template, directory)?,
        Node::Text { .. }
        | Node::TextInput { .. }
        | Node::Icon { .. }
        | Node::Image { .. }
        | Node::ScreenModule { .. } => {}
    }
    Ok(())
}

fn bundle_action_assets(action: &mut Action, directory: &Path) -> Result<()> {
    match action {
        Action::Controller { payload, .. }
        | Action::Native {
            operation: crate::ir::NativeOperation { payload, .. },
        } => bundle_payload_assets(payload, directory)?,
        Action::Sequence(actions) => {
            for action in actions {
                bundle_action_assets(action, directory)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn bundle_payload_assets(payload: &mut [PayloadPart], directory: &Path) -> Result<()> {
    let [PayloadPart::Literal(payload)] = payload else {
        return Ok(());
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return Ok(());
    };
    if rewrite_audio_assets(&mut value, directory)? {
        *payload = value.to_string();
    }
    Ok(())
}

fn rewrite_audio_assets(value: &mut serde_json::Value, directory: &Path) -> Result<bool> {
    match value {
        serde_json::Value::String(value) => {
            let Some(path) = value.strip_prefix("ink-file://") else {
                return Ok(false);
            };
            let source = Path::new(path);
            let extension = source
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("audio");
            let mut hasher = DefaultHasher::new();
            source.hash(&mut hasher);
            let name = format!("{:016x}.{extension}", hasher.finish());
            std::fs::create_dir_all(directory)
                .with_context(|| format!("could not create {}", directory.display()))?;
            std::fs::copy(source, directory.join(&name))
                .with_context(|| format!("could not bundle audio asset {}", source.display()))?;
            *value = format!("asset:///ink/{name}");
            Ok(true)
        }
        serde_json::Value::Array(values) => {
            let mut changed = false;
            for value in values {
                changed |= rewrite_audio_assets(value, directory)?;
            }
            Ok(changed)
        }
        serde_json::Value::Object(fields) => {
            let mut changed = false;
            for value in fields.values_mut() {
                changed |= rewrite_audio_assets(value, directory)?;
            }
            Ok(changed)
        }
        _ => Ok(false),
    }
}

struct Compiler<'a> {
    project_root: &'a Path,
    stack: Vec<PathBuf>,
    extensions: BTreeSet<crate::ir::Extension>,
    android_permissions: BTreeSet<crate::ir::AndroidPermission>,
    states: Vec<State>,
    resources: Vec<Resource>,
    application_resources: Vec<ResourceId>,
    controllers: Vec<Controller>,
    application_controllers: Vec<ControllerId>,
    keyed_states: HashMap<String, StateId>,
    resolver: ModuleResolver,
}

impl Compiler<'_> {
    fn module(&mut self, path: &Path, kind: ModuleKind) -> Result<Node> {
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

    fn lower_and_expand(&mut self, path: &Path, kind: ModuleKind) -> Result<Node> {
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

        let app = lower::lower(&parsed.program, path, kind, &self.resolver)
            .map_err(|error| anyhow::anyhow!(error.render(path, &source)))?;
        self.extensions.extend(app.extensions);
        self.android_permissions.extend(app.android_permissions);
        let mapping = app
            .states
            .into_iter()
            .map(|state| self.register_state(state))
            .collect::<Result<Vec<_>>>()?;
        let resource_mapping = app
            .resources
            .into_iter()
            .map(|mut resource| {
                for part in &mut resource.payload {
                    if let PayloadPart::State(state) = part {
                        remap(state, &mapping);
                    }
                }
                let id = ResourceId(self.resources.len());
                self.resources.push(resource);
                id
            })
            .collect::<Vec<_>>();
        let controller_mapping = app
            .controllers
            .into_iter()
            .map(|mut controller| {
                remap(&mut controller.state, &mapping);
                let id = ControllerId(self.controllers.len());
                self.controllers.push(controller);
                id
            })
            .collect::<Vec<_>>();
        if matches!(kind, ModuleKind::App) {
            self.application_resources = app
                .application_resources
                .into_iter()
                .map(|resource| resource_mapping[resource.0])
                .collect();
            self.application_controllers = app
                .application_controllers
                .into_iter()
                .map(|controller| controller_mapping[controller.0])
                .collect();
        } else {
            self.application_controllers.extend(
                app.application_controllers
                    .into_iter()
                    .map(|controller| controller_mapping[controller.0]),
            );
        }
        let mut root = app.root;
        remap_node(&mut root, &mapping, &resource_mapping, &controller_mapping);
        self.expand_node(root)
    }

    fn register_state(&mut self, state: State) -> Result<StateId> {
        let Some(key) = state_key(&state.lifetime) else {
            let id = StateId(self.states.len());
            self.states.push(state);
            return Ok(id);
        };
        if let Some(id) = self.keyed_states.get(key).copied() {
            let existing = &self.states[id.0];
            if existing.lifetime != state.lifetime {
                bail!(
                    "state key {key:?} is declared as both shared and persisted in {} and {}",
                    display_path(self.project_root, &existing.source.path),
                    display_path(self.project_root, &state.source.path),
                );
            }
            if existing.shape != state.shape {
                bail!(
                    "state key {key:?} has a different type in {} and {}",
                    display_path(self.project_root, &existing.source.path),
                    display_path(self.project_root, &state.source.path),
                );
            }
            if existing.initial != state.initial {
                bail!(
                    "state key {key:?} has a different initial value in {} and {}",
                    display_path(self.project_root, &existing.source.path),
                    display_path(self.project_root, &state.source.path),
                );
            }
            return Ok(id);
        }
        let id = StateId(self.states.len());
        self.keyed_states.insert(key.to_owned(), id);
        self.states.push(state);
        Ok(id)
    }

    fn expand_node(&mut self, node: Node) -> Result<Node> {
        Ok(match node {
            Node::Screen {
                children,
                title,
                centered,
                resources,
                controllers,
            } => Node::Screen {
                children: self.expand_nodes(children)?,
                title,
                centered,
                resources,
                controllers,
            },
            Node::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            } => Node::Stack {
                children: self.expand_nodes(children)?,
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
                            screen: Box::new(self.expand_node(*tab.screen)?),
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
                            screen: Box::new(self.expand_node(*route.screen)?),
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
                consequent: Box::new(self.expand_node(*consequent)?),
                alternate: alternate
                    .map(|node| self.expand_node(*node).map(Box::new))
                    .transpose()?,
            },
            Node::ForEach {
                collection,
                template,
            } => Node::ForEach {
                collection,
                template: Box::new(self.expand_node(*template)?),
            },
            Node::ScreenModule { path } => self.module(&path, ModuleKind::Screen)?,
            node @ (Node::Text { .. }
            | Node::TextInput { .. }
            | Node::Button { .. }
            | Node::SelectorButton { .. }
            | Node::Icon { .. }
            | Node::Image { .. }
            | Node::Toggle { .. }) => node,
        })
    }

    fn expand_nodes(&mut self, nodes: Vec<Node>) -> Result<Vec<Node>> {
        nodes
            .into_iter()
            .map(|node| self.expand_node(node))
            .collect()
    }
}

fn state_key(lifetime: &StateLifetime) -> Option<&str> {
    match lifetime {
        StateLifetime::Local => None,
        StateLifetime::Shared(key) | StateLifetime::Persisted(key) => Some(key),
    }
}

fn remap_node(
    node: &mut Node,
    mapping: &[StateId],
    resource_mapping: &[ResourceId],
    controller_mapping: &[ControllerId],
) {
    match node {
        Node::Screen {
            children,
            resources,
            controllers,
            ..
        } => {
            for resource in resources {
                remap_resource(resource, resource_mapping);
            }
            for controller in controllers {
                remap_controller(controller, controller_mapping);
            }
            for child in children {
                remap_node(child, mapping, resource_mapping, controller_mapping);
            }
        }
        Node::Stack { children, .. } => {
            for child in children {
                remap_node(child, mapping, resource_mapping, controller_mapping);
            }
        }
        Node::Text { parts, .. } => {
            for part in parts {
                match part {
                    TextPart::State(state) | TextPart::ListLength(state) => remap(state, mapping),
                    TextPart::Resource(resource, _) => {
                        remap_resource(resource, resource_mapping);
                    }
                    TextPart::Controller(controller, _) => {
                        remap_controller(controller, controller_mapping);
                    }
                    TextPart::Literal(_) | TextPart::Item(_) => {}
                }
            }
        }
        Node::TextInput { state, .. } => remap(state, mapping),
        Node::Toggle { state, action, .. } => {
            remap(state, mapping);
            remap_action(action, mapping, resource_mapping, controller_mapping);
        }
        Node::Button { label, action, .. } => {
            for part in label {
                match part {
                    TextPart::State(state) | TextPart::ListLength(state) => remap(state, mapping),
                    TextPart::Resource(resource, _) => {
                        remap_resource(resource, resource_mapping);
                    }
                    TextPart::Controller(controller, _) => {
                        remap_controller(controller, controller_mapping);
                    }
                    TextPart::Literal(_) | TextPart::Item(_) => {}
                }
            }
            if let Some(action) = action {
                remap_action(action, mapping, resource_mapping, controller_mapping);
            }
        }
        Node::SelectorButton { value, action, .. } => {
            for part in value {
                match part {
                    TextPart::State(state) | TextPart::ListLength(state) => remap(state, mapping),
                    TextPart::Resource(resource, _) => {
                        remap_resource(resource, resource_mapping);
                    }
                    TextPart::Controller(controller, _) => {
                        remap_controller(controller, controller_mapping);
                    }
                    TextPart::Literal(_) | TextPart::Item(_) => {}
                }
            }
            if let Some(action) = action {
                remap_action(action, mapping, resource_mapping, controller_mapping);
            }
        }
        Node::Tabs { state, tabs } => {
            remap(state, mapping);
            for tab in tabs {
                remap_action(
                    &mut tab.action,
                    mapping,
                    resource_mapping,
                    controller_mapping,
                );
                remap_node(
                    &mut tab.screen,
                    mapping,
                    resource_mapping,
                    controller_mapping,
                );
            }
        }
        Node::Navigator { routes } => {
            for route in routes {
                remap_node(
                    &mut route.screen,
                    mapping,
                    resource_mapping,
                    controller_mapping,
                );
            }
        }
        Node::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            match condition {
                Condition::Bool { state, .. }
                | Condition::ListEmpty { state, .. }
                | Condition::Equals { state, .. } => {
                    remap(state, mapping);
                }
                Condition::ResourceEquals { resource, .. } => {
                    remap_resource(resource, resource_mapping);
                }
                Condition::ControllerEquals { controller, .. } => {
                    remap_controller(controller, controller_mapping);
                }
            }
            remap_node(consequent, mapping, resource_mapping, controller_mapping);
            if let Some(alternate) = alternate {
                remap_node(alternate, mapping, resource_mapping, controller_mapping);
            }
        }
        Node::ForEach {
            collection,
            template,
        } => {
            match collection {
                Collection::State(state) => remap(state, mapping),
                Collection::Resource(resource, _) => remap_resource(resource, resource_mapping),
            }
            remap_node(template, mapping, resource_mapping, controller_mapping);
        }
        Node::Image {
            source: ImageSource::Remote(parts),
            ..
        } => {
            for part in parts {
                match part {
                    TextPart::State(state) | TextPart::ListLength(state) => remap(state, mapping),
                    TextPart::Resource(resource, _) => remap_resource(resource, resource_mapping),
                    TextPart::Controller(controller, _) => {
                        remap_controller(controller, controller_mapping);
                    }
                    TextPart::Literal(_) | TextPart::Item(_) => {}
                }
            }
        }
        Node::Icon { .. } | Node::Image { .. } => {}
        Node::ScreenModule { .. } => {}
    }
}

fn remap_action(
    action: &mut Action,
    mapping: &[StateId],
    resource_mapping: &[ResourceId],
    controller_mapping: &[ControllerId],
) {
    match action {
        Action::Increment { state, .. }
        | Action::SetNumber { state, .. }
        | Action::SetBool { state, .. }
        | Action::SetString { state, .. }
        | Action::Toggle { state }
        | Action::ClearList { state }
        | Action::RemoveListItem { state } => remap(state, mapping),
        Action::SetList { state, value }
        | Action::AppendList { state, value }
        | Action::ReplaceListItem { state, value } => {
            remap(state, mapping);
            remap_value(value, mapping);
        }
        Action::Sequence(actions) => {
            for action in actions {
                remap_action(action, mapping, resource_mapping, controller_mapping);
            }
        }
        Action::ReloadResource { resource } => remap_resource(resource, resource_mapping),
        Action::Native { operation } => {
            for part in &mut operation.payload {
                if let PayloadPart::State(state) = part {
                    remap(state, mapping);
                }
            }
        }
        Action::Controller {
            controller,
            payload,
            ..
        } => {
            remap_controller(controller, controller_mapping);
            for part in payload {
                if let PayloadPart::State(state) = part {
                    remap(state, mapping);
                }
            }
        }
        Action::Navigate { .. } | Action::Back => {}
    }
}

fn remap_resource(resource: &mut ResourceId, mapping: &[ResourceId]) {
    *resource = mapping[resource.0];
}

fn remap_controller(controller: &mut ControllerId, mapping: &[ControllerId]) {
    *controller = mapping[controller.0];
}

fn remap_value(value: &mut Value, mapping: &[StateId]) {
    match value {
        Value::State(state) => remap(state, mapping),
        Value::List(values) => {
            for value in values {
                remap_value(value, mapping);
            }
        }
        Value::Object(fields) => {
            for (_, value) in fields {
                remap_value(value, mapping);
            }
        }
        Value::Number(_) | Value::Bool(_) | Value::String(_) | Value::Item(_) => {}
    }
}

fn remap(state: &mut StateId, mapping: &[StateId]) {
    *state = mapping[state.0];
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
