use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use oxc::{
    ast::ast::{
        Argument, ArrowFunctionBody, BindingPattern, ExportDefaultDeclarationKind, Expression,
        Function, ImportDeclarationSpecifier, JSXAttribute, JSXAttributeItem, JSXAttributeName,
        JSXAttributeValue, JSXChild, JSXElement, JSXElementName, JSXExpression, ObjectPropertyKind,
        PropertyKey, PropertyKind, Statement, TSLiteral, TSSignature, TSType,
        VariableDeclarationKind,
    },
    span::{GetSpan, Span},
    syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator},
};

use crate::{
    diagnostic::CompileError,
    ir::{
        Action, Alignment, AndroidPermission, App, Axis, CameraPreviewKind, Collection, Condition,
        Controller, ControllerId, Extension, ImageFit, ImageSource, Justification, NativeOperation,
        Node, PayloadPart, Resource, ResourceField, ResourceId, ResourceProtocol, Route,
        RouteArgument, SourceSpan, State, StateId, StateLifetime, StateLiteral, StateShape,
        StateValue, Tab, TextAlignment, TextPart, Tone, Value, ValueOperator,
    },
    module_schema::{self, ExportKind, ExtensionElement, ExtensionFunction},
    resolver::ModuleResolver,
};

const INK_IMPORTS: [&str; 21] = [
    "Button",
    "Icon",
    "Image",
    "Navigator",
    "Route",
    "Screen",
    "Field",
    "Stack",
    "Tab",
    "Tabs",
    "Text",
    "TextInput",
    "Toggle",
    "back",
    "all",
    "computed",
    "match",
    "persistedState",
    "routeParams",
    "sharedState",
    "state",
];

#[derive(Clone, PartialEq)]
struct StateBinding {
    id: StateId,
    kind: StateShape,
}

#[derive(Clone)]
struct ResourceBinding {
    id: ResourceId,
    shape: StateShape,
    request: Option<NativeOperation>,
    protocol: ResourceProtocol,
}

struct ResourceValueBinding {
    resource: ResourceId,
    field: ResourceField,
    kind: StateShape,
}

struct ResourceInitialiser {
    definition: Resource,
    request: Option<NativeOperation>,
    android_permission: Option<AndroidPermission>,
}

#[derive(Clone)]
struct ControllerBinding {
    id: ControllerId,
    kind: NativeControllerKind,
    shape: StateShape,
}

struct ControllerValueBinding {
    controller: ControllerId,
    path: Vec<String>,
    kind: StateShape,
    controller_kind: NativeControllerKind,
}

struct ControllerInitialiser {
    definition: Controller,
    kind: NativeControllerKind,
    initial: StateValue,
    shape: StateShape,
}

#[derive(Clone)]
struct ComputedBinding {
    value: Value,
    kind: StateShape,
}

#[derive(Clone)]
struct CombinedBinding {
    resources: Vec<(String, ResourceBinding)>,
}

#[derive(Clone)]
struct RouteParamsBinding {
    fields: BTreeMap<String, StateShape>,
}

#[derive(Clone, Default)]
struct Bindings {
    states: HashMap<String, StateBinding>,
    computed: HashMap<String, ComputedBinding>,
    combined: HashMap<String, CombinedBinding>,
    route_params: HashMap<String, RouteParamsBinding>,
    resources: HashMap<String, ResourceBinding>,
    controllers: HashMap<String, ControllerBinding>,
    extension_functions: HashMap<String, ExtensionFunction>,
    source_path: PathBuf,
}

impl Bindings {
    fn get(&self, name: &str) -> Option<&StateBinding> {
        self.states.get(name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NativeControllerKind {
    Level,
    Pitch,
    Player,
    Recorder,
    Notifications,
    NotificationTap,
    Photo,
    Scanner,
    RingtoneInstaller,
    LightPush,
}

#[derive(Clone)]
enum MatchBinding {
    Resource(ResourceBinding),
    Controller(ControllerBinding),
    Combined(CombinedBinding),
}

#[derive(Clone, Copy)]
struct ItemBinding<'a> {
    name: &'a str,
    kind: &'a StateShape,
}

enum BooleanValue {
    Literal(bool),
    Dynamic(Condition),
}

#[derive(Clone, Copy)]
pub enum ModuleKind {
    App,
    Screen,
}

struct Imports {
    extensions: BTreeSet<Extension>,
    extension_functions: HashMap<String, ExtensionFunction>,
    extension_elements: HashMap<String, ExtensionElement>,
    ink: HashSet<String>,
    screens: HashMap<String, PathBuf>,
    type_aliases: HashMap<String, StateShape>,
    source_path: PathBuf,
}

pub fn lower(
    program: &oxc::ast::ast::Program<'_>,
    source_path: &Path,
    kind: ModuleKind,
    resolver: &ModuleResolver,
) -> Result<App, CompileError> {
    let imports = validate_imports(program, source_path, resolver)?;
    let function = app_function(program)?;
    lower_function(function, &imports, kind)
}

fn validate_imports(
    program: &oxc::ast::ast::Program<'_>,
    source_path: &Path,
    resolver: &ModuleResolver,
) -> Result<Imports, CompileError> {
    let allowed = HashSet::from(INK_IMPORTS);
    let mut extensions = BTreeSet::new();
    let mut extension_functions = HashMap::new();
    let mut extension_elements = HashMap::new();
    let mut ink = HashSet::new();
    let mut screens = HashMap::new();
    let mut type_aliases = HashMap::new();

    for statement in &program.body {
        if let Statement::TSTypeAliasDeclaration(alias) = statement {
            if alias.type_parameters.is_some() {
                return Err(CompileError::new(
                    "Ink data types cannot have type parameters",
                    alias.span,
                ));
            }
            let name = alias.id.name.to_string();
            let shape = state_kind_from_type(&alias.type_annotation)?;
            if type_aliases.insert(name.clone(), shape).is_some() {
                return Err(CompileError::new(
                    format!("type {name} is declared twice"),
                    alias.span,
                ));
            }
            continue;
        }
        let Statement::ImportDeclaration(declaration) = statement else {
            continue;
        };
        let source = declaration.source.value.as_str();
        let Some(specifiers) = &declaration.specifiers else {
            extensions.insert(resolver.extension(source_path, source, declaration.source.span)?);
            continue;
        };

        if source != "ink" {
            if let [ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier)] =
                specifiers.as_slice()
            {
                let local = specifier.local.name.as_str();
                if ink.contains(local)
                    || screens.contains_key(local)
                    || extension_functions.contains_key(local)
                    || extension_elements.contains_key(local)
                {
                    return Err(CompileError::new(
                        format!("{local} is imported twice"),
                        specifier.span,
                    ));
                }
                screens.insert(
                    local.to_owned(),
                    resolver.screen(source_path, source, declaration.source.span)?,
                );
            } else {
                let extension = resolver.extension(source_path, source, declaration.source.span)?;
                extensions.insert(extension);
                for specifier in specifiers {
                    let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                        return Err(CompileError::new(
                            "Ink extensions use named imports",
                            specifier.span(),
                        ));
                    };
                    let imported = specifier.imported.name();
                    if imported.as_str() != specifier.local.name.as_str() {
                        return Err(CompileError::new(
                            "aliased Ink extension imports are not supported yet",
                            specifier.span,
                        ));
                    }
                    let Some(export) = module_schema::export(extension, imported.as_str()) else {
                        return Err(CompileError::new(
                            format!("{imported} is not exported by this Ink extension"),
                            specifier.span,
                        ));
                    };
                    if let ExportKind::Element(element) = export {
                        let local = specifier.local.name.to_string();
                        if ink.contains(&local)
                            || screens.contains_key(&local)
                            || extension_functions.contains_key(&local)
                            || extension_elements.insert(local.clone(), element).is_some()
                        {
                            return Err(CompileError::new(
                                format!("{local} is imported twice"),
                                specifier.span,
                            ));
                        }
                        continue;
                    }
                    let function = match export {
                        ExportKind::Function(function) => function,
                        ExportKind::Element(_) => unreachable!(),
                    };
                    let local = specifier.local.name.to_string();
                    if ink.contains(&local)
                        || screens.contains_key(&local)
                        || extension_elements.contains_key(&local)
                        || extension_functions
                            .insert(local.clone(), function)
                            .is_some()
                    {
                        return Err(CompileError::new(
                            format!("{local} is imported twice"),
                            specifier.span,
                        ));
                    }
                }
            }
            continue;
        }

        for specifier in specifiers {
            let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                return Err(CompileError::new(
                    "use named imports from \"ink\"",
                    specifier.span(),
                ));
            };
            let imported_name = specifier.imported.name();
            let imported_name = imported_name.as_str();
            let local_name = specifier.local.name.as_str();

            if imported_name != local_name {
                return Err(CompileError::new(
                    "aliased Ink imports are not supported yet",
                    specifier.span,
                ));
            }
            if !allowed.contains(imported_name) {
                return Err(CompileError::new(
                    format!("{imported_name} is not an Ink primitive"),
                    specifier.span,
                ));
            }
            if screens.contains_key(local_name)
                || extension_functions.contains_key(local_name)
                || extension_elements.contains_key(local_name)
                || !ink.insert(imported_name.to_owned())
            {
                return Err(CompileError::new(
                    format!("{local_name} is imported twice"),
                    specifier.span,
                ));
            }
        }
    }

    Ok(Imports {
        extensions,
        extension_functions,
        extension_elements,
        ink,
        screens,
        type_aliases,
        source_path: source_path.to_owned(),
    })
}

fn app_function<'a>(
    program: &'a oxc::ast::ast::Program<'a>,
) -> Result<&'a Function<'a>, CompileError> {
    let mut function = None;

    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(_) | Statement::TSTypeAliasDeclaration(_) => {}
            Statement::ExportDefaultDeclaration(export) => {
                let ExportDefaultDeclarationKind::FunctionDeclaration(candidate) =
                    &export.declaration
                else {
                    return Err(CompileError::new(
                        "the default export must be a function declaration",
                        export.span,
                    ));
                };
                if function.replace(candidate.as_ref()).is_some() {
                    return Err(CompileError::new(
                        "an Ink app has exactly one default export",
                        export.span,
                    ));
                }
            }
            _ => {
                return Err(CompileError::new(
                    "only Ink imports and one default app function are allowed at the top level",
                    statement.span(),
                ));
            }
        }
    }

    function.ok_or_else(|| {
        CompileError::new("missing default app function", program.span)
            .with_help("export default function App() { ... }")
    })
}

fn lower_function(
    function: &Function<'_>,
    imports: &Imports,
    kind: ModuleKind,
) -> Result<App, CompileError> {
    if function.r#async || function.generator || !function.params.items.is_empty() {
        return Err(CompileError::new(
            "Ink functions must be synchronous and take no arguments",
            function.span,
        ));
    }
    let body = function
        .body
        .as_ref()
        .ok_or_else(|| CompileError::new("the app function needs a body", function.span))?;
    let mut states = Vec::new();
    let mut resources = Vec::new();
    let mut controllers = Vec::new();
    let mut android_permissions = BTreeSet::new();
    let mut state_names = Bindings {
        extension_functions: imports.extension_functions.clone(),
        source_path: imports.source_path.clone(),
        ..Bindings::default()
    };
    let mut declared_names = HashSet::new();
    let mut root = None;

    for statement in &body.statements {
        match statement {
            Statement::VariableDeclaration(declaration) if root.is_none() => {
                if declaration.kind != VariableDeclarationKind::Const {
                    return Err(CompileError::new(
                        "state declarations must use const",
                        declaration.span,
                    ));
                }
                for declarator in &declaration.declarations {
                    let BindingPattern::BindingIdentifier(binding) = &declarator.id else {
                        return Err(CompileError::new(
                            "state declarations need a simple identifier",
                            declarator.id.span(),
                        ));
                    };
                    let name = binding.name.as_str();
                    if !declared_names.insert(name) {
                        return Err(CompileError::new(
                            format!("{name} is declared twice"),
                            declarator.span,
                        ));
                    }
                    if let Some(params) =
                        route_params_initialiser(declarator.init.as_ref(), imports)?
                    {
                        if matches!(kind, ModuleKind::App) {
                            return Err(CompileError::new(
                                "routeParams<T>() belongs in a screen module",
                                declarator.span,
                            ));
                        }
                        if !state_names.route_params.is_empty() {
                            return Err(CompileError::new(
                                "a screen declares routeParams<T>() once",
                                declarator.span,
                            ));
                        }
                        state_names.route_params.insert(name.to_owned(), params);
                        continue;
                    }
                    if let Some(combined) =
                        combined_initialiser(declarator.init.as_ref(), imports, &state_names)?
                    {
                        state_names.combined.insert(name.to_owned(), combined);
                        continue;
                    }
                    if let Some(computed) =
                        computed_initialiser(declarator.init.as_ref(), imports, &state_names)?
                    {
                        state_names.computed.insert(name.to_owned(), computed);
                        continue;
                    }
                    if let Some(mut controller) =
                        controller_initialiser(declarator.init.as_ref(), imports)?
                    {
                        let state = StateId(states.len());
                        let id = ControllerId(controllers.len());
                        controller.definition.state = state;
                        if matches!(
                            controller.kind,
                            NativeControllerKind::Level
                                | NativeControllerKind::Pitch
                                | NativeControllerKind::Recorder
                        ) {
                            android_permissions.insert(AndroidPermission::Microphone);
                        }
                        if matches!(
                            controller.kind,
                            NativeControllerKind::Photo | NativeControllerKind::Scanner
                        ) {
                            android_permissions.insert(AndroidPermission::Camera);
                        }
                        state_names.controllers.insert(
                            name.to_owned(),
                            ControllerBinding {
                                id,
                                kind: controller.kind,
                                shape: controller.shape.clone(),
                            },
                        );
                        states.push(State {
                            initial: controller.initial,
                            shape: controller.shape,
                            lifetime: StateLifetime::Local,
                            source: SourceSpan {
                                path: imports.source_path.clone(),
                                span: declarator.span,
                            },
                        });
                        controllers.push(controller.definition);
                        continue;
                    }
                    if let Some(resource) =
                        resource_initialiser(declarator.init.as_ref(), imports, &state_names)?
                    {
                        let id = ResourceId(resources.len());
                        if let Some(permission) = resource.android_permission {
                            android_permissions.insert(permission);
                        }
                        state_names.resources.insert(
                            name.to_owned(),
                            ResourceBinding {
                                id,
                                shape: resource.definition.shape.clone(),
                                request: resource.request,
                                protocol: resource.definition.protocol,
                            },
                        );
                        resources.push(resource.definition);
                        continue;
                    }
                    let (initial, kind, lifetime, constructor) =
                        state_initialiser(declarator.init.as_ref(), declarator.span)?;
                    require_import(imports, constructor, declarator.span)?;
                    let binding = StateBinding {
                        id: StateId(states.len()),
                        kind: kind.clone(),
                    };
                    state_names.states.insert(name.to_owned(), binding);
                    states.push(State {
                        initial,
                        shape: kind,
                        lifetime,
                        source: SourceSpan {
                            path: imports.source_path.clone(),
                            span: declarator.span,
                        },
                    });
                }
            }
            Statement::ReturnStatement(statement) if root.is_none() => {
                let Some(argument) = &statement.argument else {
                    return Err(CompileError::new(
                        "the app function must return <Screen>, <Tabs> or <Navigator>",
                        statement.span,
                    ));
                };
                let Expression::JSXElement(element) = unparenthesised(argument) else {
                    return Err(CompileError::new(
                        "the app function must return <Screen>, <Tabs> or <Navigator>",
                        argument.span(),
                    ));
                };
                let node = if element_name(&element.opening_element.name)? == "Navigator" {
                    lower_navigator(element, &state_names, imports)?
                } else {
                    lower_node(element, &state_names, imports, None)?
                };
                match kind {
                    ModuleKind::App
                        if !matches!(
                            node,
                            Node::Screen { .. } | Node::Tabs { .. } | Node::Navigator { .. }
                        ) =>
                    {
                        return Err(CompileError::new(
                            "the app root must be <Screen>, <Tabs> or <Navigator>",
                            element.span,
                        ));
                    }
                    ModuleKind::Screen
                        if !matches!(node, Node::Screen { .. } | Node::ScreenModule { .. }) =>
                    {
                        return Err(CompileError::new(
                            "a screen module must return <Screen>",
                            element.span,
                        ));
                    }
                    _ => {}
                }
                root = Some(node);
            }
            _ => {
                return Err(CompileError::new(
                    "the app body may contain state declarations followed by one return",
                    statement.span(),
                ));
            }
        }
    }

    let mut root = root.ok_or_else(|| match kind {
        ModuleKind::App => CompileError::new(
            "the app function must return <Screen>, <Tabs> or <Navigator>",
            body.span,
        ),
        ModuleKind::Screen => CompileError::new("a screen module must return <Screen>", body.span),
    })?;
    assign_tab_states(&mut root, &mut states, &imports.source_path, function.span);
    let scoped_resources = (0..resources.len()).map(ResourceId).collect::<Vec<_>>();
    let scoped_controllers = (0..controllers.len())
        .map(ControllerId)
        .filter(|controller| {
            !matches!(
                controllers[controller.0].kind.as_str(),
                "notification-tap" | "light-push"
            )
        })
        .collect::<Vec<_>>();
    let event_controllers = (0..controllers.len())
        .map(ControllerId)
        .filter(|controller| {
            matches!(
                controllers[controller.0].kind.as_str(),
                "notification-tap" | "light-push"
            )
        })
        .collect::<Vec<_>>();
    let application_resources = match kind {
        ModuleKind::App => scoped_resources,
        ModuleKind::Screen => {
            let Node::Screen {
                resources: screen_resources,
                ..
            } = &mut root
            else {
                if resources.is_empty() {
                    return Ok(App {
                        extensions: imports.extensions.clone(),
                        android_permissions,
                        states,
                        resources,
                        application_resources: Vec::new(),
                        controllers,
                        application_controllers: Vec::new(),
                        root,
                    });
                }
                return Err(CompileError::new(
                    "a screen with resources must directly return <Screen>",
                    body.span,
                ));
            };
            *screen_resources = scoped_resources;
            Vec::new()
        }
    };
    let application_controllers = match kind {
        ModuleKind::App => scoped_controllers
            .into_iter()
            .chain(event_controllers)
            .collect(),
        ModuleKind::Screen => {
            let Node::Screen {
                controllers: screen_controllers,
                ..
            } = &mut root
            else {
                return Err(CompileError::new(
                    "a screen with native controllers must directly return <Screen>",
                    body.span,
                ));
            };
            *screen_controllers = scoped_controllers;
            event_controllers
        }
    };
    Ok(App {
        extensions: imports.extensions.clone(),
        android_permissions,
        states,
        resources,
        application_resources,
        controllers,
        application_controllers,
        root,
    })
}

fn resource_initialiser(
    initialiser: Option<&Expression<'_>>,
    imports: &Imports,
    states: &Bindings,
) -> Result<Option<ResourceInitialiser>, CompileError> {
    let Some(Expression::CallExpression(call)) = initialiser else {
        return Ok(None);
    };
    let Expression::Identifier(callee) = &call.callee else {
        return Ok(None);
    };
    let Some(function) = imports
        .extension_functions
        .get(callee.name.as_str())
        .copied()
    else {
        return Ok(None);
    };
    if matches!(
        function,
        ExtensionFunction::LevelMeter
            | ExtensionFunction::PitchDetector
            | ExtensionFunction::LocalNotifications
            | ExtensionFunction::NotificationTap
            | ExtensionFunction::OpenDialler
            | ExtensionFunction::RingtoneInstaller
            | ExtensionFunction::LightPush
    ) {
        return Ok(None);
    }
    if !matches!(
        function,
        ExtensionFunction::Json
            | ExtensionFunction::CachedJson
            | ExtensionFunction::Mutation
            | ExtensionFunction::PeriodicJson
    ) && call.type_arguments.is_some()
    {
        return Err(CompileError::new(
            "Ink resources do not take type arguments",
            call.span,
        ));
    }
    let resource = match function {
        ExtensionFunction::LightSdkVersion => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "lightSdkVersion() takes no arguments",
                    call.span,
                ));
            }
            ResourceInitialiser {
                definition: Resource {
                    module: "light-sdk".to_owned(),
                    operation: "version".to_owned(),
                    payload: vec![PayloadPart::Literal(String::new())],
                    shape: StateShape::String,
                    timeout_ms: 10_000,
                    reload_on_resume: true,
                    protocol: ResourceProtocol::Async,
                },
                request: None,
                android_permission: None,
            }
        }
        ExtensionFunction::LightSdkPermission => {
            let [Argument::StringLiteral(permission)] = call.arguments.as_slice() else {
                return Err(CompileError::new(
                    "lightSdkPermission() takes one permission name",
                    call.span,
                ));
            };
            if permission.value.as_str() != "camera" {
                return Err(CompileError::new(
                    "lightSdkPermission() currently supports camera",
                    permission.span,
                ));
            }
            ResourceInitialiser {
                definition: Resource {
                    module: "light-sdk".to_owned(),
                    operation: "permission-status".to_owned(),
                    payload: vec![PayloadPart::Literal("camera".to_owned())],
                    shape: StateShape::String,
                    timeout_ms: 10_000,
                    reload_on_resume: true,
                    protocol: ResourceProtocol::Async,
                },
                request: Some(NativeOperation {
                    module: "light-sdk".to_owned(),
                    operation: "request-permission".to_owned(),
                    payload: vec![PayloadPart::Literal("camera".to_owned())],
                    timeout_ms: 10_000,
                }),
                android_permission: Some(AndroidPermission::Camera),
            }
        }
        ExtensionFunction::Json => network_json_resource(call, imports, states, false)?,
        ExtensionFunction::CachedJson => network_json_resource(call, imports, states, true)?,
        ExtensionFunction::Mutation => network_mutation_resource(call, imports, states)?,
        ExtensionFunction::PeriodicJson => background_json_resource(call, imports)?,
        ExtensionFunction::MicrophonePermission => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "microphonePermission() takes no arguments",
                    call.span,
                ));
            }
            ResourceInitialiser {
                definition: Resource {
                    module: "audio".to_owned(),
                    operation: "permission-status".to_owned(),
                    payload: vec![PayloadPart::Literal(String::new())],
                    shape: StateShape::String,
                    timeout_ms: 10_000,
                    reload_on_resume: true,
                    protocol: ResourceProtocol::Async,
                },
                request: Some(NativeOperation {
                    module: "audio".to_owned(),
                    operation: "request-permission".to_owned(),
                    payload: vec![PayloadPart::Literal(String::new())],
                    timeout_ms: 10_000,
                }),
                android_permission: Some(AndroidPermission::Microphone),
            }
        }
        ExtensionFunction::LocationPermission => {
            let accuracy = location_permission_accuracy(call)?;
            ResourceInitialiser {
                definition: Resource {
                    module: "light-sdk".to_owned(),
                    operation: "permission-status".to_owned(),
                    payload: vec![PayloadPart::Literal(format!("location-{accuracy}"))],
                    shape: StateShape::String,
                    timeout_ms: 10_000,
                    reload_on_resume: true,
                    protocol: ResourceProtocol::Async,
                },
                request: Some(NativeOperation {
                    module: "light-sdk".to_owned(),
                    operation: "request-permission".to_owned(),
                    payload: vec![PayloadPart::Literal(format!("location-{accuracy}"))],
                    timeout_ms: 10_000,
                }),
                android_permission: Some(AndroidPermission::Location),
            }
        }
        ExtensionFunction::CurrentLocation => current_location_resource(call)?,
        ExtensionFunction::NfcTag => nfc_tag_resource(call)?,
        ExtensionFunction::NotificationPermission => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "notificationPermission() takes no arguments",
                    call.span,
                ));
            }
            ResourceInitialiser {
                definition: Resource {
                    module: "notifications".to_owned(),
                    operation: "permission-status".to_owned(),
                    payload: vec![PayloadPart::Literal(String::new())],
                    shape: StateShape::String,
                    timeout_ms: 10_000,
                    reload_on_resume: true,
                    protocol: ResourceProtocol::Async,
                },
                request: Some(NativeOperation {
                    module: "notifications".to_owned(),
                    operation: "request-permission".to_owned(),
                    payload: vec![PayloadPart::Literal(String::new())],
                    timeout_ms: 10_000,
                }),
                android_permission: Some(AndroidPermission::Notifications),
            }
        }
        ExtensionFunction::CameraPermission => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "cameraPermission() takes no arguments",
                    call.span,
                ));
            }
            ResourceInitialiser {
                definition: Resource {
                    module: "camera".to_owned(),
                    operation: "permission-status".to_owned(),
                    payload: vec![PayloadPart::Literal(String::new())],
                    shape: StateShape::String,
                    timeout_ms: 10_000,
                    reload_on_resume: true,
                    protocol: ResourceProtocol::Async,
                },
                request: Some(NativeOperation {
                    module: "camera".to_owned(),
                    operation: "request-permission".to_owned(),
                    payload: vec![PayloadPart::Literal(String::new())],
                    timeout_ms: 10_000,
                }),
                android_permission: Some(AndroidPermission::Camera),
            }
        }
        ExtensionFunction::LevelMeter
        | ExtensionFunction::PitchDetector
        | ExtensionFunction::AudioPlayer
        | ExtensionFunction::AudioRecorder
        | ExtensionFunction::LocalNotifications
        | ExtensionFunction::NotificationTap
        | ExtensionFunction::PhotoCapture
        | ExtensionFunction::CodeScanner
        | ExtensionFunction::OpenDialler
        | ExtensionFunction::RingtoneInstaller
        | ExtensionFunction::LightPush => unreachable!(),
    };
    Ok(Some(resource))
}

fn computed_initialiser(
    initialiser: Option<&Expression<'_>>,
    imports: &Imports,
    bindings: &Bindings,
) -> Result<Option<ComputedBinding>, CompileError> {
    let Some(Expression::CallExpression(call)) = initialiser else {
        return Ok(None);
    };
    let Expression::Identifier(callee) = &call.callee else {
        return Ok(None);
    };
    if callee.name.as_str() != "computed" || !imports.ink.contains("computed") {
        return Ok(None);
    }
    if call.type_arguments.is_some() {
        return Err(CompileError::new(
            "computed() infers its value type",
            call.span,
        ));
    }
    let [argument] = call.arguments.as_slice() else {
        return Err(CompileError::new(
            "computed() takes one expression function",
            call.span,
        ));
    };
    let Some(Expression::ArrowFunctionExpression(function)) = argument.as_expression() else {
        return Err(CompileError::new(
            "computed() takes an arrow function",
            argument.span(),
        ));
    };
    if function.r#async || !function.params.items.is_empty() || function.params.rest.is_some() {
        return Err(CompileError::new(
            "computed() functions must be synchronous and take no arguments",
            function.span,
        ));
    }
    let Some(expression) = function.body.as_expression() else {
        return Err(CompileError::new(
            "computed() directly returns one pure expression",
            function.body.span(),
        ));
    };
    let (value, kind) = computed_expression(expression, bindings)?;
    Ok(Some(ComputedBinding { value, kind }))
}

fn combined_initialiser(
    initialiser: Option<&Expression<'_>>,
    imports: &Imports,
    bindings: &Bindings,
) -> Result<Option<CombinedBinding>, CompileError> {
    let Some(Expression::CallExpression(call)) = initialiser else {
        return Ok(None);
    };
    let Expression::Identifier(callee) = &call.callee else {
        return Ok(None);
    };
    if callee.name.as_str() != "all" || !imports.ink.contains("all") {
        return Ok(None);
    }
    if call.type_arguments.is_some() {
        return Err(CompileError::new(
            "all() infers its resource types",
            call.span,
        ));
    }
    let [Argument::ObjectExpression(object)] = call.arguments.as_slice() else {
        return Err(CompileError::new(
            "all() takes an object of async resources",
            call.span,
        ));
    };
    if object.properties.is_empty() {
        return Err(CompileError::new(
            "all() needs at least one resource",
            object.span,
        ));
    }
    let mut resources = Vec::new();
    let mut names = HashSet::new();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "all() cannot use spreads",
                property.span(),
            ));
        };
        if property.kind != PropertyKind::Init || property.method || property.computed {
            return Err(CompileError::new(
                "all() uses named resource properties",
                property.span,
            ));
        }
        let name = property_name(&property.key)?;
        if !names.insert(name.clone()) {
            return Err(CompileError::new(
                format!("all() resource {name:?} is declared twice"),
                property.span,
            ));
        }
        let Expression::Identifier(value) = &property.value else {
            return Err(CompileError::new(
                "all() property values are resource variables",
                property.value.span(),
            ));
        };
        let resource = bindings.resources.get(value.name.as_str()).ok_or_else(|| {
            CompileError::new(
                format!("{} is not an async resource", value.name),
                value.span,
            )
        })?;
        if resource.protocol != ResourceProtocol::Async {
            return Err(CompileError::new(
                "all() currently combines ordinary async resources",
                value.span,
            ));
        }
        resources.push((name, resource.clone()));
    }
    Ok(Some(CombinedBinding { resources }))
}

fn route_params_initialiser(
    initialiser: Option<&Expression<'_>>,
    imports: &Imports,
) -> Result<Option<RouteParamsBinding>, CompileError> {
    let Some(Expression::CallExpression(call)) = initialiser else {
        return Ok(None);
    };
    let Expression::Identifier(callee) = &call.callee else {
        return Ok(None);
    };
    if callee.name.as_str() != "routeParams" || !imports.ink.contains("routeParams") {
        return Ok(None);
    }
    if !call.arguments.is_empty() {
        return Err(CompileError::new(
            "routeParams<T>() takes no runtime arguments",
            call.span,
        ));
    }
    let Some(arguments) = &call.type_arguments else {
        return Err(CompileError::new(
            "routeParams<T>() needs an object data type",
            call.span,
        ));
    };
    let [params] = arguments.params.as_slice() else {
        return Err(CompileError::new(
            "routeParams<T>() accepts one object data type",
            arguments.span,
        ));
    };
    let shape = match params {
        TSType::TSTypeReference(reference) if reference.type_arguments.is_none() => {
            let oxc::ast::ast::TSTypeName::IdentifierReference(name) = &reference.type_name else {
                return Err(CompileError::new(
                    "route parameters use a local type alias or inline object type",
                    reference.span,
                ));
            };
            imports
                .type_aliases
                .get(name.name.as_str())
                .cloned()
                .ok_or_else(|| {
                    CompileError::new(
                        format!("unknown route parameter type {}", name.name),
                        reference.span,
                    )
                })?
        }
        params => state_kind_from_type(params)?,
    };
    let StateShape::Object(fields) = shape else {
        return Err(CompileError::new(
            "route parameters use an object data type",
            params.span(),
        ));
    };
    if fields.values().any(|shape| !scalar_kind(shape)) {
        return Err(CompileError::new(
            "route parameters are scalar values",
            params.span(),
        ));
    }
    Ok(Some(RouteParamsBinding { fields }))
}

fn computed_expression(
    expression: &Expression<'_>,
    bindings: &Bindings,
) -> Result<(Value, StateShape), CompileError> {
    let expression = unparenthesised(expression);
    match expression {
        Expression::NumericLiteral(value) if value.value.is_finite() => {
            Ok((Value::Number(value.value), StateShape::Number))
        }
        Expression::StringLiteral(value) => {
            Ok((Value::String(value.value.to_string()), StateShape::String))
        }
        Expression::BooleanLiteral(value) => Ok((Value::Bool(value.value), StateShape::Bool)),
        Expression::BinaryExpression(binary) => {
            let (left, left_kind) = computed_expression(&binary.left, bindings)?;
            let (right, right_kind) = computed_expression(&binary.right, bindings)?;
            let operator = match binary.operator {
                BinaryOperator::Addition => ValueOperator::Add,
                BinaryOperator::Subtraction => ValueOperator::Subtract,
                BinaryOperator::Multiplication => ValueOperator::Multiply,
                BinaryOperator::Division => ValueOperator::Divide,
                _ => {
                    return Err(CompileError::new(
                        "computed() supports +, -, * and /",
                        binary.span,
                    ));
                }
            };
            let kind = match operator {
                ValueOperator::Add
                    if scalar_base_kind(&left_kind) == Some(StateShape::String)
                        && scalar_base_kind(&right_kind) == Some(StateShape::String) =>
                {
                    StateShape::String
                }
                _ if scalar_base_kind(&left_kind) == Some(StateShape::Number)
                    && scalar_base_kind(&right_kind) == Some(StateShape::Number) =>
                {
                    StateShape::Number
                }
                _ => {
                    return Err(CompileError::new(
                        "computed arithmetic uses two numbers, or + with two strings",
                        binary.span,
                    ));
                }
            };
            Ok((
                Value::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                },
                kind,
            ))
        }
        _ => {
            if let Some(value) = expression_derived_value(expression, bindings)? {
                return Ok(value);
            }
            if let Ok(state) = list_length_state(expression, bindings) {
                return Ok((Value::ListLength(state), StateShape::Number));
            }
            if let Some(value) = expression_controller_value(expression, bindings)?
                && scalar_kind(&value.kind)
            {
                return Ok((Value::Controller(value.controller, value.path), value.kind));
            }
            if let Some(value) = expression_resource_value(expression, bindings)?
                && scalar_kind(&value.kind)
            {
                return Ok((Value::Resource(value.resource, value.field), value.kind));
            }
            let value = expression_state_value(expression, bindings).map_err(|_| {
                CompileError::new(
                    "computed() expressions use scalar state, resource or controller values",
                    expression.span(),
                )
            })?;
            if !scalar_kind(&value.kind) {
                return Err(CompileError::new(
                    "computed() values must be scalar",
                    expression.span(),
                ));
            }
            Ok((Value::State(value.id), value.kind))
        }
    }
}

fn scalar_base_kind(shape: &StateShape) -> Option<StateShape> {
    match shape {
        StateShape::Number => Some(StateShape::Number),
        StateShape::Bool => Some(StateShape::Bool),
        StateShape::String => Some(StateShape::String),
        StateShape::Literal(StateLiteral::Number(_)) => Some(StateShape::Number),
        StateShape::Literal(StateLiteral::Bool(_)) => Some(StateShape::Bool),
        StateShape::Literal(StateLiteral::String(_)) => Some(StateShape::String),
        _ => None,
    }
}

fn expression_computed_value(
    expression: &Expression<'_>,
    bindings: &Bindings,
) -> Result<Option<ComputedBinding>, CompileError> {
    let Some((name, path)) = member_path(expression) else {
        return Ok(None);
    };
    let Some(computed) = bindings.computed.get(name) else {
        return Ok(None);
    };
    if path.as_slice() != ["value"] {
        return Err(CompileError::new(
            "computed values expose only .value",
            expression.span(),
        ));
    }
    Ok(Some(computed.clone()))
}

fn expression_route_param(
    expression: &Expression<'_>,
    bindings: &Bindings,
) -> Result<Option<(String, StateShape)>, CompileError> {
    let Some((name, path)) = member_path(expression) else {
        return Ok(None);
    };
    let Some(params) = bindings.route_params.get(name) else {
        return Ok(None);
    };
    let [field] = path.as_slice() else {
        return Err(CompileError::new(
            "route parameters are read by field name",
            expression.span(),
        ));
    };
    let shape = params
        .fields
        .get(field)
        .ok_or_else(|| CompileError::new("unknown route parameter", expression.span()))?;
    Ok(Some((field.clone(), shape.clone())))
}

fn expression_derived_value(
    expression: &Expression<'_>,
    bindings: &Bindings,
) -> Result<Option<(Value, StateShape)>, CompileError> {
    if let Some((name, kind)) = expression_route_param(expression, bindings)? {
        return Ok(Some((Value::RouteParam(name), kind)));
    }
    if let Some(computed) = expression_computed_value(expression, bindings)? {
        return Ok(Some((computed.value, computed.kind)));
    }
    let Some((name, path)) = member_path(expression) else {
        return Ok(None);
    };
    let Some(combined) = bindings.combined.get(name) else {
        return Ok(None);
    };
    let ids = || {
        combined
            .resources
            .iter()
            .map(|(_, resource)| resource.id)
            .collect()
    };
    match path.as_slice() {
        [status] if status == "status" => Ok(Some((
            Value::CombinedStatus(ids()),
            status_shape(&["loading", "ready", "error"]),
        ))),
        [error, resource] if error == "error" && resource == "resource" => Ok(Some((
            Value::CombinedErrorResource(
                combined
                    .resources
                    .iter()
                    .map(|(name, resource)| (name.clone(), resource.id))
                    .collect(),
            ),
            StateShape::String,
        ))),
        [outer, inner, field] if outer == "error" && inner == "error" => {
            let (field, shape) = match field.as_str() {
                "kind" => (ResourceField::ErrorKind, StateShape::String),
                "message" => (ResourceField::ErrorMessage, StateShape::String),
                "retryable" => (ResourceField::ErrorRetryable, StateShape::Bool),
                _ => {
                    return Err(CompileError::new(
                        "combined errors expose kind, message and retryable",
                        expression.span(),
                    ));
                }
            };
            Ok(Some((Value::CombinedErrorField(ids(), field), shape)))
        }
        _ => Ok(None),
    }
}

fn location_permission_accuracy(
    call: &oxc::ast::ast::CallExpression<'_>,
) -> Result<&'static str, CompileError> {
    match call.arguments.as_slice() {
        [] => Ok("precise"),
        [Argument::StringLiteral(accuracy)] => match accuracy.value.as_str() {
            "approximate" => Ok("approximate"),
            "precise" => Ok("precise"),
            _ => Err(CompileError::new(
                "location accuracy must be \"approximate\" or \"precise\"",
                accuracy.span,
            )),
        },
        _ => Err(CompileError::new(
            "locationPermission() accepts an optional accuracy",
            call.span,
        )),
    }
}

fn current_location_resource(
    call: &oxc::ast::ast::CallExpression<'_>,
) -> Result<ResourceInitialiser, CompileError> {
    let mut accuracy = "precise";
    let mut max_age_ms = 60_000_u64;
    let mut timeout_ms = 15_000_u64;
    match call.arguments.as_slice() {
        [] => {}
        [Argument::ObjectExpression(options)] => {
            let mut seen = HashSet::new();
            for property in &options.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return Err(CompileError::new(
                        "location options cannot use spreads",
                        property.span(),
                    ));
                };
                let name = property_name(&property.key)?;
                if !seen.insert(name.clone()) {
                    return Err(CompileError::new(
                        format!("location option {name:?} is declared twice"),
                        property.span,
                    ));
                }
                match name.as_str() {
                    "accuracy" => {
                        let Expression::StringLiteral(value) = &property.value else {
                            return Err(CompileError::new(
                                "location accuracy must be a string literal",
                                property.value.span(),
                            ));
                        };
                        accuracy = match value.value.as_str() {
                            "approximate" => "approximate",
                            "precise" => "precise",
                            _ => {
                                return Err(CompileError::new(
                                    "location accuracy must be \"approximate\" or \"precise\"",
                                    value.span,
                                ));
                            }
                        };
                    }
                    "maxAgeMs" => {
                        let Expression::NumericLiteral(value) = &property.value else {
                            return Err(CompileError::new(
                                "maxAgeMs must be a number literal",
                                property.value.span(),
                            ));
                        };
                        max_age_ms = integer(value.value, value.span, "maxAgeMs")?
                            .try_into()
                            .ok()
                            .filter(|value: &u64| *value <= 3_600_000)
                            .ok_or_else(|| {
                                CompileError::new(
                                    "maxAgeMs must be between 0 and 3600000",
                                    value.span,
                                )
                            })?;
                    }
                    "timeoutMs" => {
                        let Expression::NumericLiteral(value) = &property.value else {
                            return Err(CompileError::new(
                                "timeoutMs must be a number literal",
                                property.value.span(),
                            ));
                        };
                        timeout_ms = integer(value.value, value.span, "timeoutMs")?
                            .try_into()
                            .ok()
                            .filter(|value: &u64| (1_000..=120_000).contains(value))
                            .ok_or_else(|| {
                                CompileError::new(
                                    "timeoutMs must be between 1000 and 120000",
                                    value.span,
                                )
                            })?;
                    }
                    _ => {
                        return Err(CompileError::new(
                            format!("unknown location option {name:?}"),
                            property.key.span(),
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(CompileError::new(
                "currentLocation() accepts an optional { accuracy, maxAgeMs, timeoutMs } object",
                call.span,
            ));
        }
    }
    let payload = format!("{{\"accuracy\":\"{accuracy}\",\"maxAgeMs\":{max_age_ms}}}");
    Ok(ResourceInitialiser {
        definition: Resource {
            module: "location".to_owned(),
            operation: "current".to_owned(),
            payload: vec![PayloadPart::Literal(payload)],
            shape: object_shape([
                ("latitude", StateShape::Number),
                ("longitude", StateShape::Number),
                ("accuracy", StateShape::Number),
                ("provider", StateShape::String),
                ("timestamp", StateShape::Number),
            ]),
            timeout_ms,
            reload_on_resume: true,
            protocol: ResourceProtocol::Async,
        },
        request: None,
        android_permission: Some(AndroidPermission::Location),
    })
}

fn nfc_tag_resource(
    call: &oxc::ast::ast::CallExpression<'_>,
) -> Result<ResourceInitialiser, CompileError> {
    let mut timeout_ms = 30_000_u64;
    match call.arguments.as_slice() {
        [] => {}
        [Argument::ObjectExpression(options)] => {
            let mut seen = HashSet::new();
            for property in &options.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return Err(CompileError::new(
                        "NFC options cannot use spreads",
                        property.span(),
                    ));
                };
                let name = property_name(&property.key)?;
                if !seen.insert(name.clone()) {
                    return Err(CompileError::new(
                        format!("NFC option {name:?} is declared twice"),
                        property.span,
                    ));
                }
                match name.as_str() {
                    "timeoutMs" => {
                        let Expression::NumericLiteral(value) = &property.value else {
                            return Err(CompileError::new(
                                "timeoutMs must be a number literal",
                                property.value.span(),
                            ));
                        };
                        timeout_ms = integer(value.value, value.span, "timeoutMs")?
                            .try_into()
                            .ok()
                            .filter(|value: &u64| (1_000..=120_000).contains(value))
                            .ok_or_else(|| {
                                CompileError::new(
                                    "timeoutMs must be between 1000 and 120000",
                                    value.span,
                                )
                            })?;
                    }
                    _ => {
                        return Err(CompileError::new(
                            format!("unknown NFC option {name:?}"),
                            property.key.span(),
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(CompileError::new(
                "nfcTag() accepts an optional { timeoutMs } object",
                call.span,
            ));
        }
    }

    let record_shape = object_shape([
        ("kind", StateShape::String),
        ("value", StateShape::String),
        ("languageTag", StateShape::String),
        ("mimeType", StateShape::String),
        ("payloadBase64", StateShape::String),
    ]);
    Ok(ResourceInitialiser {
        definition: Resource {
            module: "nfc".to_owned(),
            operation: "read".to_owned(),
            payload: vec![PayloadPart::Literal(String::new())],
            shape: object_shape([
                ("serialNumber", StateShape::String),
                ("hasText", StateShape::Bool),
                ("text", StateShape::String),
                ("hasUri", StateShape::Bool),
                ("uri", StateShape::String),
                ("records", StateShape::List(Box::new(record_shape))),
            ]),
            timeout_ms,
            reload_on_resume: false,
            protocol: ResourceProtocol::Async,
        },
        request: None,
        android_permission: Some(AndroidPermission::Nfc),
    })
}

fn controller_initialiser(
    initialiser: Option<&Expression<'_>>,
    imports: &Imports,
) -> Result<Option<ControllerInitialiser>, CompileError> {
    let Some(Expression::CallExpression(call)) = initialiser else {
        return Ok(None);
    };
    let Expression::Identifier(callee) = &call.callee else {
        return Ok(None);
    };
    let Some(function) = imports
        .extension_functions
        .get(callee.name.as_str())
        .copied()
    else {
        return Ok(None);
    };
    if !matches!(
        function,
        ExtensionFunction::LevelMeter
            | ExtensionFunction::PitchDetector
            | ExtensionFunction::AudioPlayer
            | ExtensionFunction::AudioRecorder
            | ExtensionFunction::LocalNotifications
            | ExtensionFunction::NotificationTap
            | ExtensionFunction::PhotoCapture
            | ExtensionFunction::CodeScanner
            | ExtensionFunction::RingtoneInstaller
            | ExtensionFunction::LightPush
    ) {
        return Ok(None);
    }
    if call.type_arguments.is_some() {
        return Err(CompileError::new(
            "native controllers do not take type arguments",
            call.span,
        ));
    }

    let (kind, config, shape, initial) = match function {
        ExtensionFunction::LevelMeter => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "levelMeter() takes no arguments",
                    call.span,
                ));
            }
            let shape = object_shape([
                (
                    "status",
                    status_shape(&["idle", "listening", "active", "clipping", "error"]),
                ),
                ("rms", StateShape::Number),
                ("peak", StateShape::Number),
                ("error", ink_error_shape()),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("rms", StateValue::Number(0.0)),
                ("peak", StateValue::Number(0.0)),
                ("error", ink_error_value()),
            ]);
            (NativeControllerKind::Level, "{}".to_owned(), shape, initial)
        }
        ExtensionFunction::PitchDetector => {
            let reference_hz = pitch_reference(call)?;
            let shape = object_shape([
                (
                    "status",
                    status_shape(&["idle", "listening", "active", "error"]),
                ),
                ("frequencyHz", StateShape::Number),
                ("note", StateShape::String),
                ("octave", StateShape::Number),
                ("cents", StateShape::Number),
                ("confidence", StateShape::Number),
                ("error", ink_error_shape()),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("frequencyHz", StateValue::Number(0.0)),
                ("note", StateValue::String(String::new())),
                ("octave", StateValue::Number(0.0)),
                ("cents", StateValue::Number(0.0)),
                ("confidence", StateValue::Number(0.0)),
                ("error", ink_error_value()),
            ]);
            (
                NativeControllerKind::Pitch,
                format!("{{\"referenceHz\":{reference_hz}}}"),
                shape,
                initial,
            )
        }
        ExtensionFunction::AudioPlayer => {
            let (usage, playback) = player_options(call)?;
            let shape = object_shape([
                (
                    "status",
                    status_shape(&["idle", "loading", "paused", "playing", "ended", "error"]),
                ),
                ("id", StateShape::String),
                ("src", StateShape::String),
                ("title", StateShape::String),
                ("artist", StateShape::String),
                ("album", StateShape::String),
                ("artwork", StateShape::String),
                ("index", StateShape::Number),
                ("positionMs", StateShape::Number),
                ("durationMs", StateShape::Number),
                ("bufferedMs", StateShape::Number),
                ("speed", StateShape::Number),
                ("error", ink_error_shape()),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("id", StateValue::String(String::new())),
                ("src", StateValue::String(String::new())),
                ("title", StateValue::String(String::new())),
                ("artist", StateValue::String(String::new())),
                ("album", StateValue::String(String::new())),
                ("artwork", StateValue::String(String::new())),
                ("index", StateValue::Number(-1.0)),
                ("positionMs", StateValue::Number(0.0)),
                ("durationMs", StateValue::Number(0.0)),
                ("bufferedMs", StateValue::Number(0.0)),
                ("speed", StateValue::Number(1.0)),
                ("error", ink_error_value()),
            ]);
            (
                NativeControllerKind::Player,
                format!("{{\"usage\":\"{usage}\",\"playback\":\"{playback}\"}}"),
                shape,
                initial,
            )
        }
        ExtensionFunction::AudioRecorder => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "audioRecorder() takes no arguments",
                    call.span,
                ));
            }
            let shape = object_shape([
                (
                    "status",
                    status_shape(&["idle", "recording", "stopping", "ready", "error"]),
                ),
                ("durationMs", StateShape::Number),
                ("id", StateShape::String),
                ("src", StateShape::String),
                ("recordingDurationMs", StateShape::Number),
                ("error", ink_error_shape()),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("durationMs", StateValue::Number(0.0)),
                ("id", StateValue::String(String::new())),
                ("src", StateValue::String(String::new())),
                ("recordingDurationMs", StateValue::Number(0.0)),
                ("error", ink_error_value()),
            ]);
            (
                NativeControllerKind::Recorder,
                "{}".to_owned(),
                shape,
                initial,
            )
        }
        ExtensionFunction::LocalNotifications => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "localNotifications() takes no arguments",
                    call.span,
                ));
            }
            let shape = object_shape([
                ("status", status_shape(&["idle", "error"])),
                ("operation", StateShape::String),
                ("id", StateShape::String),
                ("error", ink_error_shape()),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("operation", StateValue::String(String::new())),
                ("id", StateValue::String(String::new())),
                ("error", ink_error_value()),
            ]);
            (
                NativeControllerKind::Notifications,
                "{}".to_owned(),
                shape,
                initial,
            )
        }
        ExtensionFunction::NotificationTap => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "notificationTap() takes no arguments",
                    call.span,
                ));
            }
            let shape = object_shape([
                ("status", status_shape(&["empty", "ready"])),
                (
                    "value",
                    object_shape([("id", StateShape::String), ("data", StateShape::String)]),
                ),
            ]);
            let initial = object_value([
                ("status", StateValue::String("empty".to_owned())),
                (
                    "value",
                    object_value([
                        ("id", StateValue::String(String::new())),
                        ("data", StateValue::String(String::new())),
                    ]),
                ),
            ]);
            (
                NativeControllerKind::NotificationTap,
                "{}".to_owned(),
                shape,
                initial,
            )
        }
        ExtensionFunction::PhotoCapture => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "photoCapture() takes no arguments",
                    call.span,
                ));
            }
            camera_controller(NativeControllerKind::Photo, "{}".to_owned())
        }
        ExtensionFunction::CodeScanner => {
            camera_controller(NativeControllerKind::Scanner, scanner_config(call)?)
        }
        ExtensionFunction::RingtoneInstaller => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "ringtoneInstaller() takes no arguments",
                    call.span,
                ));
            }
            let shape = object_shape([
                (
                    "status",
                    status_shape(&["idle", "installing", "installed", "error"]),
                ),
                ("error", ink_error_shape()),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("error", ink_error_value()),
            ]);
            (
                NativeControllerKind::RingtoneInstaller,
                "{}".to_owned(),
                shape,
                initial,
            )
        }
        ExtensionFunction::LightPush => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "lightPush() takes no arguments",
                    call.span,
                ));
            }
            let message = object_shape([
                ("id", StateShape::String),
                ("groupKey", StateShape::String),
                ("title", StateShape::String),
                ("body", StateShape::String),
                ("route", StateShape::String),
                ("receivedAtMs", StateShape::Number),
            ]);
            let shape = object_shape([
                (
                    "status",
                    status_shape(&["idle", "registering", "synchronising", "ready", "error"]),
                ),
                ("endpoint", StateShape::String),
                ("registeredAtMs", StateShape::Number),
                ("openedKey", StateShape::String),
                ("messages", StateShape::List(Box::new(message))),
                ("error", ink_error_shape()),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("endpoint", StateValue::String(String::new())),
                ("registeredAtMs", StateValue::Number(0.0)),
                ("openedKey", StateValue::String(String::new())),
                ("messages", StateValue::List(Vec::new())),
                ("error", ink_error_value()),
            ]);
            (
                NativeControllerKind::LightPush,
                "{}".to_owned(),
                shape,
                initial,
            )
        }
        _ => unreachable!(),
    };
    Ok(Some(ControllerInitialiser {
        definition: Controller {
            state: StateId(0),
            module: match kind {
                NativeControllerKind::Notifications
                | NativeControllerKind::NotificationTap
                | NativeControllerKind::LightPush => "notifications",
                NativeControllerKind::Photo | NativeControllerKind::Scanner => "camera",
                NativeControllerKind::RingtoneInstaller => "light-sdk",
                _ => "audio",
            }
            .to_owned(),
            kind: match kind {
                NativeControllerKind::Level => "level",
                NativeControllerKind::Pitch => "pitch",
                NativeControllerKind::Player => "player",
                NativeControllerKind::Recorder => "recorder",
                NativeControllerKind::Notifications => "local-notifications",
                NativeControllerKind::NotificationTap => "notification-tap",
                NativeControllerKind::Photo => "photo",
                NativeControllerKind::Scanner => "scanner",
                NativeControllerKind::RingtoneInstaller => "ringtone-installer",
                NativeControllerKind::LightPush => "light-push",
            }
            .to_owned(),
            config,
        },
        kind,
        initial,
        shape,
    }))
}

fn camera_controller(
    kind: NativeControllerKind,
    config: String,
) -> (NativeControllerKind, String, StateShape, StateValue) {
    let value_shape = match kind {
        NativeControllerKind::Photo => object_shape([
            ("source", StateShape::String),
            ("width", StateShape::Number),
            ("height", StateShape::Number),
            ("mimeType", StateShape::String),
            ("capturedAtMs", StateShape::Number),
        ]),
        NativeControllerKind::Scanner => {
            object_shape([("text", StateShape::String), ("format", StateShape::String)])
        }
        _ => unreachable!(),
    };
    let value = match kind {
        NativeControllerKind::Photo => object_value([
            ("source", StateValue::String(String::new())),
            ("width", StateValue::Number(0.0)),
            ("height", StateValue::Number(0.0)),
            ("mimeType", StateValue::String("image/jpeg".to_owned())),
            ("capturedAtMs", StateValue::Number(0.0)),
        ]),
        NativeControllerKind::Scanner => object_value([
            ("text", StateValue::String(String::new())),
            ("format", StateValue::String("qr".to_owned())),
        ]),
        _ => unreachable!(),
    };
    (
        kind,
        config,
        object_shape([
            (
                "status",
                status_shape(&["idle", "opening", "active", "ready", "error"]),
            ),
            ("value", value_shape),
            ("error", ink_error_shape()),
        ]),
        object_value([
            ("status", StateValue::String("idle".to_owned())),
            ("value", value),
            ("error", ink_error_value()),
        ]),
    )
}

fn ink_error_shape() -> StateShape {
    object_shape([
        ("kind", StateShape::String),
        ("message", StateShape::String),
        ("retryable", StateShape::Bool),
    ])
}

fn ink_error_value() -> StateValue {
    object_value([
        ("kind", StateValue::String("unexpected".to_owned())),
        ("message", StateValue::String(String::new())),
        ("retryable", StateValue::Bool(false)),
    ])
}

fn scanner_config(call: &oxc::ast::ast::CallExpression<'_>) -> Result<String, CompileError> {
    const FORMATS: [&str; 13] = [
        "qr",
        "aztec",
        "data-matrix",
        "pdf417",
        "codabar",
        "code-39",
        "code-93",
        "code-128",
        "ean-8",
        "ean-13",
        "itf",
        "upc-a",
        "upc-e",
    ];
    let formats = match call.arguments.as_slice() {
        [] => vec!["qr".to_owned()],
        [Argument::ObjectExpression(options)] => match options.properties.as_slice() {
            [] => vec!["qr".to_owned()],
            [ObjectPropertyKind::ObjectProperty(property)] => {
                if property_name(&property.key)? != "formats" {
                    return Err(CompileError::new(
                        "codeScanner() accepts only formats",
                        property.key.span(),
                    ));
                }
                let Expression::ArrayExpression(values) = &property.value else {
                    return Err(CompileError::new(
                        "scanner formats must be an array literal",
                        property.value.span(),
                    ));
                };
                if values.elements.is_empty() {
                    return Err(CompileError::new(
                        "scanner formats must not be empty",
                        values.span,
                    ));
                }
                let mut seen = HashSet::new();
                values
                    .elements
                    .iter()
                    .map(|element| {
                        let Some(Expression::StringLiteral(value)) = element.as_expression() else {
                            return Err(CompileError::new(
                                "scanner formats must be string literals",
                                element.span(),
                            ));
                        };
                        let format = value.value.as_str();
                        if !FORMATS.contains(&format) {
                            return Err(CompileError::new(
                                format!("unknown scanner format {format:?}"),
                                value.span,
                            ));
                        }
                        if !seen.insert(format) {
                            return Err(CompileError::new(
                                format!("scanner format {format:?} is declared twice"),
                                value.span,
                            ));
                        }
                        Ok(format.to_owned())
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => {
                return Err(CompileError::new(
                    "codeScanner() accepts only { formats }",
                    options.span,
                ));
            }
        },
        _ => {
            return Err(CompileError::new(
                "codeScanner() accepts an optional { formats } object",
                call.span,
            ));
        }
    };
    Ok(serde_json::json!({ "formats": formats }).to_string())
}

fn player_options(
    call: &oxc::ast::ast::CallExpression<'_>,
) -> Result<(&'static str, &'static str), CompileError> {
    match call.arguments.as_slice() {
        [] => Ok(("music", "attached")),
        [Argument::ObjectExpression(options)] => {
            let mut usage = "music";
            let mut playback = "attached";
            let mut seen = HashSet::new();
            for property in &options.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return Err(CompileError::new(
                        "audio player options cannot use spreads",
                        property.span(),
                    ));
                };
                let name = property_name(&property.key)?;
                if !seen.insert(name.clone()) {
                    return Err(CompileError::new(
                        format!("audio player option {name:?} is declared twice"),
                        property.key.span(),
                    ));
                }
                let Expression::StringLiteral(value) = &property.value else {
                    return Err(CompileError::new(
                        "audio player options must be string literals",
                        property.value.span(),
                    ));
                };
                match (name.as_str(), value.value.as_str()) {
                    ("usage", "music") => usage = "music",
                    ("usage", "speech") => usage = "speech",
                    ("playback", "attached") => playback = "attached",
                    ("playback", "detached") => playback = "detached",
                    ("usage", _) => {
                        return Err(CompileError::new(
                            "audio player usage must be \"music\" or \"speech\"",
                            value.span,
                        ));
                    }
                    ("playback", _) => {
                        return Err(CompileError::new(
                            "audio playback must be \"attached\" or \"detached\"",
                            value.span,
                        ));
                    }
                    _ => {
                        return Err(CompileError::new(
                            format!("unknown audio player option {name:?}"),
                            property.key.span(),
                        ));
                    }
                }
            }
            Ok((usage, playback))
        }
        _ => Err(CompileError::new(
            "audioPlayer() accepts an optional { usage, playback } object",
            call.span,
        )),
    }
}

fn pitch_reference(call: &oxc::ast::ast::CallExpression<'_>) -> Result<f64, CompileError> {
    let value = match call.arguments.as_slice() {
        [] => 440.0,
        [Argument::ObjectExpression(options)] => {
            let [ObjectPropertyKind::ObjectProperty(property)] = options.properties.as_slice()
            else {
                return Err(CompileError::new(
                    "pitchDetector() accepts only { referenceHz }",
                    options.span,
                ));
            };
            if property_name(&property.key)? != "referenceHz" {
                return Err(CompileError::new(
                    "pitchDetector() accepts only referenceHz",
                    property.key.span(),
                ));
            }
            let Expression::NumericLiteral(value) = &property.value else {
                return Err(CompileError::new(
                    "referenceHz must be a number literal",
                    property.value.span(),
                ));
            };
            value.value
        }
        _ => {
            return Err(CompileError::new(
                "pitchDetector() accepts an optional { referenceHz } object",
                call.span,
            ));
        }
    };
    if !(400.0..=480.0).contains(&value) {
        return Err(CompileError::new(
            "referenceHz must be between 400 and 480",
            call.span,
        ));
    }
    Ok(value)
}

fn object_shape<const N: usize>(fields: [(&str, StateShape); N]) -> StateShape {
    StateShape::Object(
        fields
            .into_iter()
            .map(|(name, shape)| (name.to_owned(), shape))
            .collect(),
    )
}

fn status_shape(statuses: &[&str]) -> StateShape {
    StateShape::Union(
        statuses
            .iter()
            .map(|status| StateShape::Literal(StateLiteral::String((*status).to_owned())))
            .collect(),
    )
}

fn object_value<const N: usize>(fields: [(&str, StateValue); N]) -> StateValue {
    StateValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn response_shape(
    call: &oxc::ast::ast::CallExpression<'_>,
    imports: &Imports,
    function: &str,
) -> Result<StateShape, CompileError> {
    let Some(arguments) = &call.type_arguments else {
        return Err(CompileError::new(
            format!("{function}<T>() needs the response data type"),
            call.span,
        ));
    };
    let [response_type] = arguments.params.as_slice() else {
        return Err(CompileError::new(
            format!("{function}<T>() accepts one response data type"),
            arguments.span,
        ));
    };
    match response_type {
        TSType::TSTypeReference(reference) if reference.type_arguments.is_none() => {
            let oxc::ast::ast::TSTypeName::IdentifierReference(name) = &reference.type_name else {
                return Err(CompileError::new(
                    "response types use a local type alias or an inline data type",
                    reference.span,
                ));
            };
            imports
                .type_aliases
                .get(name.name.as_str())
                .cloned()
                .ok_or_else(|| {
                    CompileError::new(
                        format!("unknown response type {}", name.name),
                        reference.span,
                    )
                })
        }
        response_type => state_kind_from_type(response_type),
    }
}

fn network_json_resource(
    call: &oxc::ast::ast::CallExpression<'_>,
    imports: &Imports,
    states: &Bindings,
    cached: bool,
) -> Result<ResourceInitialiser, CompileError> {
    let function_name = if cached { "cachedJson" } else { "json" };
    let shape = response_shape(call, imports, function_name)?;
    let Some(Argument::StringLiteral(url)) = call.arguments.first() else {
        return Err(CompileError::new(
            "json<T>() starts with an HTTPS URL string",
            call.span,
        ));
    };
    if !url.value.as_str().starts_with("https://") {
        return Err(CompileError::new(
            "network requests require HTTPS",
            url.span,
        ));
    }
    let mut timeout_ms = 15_000;
    let mut max_age_ms = 300_000;
    let mut stale_if_error_ms = 86_400_000;
    let mut query = Vec::new();
    let mut headers = Vec::new();
    match call.arguments.as_slice() {
        [_] => {}
        [_, Argument::ObjectExpression(options)] => {
            let mut seen = HashSet::new();
            for property in &options.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return Err(CompileError::new(
                        "network options cannot use spreads",
                        property.span(),
                    ));
                };
                let name = property_name(&property.key)?;
                if !seen.insert(name.clone()) {
                    return Err(CompileError::new(
                        format!("network option {name:?} is declared twice"),
                        property.span,
                    ));
                }
                match name.as_str() {
                    "query" => query = network_values(&property.value, false, states)?,
                    "headers" => headers = network_values(&property.value, true, states)?,
                    "timeoutMs" => {
                        timeout_ms = duration_option(&property.value, "timeoutMs", 1_000, 120_000)?;
                    }
                    "maxAgeMs" if cached => {
                        max_age_ms = duration_option(&property.value, "maxAgeMs", 0, 604_800_000)?;
                    }
                    "staleIfErrorMs" if cached => {
                        stale_if_error_ms =
                            duration_option(&property.value, "staleIfErrorMs", 0, 2_592_000_000)?;
                    }
                    _ => {
                        return Err(CompileError::new(
                            format!("unknown network option {name:?}"),
                            property.key.span(),
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(CompileError::new(
                format!("{function_name}<T>() accepts a URL and optional options object"),
                call.span,
            ));
        }
    }
    let mut payload = vec![PayloadPart::Literal(format!(
        "{{\"url\":{},\"query\":{{",
        serde_json::to_string(url.value.as_str()).expect("a source string is valid JSON"),
    ))];
    append_network_values(&mut payload, query);
    payload.push(PayloadPart::Literal(",\"headers\":{".to_owned()));
    append_network_values(&mut payload, headers);
    if cached {
        let schema = shape_schema(&shape);
        payload.push(PayloadPart::Literal(format!(
            ",\"schema\":{schema},\"maxAgeMs\":{max_age_ms},\"staleIfErrorMs\":{stale_if_error_ms}"
        )));
    }
    payload.push(PayloadPart::Literal("}".to_owned()));
    Ok(ResourceInitialiser {
        definition: Resource {
            module: "network".to_owned(),
            operation: if cached { "cached-json" } else { "json" }.to_owned(),
            payload,
            shape,
            timeout_ms,
            reload_on_resume: true,
            protocol: if cached {
                ResourceProtocol::Cached
            } else {
                ResourceProtocol::Async
            },
        },
        request: None,
        android_permission: None,
    })
}

fn network_mutation_resource(
    call: &oxc::ast::ast::CallExpression<'_>,
    imports: &Imports,
    states: &Bindings,
) -> Result<ResourceInitialiser, CompileError> {
    let shape = response_shape(call, imports, "mutation")?;
    let [
        Argument::StringLiteral(url),
        Argument::ObjectExpression(options),
    ] = call.arguments.as_slice()
    else {
        return Err(CompileError::new(
            "mutation<T>() takes an HTTPS URL and options object",
            call.span,
        ));
    };
    if !url.value.as_str().starts_with("https://") {
        return Err(CompileError::new(
            "network requests require HTTPS",
            url.span,
        ));
    }
    let mut method = None;
    let mut timeout_ms = 15_000;
    let mut query = Vec::new();
    let mut headers = Vec::new();
    let mut body = None;
    let mut seen = HashSet::new();
    for property in &options.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "mutation options cannot use spreads",
                property.span(),
            ));
        };
        let name = property_name(&property.key)?;
        if !seen.insert(name.clone()) {
            return Err(CompileError::new(
                format!("mutation option {name:?} is declared twice"),
                property.span,
            ));
        }
        match name.as_str() {
            "method" => {
                let Expression::StringLiteral(value) = &property.value else {
                    return Err(CompileError::new(
                        "mutation method must be POST, PUT, PATCH or DELETE",
                        property.value.span(),
                    ));
                };
                let value = value.value.as_str();
                if !matches!(value, "POST" | "PUT" | "PATCH" | "DELETE") {
                    return Err(CompileError::new(
                        "mutation method must be POST, PUT, PATCH or DELETE",
                        property.value.span(),
                    ));
                }
                method = Some(value.to_owned());
            }
            "query" => query = network_values(&property.value, false, states)?,
            "headers" => headers = network_values(&property.value, true, states)?,
            "body" => body = Some(network_values(&property.value, false, states)?),
            "timeoutMs" => {
                timeout_ms = duration_option(&property.value, "timeoutMs", 1_000, 120_000)?;
            }
            _ => {
                return Err(CompileError::new(
                    format!("unknown mutation option {name:?}"),
                    property.key.span(),
                ));
            }
        }
    }
    let method = method
        .ok_or_else(|| CompileError::new("mutation options require a method", options.span))?;
    let mut payload = vec![PayloadPart::Literal(format!(
        "{{\"url\":{},\"method\":{},\"query\":{{",
        serde_json::to_string(url.value.as_str()).expect("a source string is valid JSON"),
        serde_json::to_string(&method).expect("a network method is valid JSON"),
    ))];
    append_network_values(&mut payload, query);
    payload.push(PayloadPart::Literal(",\"headers\":{".to_owned()));
    append_network_values(&mut payload, headers);
    if let Some(body) = body {
        payload.push(PayloadPart::Literal(",\"body\":{".to_owned()));
        append_network_values(&mut payload, body);
    }
    payload.push(PayloadPart::Literal("}".to_owned()));
    Ok(ResourceInitialiser {
        definition: Resource {
            module: "network".to_owned(),
            operation: "mutation".to_owned(),
            payload,
            shape,
            timeout_ms,
            reload_on_resume: false,
            protocol: ResourceProtocol::Mutation,
        },
        request: None,
        android_permission: None,
    })
}

fn background_json_resource(
    call: &oxc::ast::ast::CallExpression<'_>,
    imports: &Imports,
) -> Result<ResourceInitialiser, CompileError> {
    let shape = response_shape(call, imports, "periodicJson")?;
    let [
        Argument::StringLiteral(key),
        Argument::StringLiteral(url),
        Argument::ObjectExpression(options),
    ] = call.arguments.as_slice()
    else {
        return Err(CompileError::new(
            "periodicJson<T>() takes a key, HTTPS URL and options object",
            call.span,
        ));
    };
    if key.value.is_empty() || key.value.len() > 128 {
        return Err(CompileError::new(
            "background keys must contain between 1 and 128 bytes",
            key.span,
        ));
    }
    if !url.value.as_str().starts_with("https://") {
        return Err(CompileError::new(
            "background requests require HTTPS",
            url.span,
        ));
    }
    let mut every_minutes = None;
    let mut timeout_ms = 15_000_u64;
    let mut query = serde_json::Map::new();
    let mut headers = serde_json::Map::new();
    let mut seen = HashSet::new();
    for property in &options.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "background options cannot use spreads",
                property.span(),
            ));
        };
        let name = property_name(&property.key)?;
        if !seen.insert(name.clone()) {
            return Err(CompileError::new(
                format!("background option {name:?} is declared twice"),
                property.span,
            ));
        }
        match name.as_str() {
            "everyMinutes" => {
                let Expression::NumericLiteral(value) = &property.value else {
                    return Err(CompileError::new(
                        "everyMinutes must be a number literal",
                        property.value.span(),
                    ));
                };
                every_minutes = Some(
                    integer(value.value, value.span, "everyMinutes")?
                        .try_into()
                        .ok()
                        .filter(|value: &u64| (15..=10_080).contains(value))
                        .ok_or_else(|| {
                            CompileError::new(
                                "everyMinutes must be between 15 and 10080",
                                value.span,
                            )
                        })?,
                );
            }
            "timeoutMs" => {
                timeout_ms = duration_option(&property.value, "timeoutMs", 1_000, 120_000)?;
            }
            "query" => query = background_values(&property.value, false)?,
            "headers" => headers = background_values(&property.value, true)?,
            _ => {
                return Err(CompileError::new(
                    format!("unknown background option {name:?}"),
                    property.key.span(),
                ));
            }
        }
    }
    let every_minutes = every_minutes.ok_or_else(|| {
        CompileError::new("background options require everyMinutes", options.span)
    })?;
    let schema = shape_schema(&shape);
    let request = serde_json::json!({
        "url": url.value.as_str(),
        "query": query,
        "headers": headers,
        "schema": schema,
    });
    let request_fingerprint = stable_hash(&request.to_string());
    let payload = serde_json::json!({
        "key": key.value.as_str(),
        "url": url.value.as_str(),
        "query": request["query"].clone(),
        "headers": request["headers"].clone(),
        "schema": request["schema"].clone(),
        "everyMinutes": every_minutes,
        "timeoutMs": timeout_ms,
        "requestFingerprint": format!("{request_fingerprint:016x}"),
    });
    Ok(ResourceInitialiser {
        definition: Resource {
            module: "background".to_owned(),
            operation: "periodic-json".to_owned(),
            payload: vec![PayloadPart::Literal(payload.to_string())],
            shape,
            timeout_ms: 5_000,
            reload_on_resume: false,
            protocol: ResourceProtocol::Background,
        },
        request: None,
        android_permission: None,
    })
}

fn background_values(
    expression: &Expression<'_>,
    strings_only: bool,
) -> Result<serde_json::Map<String, serde_json::Value>, CompileError> {
    let Expression::ObjectExpression(object) = expression else {
        return Err(CompileError::new(
            "background query and headers must be object literals",
            expression.span(),
        ));
    };
    let mut values = serde_json::Map::new();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "background query and headers cannot use spreads",
                property.span(),
            ));
        };
        let name = property_name(&property.key)?;
        let value = match &property.value {
            Expression::StringLiteral(value) => serde_json::Value::String(value.value.to_string()),
            Expression::BooleanLiteral(value) if !strings_only => {
                serde_json::Value::Bool(value.value)
            }
            Expression::NumericLiteral(value) if !strings_only && value.value.is_finite() => {
                serde_json::Number::from_f64(value.value)
                    .map(serde_json::Value::Number)
                    .expect("a finite number has a JSON representation")
            }
            Expression::UnaryExpression(value)
                if !strings_only && value.operator == UnaryOperator::UnaryNegation =>
            {
                let Expression::NumericLiteral(number) = &value.argument else {
                    return Err(CompileError::new(
                        "background query values must be literal scalars",
                        value.span,
                    ));
                };
                serde_json::Number::from_f64(-number.value)
                    .map(serde_json::Value::Number)
                    .expect("a finite number has a JSON representation")
            }
            _ => {
                return Err(CompileError::new(
                    if strings_only {
                        "background header values must be string literals"
                    } else {
                        "background query values must be string, number or boolean literals"
                    },
                    property.value.span(),
                ));
            }
        };
        if values.insert(name.clone(), value).is_some() {
            return Err(CompileError::new(
                format!("background value {name:?} is declared twice"),
                property.span,
            ));
        }
    }
    Ok(values)
}

fn shape_schema(shape: &StateShape) -> serde_json::Value {
    match shape {
        StateShape::Null => serde_json::json!({ "null": true }),
        StateShape::Number => serde_json::Value::String("number".to_owned()),
        StateShape::Bool => serde_json::Value::String("boolean".to_owned()),
        StateShape::String => serde_json::Value::String("string".to_owned()),
        StateShape::Literal(value) => serde_json::json!({
            "literal": match value {
                StateLiteral::Number(value) => serde_json::json!(value),
                StateLiteral::Bool(value) => serde_json::json!(value),
                StateLiteral::String(value) => serde_json::json!(value),
            }
        }),
        StateShape::Optional(shape) => serde_json::json!({ "optional": shape_schema(shape) }),
        StateShape::Union(shapes) => serde_json::json!({
            "oneOf": shapes.iter().map(shape_schema).collect::<Vec<_>>()
        }),
        StateShape::List(item) => serde_json::json!({ "array": shape_schema(item) }),
        StateShape::Object(fields) => serde_json::Value::Object(
            fields
                .iter()
                .map(|(name, shape)| (name.clone(), shape_schema(shape)))
                .collect(),
        ),
    }
}

fn stable_hash(value: &str) -> u64 {
    value.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn network_values(
    expression: &Expression<'_>,
    strings_only: bool,
    states: &Bindings,
) -> Result<Vec<(String, PayloadPart)>, CompileError> {
    let Expression::ObjectExpression(object) = expression else {
        return Err(CompileError::new(
            "query and headers must be object literals",
            expression.span(),
        ));
    };
    let mut values = Vec::new();
    let mut seen = HashSet::new();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "query and headers cannot use spreads",
                property.span(),
            ));
        };
        let name = property_name(&property.key)?;
        if !seen.insert(name.clone()) {
            return Err(CompileError::new(
                format!("network value {name:?} is declared twice"),
                property.span,
            ));
        }
        let part = match &property.value {
            Expression::StringLiteral(value) => PayloadPart::Literal(
                serde_json::to_string(value.value.as_str()).expect("a source string is valid JSON"),
            ),
            Expression::BooleanLiteral(value) if !strings_only => {
                PayloadPart::Literal(value.value.to_string())
            }
            Expression::NumericLiteral(value) if !strings_only && value.value.is_finite() => {
                PayloadPart::Literal(value.value.to_string())
            }
            Expression::UnaryExpression(value)
                if !strings_only && value.operator == UnaryOperator::UnaryNegation =>
            {
                let Expression::NumericLiteral(number) = &value.argument else {
                    return Err(CompileError::new(
                        "query values must be strings, numbers or booleans",
                        value.span,
                    ));
                };
                PayloadPart::Literal((-number.value).to_string())
            }
            expression => {
                let binding = expression_state_value(expression, states)?;
                if strings_only && binding.kind != StateShape::String
                    || !strings_only && !scalar_kind(&binding.kind)
                {
                    return Err(CompileError::new(
                        if strings_only {
                            "header values must be strings"
                        } else {
                            "query values must be strings, numbers or booleans"
                        },
                        expression.span(),
                    ));
                }
                PayloadPart::State(binding.id)
            }
        };
        values.push((name, part));
    }
    Ok(values)
}

fn append_network_values(payload: &mut Vec<PayloadPart>, values: Vec<(String, PayloadPart)>) {
    for (index, (name, value)) in values.into_iter().enumerate() {
        let comma = if index == 0 { "" } else { "," };
        payload.push(PayloadPart::Literal(format!(
            "{comma}{}:",
            serde_json::to_string(&name).expect("a source string is valid JSON"),
        )));
        payload.push(value);
    }
    payload.push(PayloadPart::Literal("}".to_owned()));
}

fn duration_option(
    expression: &Expression<'_>,
    name: &str,
    minimum: u64,
    maximum: u64,
) -> Result<u64, CompileError> {
    let Expression::NumericLiteral(value) = expression else {
        return Err(CompileError::new(
            format!("{name} must be a number literal"),
            expression.span(),
        ));
    };
    integer(value.value, value.span, name)?
        .try_into()
        .ok()
        .filter(|value| (minimum..=maximum).contains(value))
        .ok_or_else(|| {
            CompileError::new(
                format!("{name} must be between {minimum} and {maximum}"),
                value.span,
            )
        })
}

fn state_initialiser(
    initialiser: Option<&Expression<'_>>,
    span: Span,
) -> Result<(StateValue, StateShape, StateLifetime, &'static str), CompileError> {
    let Some(Expression::CallExpression(call)) = initialiser else {
        return Err(CompileError::new(
            "state must be initialised with state(value), sharedState(key, value) or persistedState(key, value)",
            span,
        ));
    };
    let Expression::Identifier(callee) = &call.callee else {
        return Err(CompileError::new(
            "expected an Ink state constructor",
            call.callee.span(),
        ));
    };
    let constructor = callee.name.as_str();
    let (value_index, lifetime, constructor) = match constructor {
        "state" if call.arguments.len() == 1 => (0, StateLifetime::Local, "state"),
        "sharedState" | "persistedState" if call.arguments.len() == 2 => {
            let Some(Expression::StringLiteral(key)) = call.arguments[0].as_expression() else {
                return Err(CompileError::new(
                    format!("{constructor} key must be a string literal"),
                    call.arguments[0].span(),
                ));
            };
            let key_span = key.span;
            let key = key.value.as_str();
            if key.is_empty() || key.len() > 128 || key.chars().any(char::is_control) {
                return Err(CompileError::new(
                    "state keys must be 1–128 bytes and contain no control characters",
                    key_span,
                ));
            }
            let lifetime = if constructor == "sharedState" {
                StateLifetime::Shared(key.to_owned())
            } else {
                StateLifetime::Persisted(key.to_owned())
            };
            (
                1,
                lifetime,
                if constructor == "sharedState" {
                    "sharedState"
                } else {
                    "persistedState"
                },
            )
        }
        "state" => return Err(CompileError::new("expected state(value)", call.span)),
        "sharedState" => {
            return Err(CompileError::new(
                "expected sharedState(key, value)",
                call.span,
            ));
        }
        "persistedState" => {
            return Err(CompileError::new(
                "expected persistedState(key, value)",
                call.span,
            ));
        }
        _ => {
            return Err(CompileError::new(
                "expected an Ink state constructor",
                call.span,
            ));
        }
    };
    let Some(value) = call.arguments[value_index].as_expression() else {
        return Err(CompileError::new(
            "state values cannot use spread syntax",
            call.arguments[value_index].span(),
        ));
    };
    let value = literal_state_value(value)?;
    let declared = match &call.type_arguments {
        Some(arguments) => {
            let [kind] = arguments.params.as_slice() else {
                return Err(CompileError::new(
                    "state accepts one type argument",
                    arguments.span,
                ));
            };
            Some(state_kind_from_type(kind)?)
        }
        None => None,
    };
    let inferred = state_kind(&value);
    let kind = match (declared, inferred) {
        (Some(declared), _) if !shape_accepts_value(&declared, &value) => {
            return Err(CompileError::new(
                "the state type does not match its initial value",
                call.span,
            ));
        }
        (Some(declared), _) => declared,
        (None, Some(inferred)) => inferred,
        (None, None) => {
            return Err(CompileError::new(
                "empty list state needs an explicit array type",
                call.span,
            )
            .with_help(match &lifetime {
                StateLifetime::Local => "state<{ name: string }[]>([])",
                StateLifetime::Shared(_) => "sharedState<{ name: string }[]>(\"items\", [])",
                StateLifetime::Persisted(_) => "persistedState<{ name: string }[]>(\"items\", [])",
            }));
        }
    };
    if matches!(&kind, StateShape::Object(_)) {
        return Err(CompileError::new(
            "state supports scalar values and lists",
            call.span,
        ));
    }
    Ok((value, kind, lifetime, constructor))
}

fn literal_state_value(value: &Expression<'_>) -> Result<StateValue, CompileError> {
    match value {
        Expression::NullLiteral(_) => Ok(StateValue::Null),
        Expression::NumericLiteral(value) if value.value.is_finite() => {
            Ok(StateValue::Number(value.value))
        }
        Expression::BooleanLiteral(value) => Ok(StateValue::Bool(value.value)),
        Expression::StringLiteral(value) => Ok(StateValue::String(value.value.to_string())),
        Expression::ArrayExpression(array) => {
            let mut values = Vec::with_capacity(array.elements.len());
            let mut item_kind = None;
            for element in &array.elements {
                let Some(expression) = element.as_expression() else {
                    return Err(CompileError::new(
                        "state lists cannot contain holes or spreads",
                        element.span(),
                    ));
                };
                let value = literal_state_value(expression)?;
                let Some(kind) = state_kind(&value) else {
                    return Err(CompileError::new(
                        "nested empty lists need an explicit shape",
                        expression.span(),
                    ));
                };
                if item_kind.as_ref().is_some_and(|expected| expected != &kind) {
                    return Err(CompileError::new(
                        "every item in a state list must have the same shape",
                        expression.span(),
                    ));
                }
                item_kind.get_or_insert(kind);
                values.push(value);
            }
            Ok(StateValue::List(values))
        }
        Expression::ObjectExpression(object) => {
            let mut values = Vec::with_capacity(object.properties.len());
            let mut names = HashSet::new();
            for property in &object.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return Err(CompileError::new(
                        "state objects cannot use spread properties",
                        property.span(),
                    ));
                };
                if property.kind != PropertyKind::Init
                    || property.method
                    || property.shorthand
                    || property.computed
                {
                    return Err(CompileError::new(
                        "state objects use explicit literal properties",
                        property.span,
                    ));
                }
                let name = property_name(&property.key)?;
                if !names.insert(name.clone()) {
                    return Err(CompileError::new(
                        format!("state object property {name:?} is declared twice"),
                        property.key.span(),
                    ));
                }
                values.push((name, literal_state_value(&property.value)?));
            }
            Ok(StateValue::Object(values))
        }
        _ => Err(CompileError::new(
            "state values must be literal nulls, numbers, booleans, strings, objects or lists",
            value.span(),
        )),
    }
}

fn state_kind(value: &StateValue) -> Option<StateShape> {
    match value {
        StateValue::Null => Some(StateShape::Null),
        StateValue::Number(_) => Some(StateShape::Number),
        StateValue::Bool(_) => Some(StateShape::Bool),
        StateValue::String(_) => Some(StateShape::String),
        StateValue::List(values) => values
            .first()
            .and_then(state_kind)
            .map(|kind| StateShape::List(Box::new(kind))),
        StateValue::Object(values) => {
            let fields = values
                .iter()
                .map(|(name, value)| Some((name.clone(), state_kind(value)?)))
                .collect::<Option<_>>()?;
            Some(StateShape::Object(fields))
        }
    }
}

fn state_kind_from_type(kind: &TSType<'_>) -> Result<StateShape, CompileError> {
    match kind {
        TSType::TSNullKeyword(_) => Ok(StateShape::Null),
        TSType::TSNumberKeyword(_) => Ok(StateShape::Number),
        TSType::TSBooleanKeyword(_) => Ok(StateShape::Bool),
        TSType::TSStringKeyword(_) => Ok(StateShape::String),
        TSType::TSLiteralType(literal) => match &literal.literal {
            TSLiteral::NumericLiteral(value) if value.value.is_finite() => {
                Ok(StateShape::Literal(StateLiteral::Number(value.value)))
            }
            TSLiteral::BooleanLiteral(value) => {
                Ok(StateShape::Literal(StateLiteral::Bool(value.value)))
            }
            TSLiteral::StringLiteral(value) => Ok(StateShape::Literal(StateLiteral::String(
                value.value.to_string(),
            ))),
            TSLiteral::UnaryExpression(value) if value.operator == UnaryOperator::UnaryNegation => {
                let Expression::NumericLiteral(number) = &value.argument else {
                    return Err(CompileError::new(
                        "data types use finite literal numbers",
                        value.span,
                    ));
                };
                Ok(StateShape::Literal(StateLiteral::Number(-number.value)))
            }
            _ => Err(CompileError::new(
                "data types use finite number, boolean and string literals",
                literal.span,
            )),
        },
        TSType::TSUnionType(union) => {
            let mut shapes = Vec::new();
            for kind in &union.types {
                let shape = state_kind_from_type(kind)?;
                let variants = match shape {
                    StateShape::Union(nested) => nested,
                    shape => vec![shape],
                };
                for shape in variants {
                    if !shapes.contains(&shape) {
                        shapes.push(shape);
                    }
                }
            }
            Ok(StateShape::Union(shapes))
        }
        TSType::TSArrayType(array) => Ok(StateShape::List(Box::new(state_kind_from_type(
            &array.element_type,
        )?))),
        TSType::TSParenthesizedType(parenthesised) => {
            state_kind_from_type(&parenthesised.type_annotation)
        }
        TSType::TSTypeLiteral(object) => {
            let mut fields = BTreeMap::new();
            for member in &object.members {
                let TSSignature::TSPropertySignature(property) = member else {
                    return Err(CompileError::new(
                        "state object types use required properties",
                        member.span(),
                    ));
                };
                if property.computed {
                    return Err(CompileError::new(
                        "state object properties must use named keys",
                        property.span,
                    ));
                }
                let name = property_name(&property.key)?;
                let annotation = property.type_annotation.as_ref().ok_or_else(|| {
                    CompileError::new("state properties need a type", property.span)
                })?;
                let mut shape = state_kind_from_type(&annotation.type_annotation)?;
                if property.optional {
                    shape = StateShape::Optional(Box::new(shape));
                }
                if fields.insert(name.clone(), shape).is_some() {
                    return Err(CompileError::new(
                        format!("state object property {name:?} is declared twice"),
                        property.key.span(),
                    ));
                }
            }
            Ok(StateShape::Object(fields))
        }
        _ => Err(CompileError::new(
            "data types use null, finite numbers, booleans, strings, literals, arrays and object literals",
            kind.span(),
        )),
    }
}

fn shape_accepts_value(shape: &StateShape, value: &StateValue) -> bool {
    match (shape, value) {
        (StateShape::Null, StateValue::Null) => true,
        (StateShape::Number, StateValue::Number(value)) => value.is_finite(),
        (StateShape::Bool, StateValue::Bool(_)) | (StateShape::String, StateValue::String(_)) => {
            true
        }
        (StateShape::Literal(StateLiteral::Number(expected)), StateValue::Number(value)) => {
            expected == value
        }
        (StateShape::Literal(StateLiteral::Bool(expected)), StateValue::Bool(value)) => {
            expected == value
        }
        (StateShape::Literal(StateLiteral::String(expected)), StateValue::String(value)) => {
            expected == value
        }
        (StateShape::Optional(_), StateValue::Null) => true,
        (StateShape::Optional(shape), value) => shape_accepts_value(shape, value),
        (StateShape::Union(shapes), value) => {
            shapes.iter().any(|shape| shape_accepts_value(shape, value))
        }
        (StateShape::List(shape), StateValue::List(values)) => {
            values.iter().all(|value| shape_accepts_value(shape, value))
        }
        (StateShape::Object(shapes), StateValue::Object(values)) => {
            values.iter().all(|(name, _)| shapes.contains_key(name))
                && shapes.iter().all(|(name, shape)| {
                    match values.iter().find(|(value_name, _)| value_name == name) {
                        Some((_, value)) => shape_accepts_value(shape, value),
                        None => matches!(shape, StateShape::Optional(_)),
                    }
                })
        }
        _ => false,
    }
}

fn shape_accepts_shape(expected: &StateShape, actual: &StateShape) -> bool {
    if expected == actual {
        return true;
    }
    if let StateShape::Union(actual) = actual {
        return actual
            .iter()
            .all(|actual| shape_accepts_shape(expected, actual));
    }
    if let StateShape::Optional(actual) = actual {
        return shape_accepts_shape(expected, &StateShape::Null)
            && shape_accepts_shape(expected, actual);
    }
    match (expected, actual) {
        (StateShape::Number, StateShape::Number | StateShape::Literal(StateLiteral::Number(_)))
        | (StateShape::Bool, StateShape::Bool | StateShape::Literal(StateLiteral::Bool(_)))
        | (StateShape::String, StateShape::String | StateShape::Literal(StateLiteral::String(_))) => {
            true
        }
        (StateShape::Optional(_), StateShape::Null) => true,
        (StateShape::Optional(expected), actual) => shape_accepts_shape(expected, actual),
        (StateShape::Union(expected), actual) => expected
            .iter()
            .any(|expected| shape_accepts_shape(expected, actual)),
        _ => false,
    }
}

fn property_name(key: &PropertyKey<'_>) -> Result<String, CompileError> {
    match key {
        PropertyKey::StaticIdentifier(name) => Ok(name.name.to_string()),
        PropertyKey::StringLiteral(name) => Ok(name.value.to_string()),
        _ => Err(CompileError::new(
            "state object keys must be names or strings",
            key.span(),
        )),
    }
}

fn lower_node(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let name = element_name(&element.opening_element.name)?;
    if let Some(path) = imports.screens.get(name) {
        reject_other_attributes(element, &[])?;
        if !element_children(element)?.is_empty() {
            return Err(CompileError::new(
                "screen modules cannot have children",
                element.span,
            ));
        }
        return Ok(Node::ScreenModule { path: path.clone() });
    }
    if imports.extension_elements.get(name) == Some(&ExtensionElement::CameraPreview) {
        return lower_camera_preview(element, states);
    }
    require_import(imports, name, element.opening_element.name.span())?;
    match name {
        "Screen" => lower_screen(element, states, imports, item),
        "Stack" => lower_stack(element, states, imports, item),
        "Text" => lower_text(element, states, item),
        "TextInput" => lower_text_input(element, states),
        "Button" => lower_button(element, states, imports, item),
        "Field" => lower_field(element, states, imports, item),
        "Icon" => lower_icon(element),
        "Image" => lower_image(element, states, item),
        "Toggle" => lower_toggle(element, states),
        "Tabs" => lower_tabs(element, states, imports),
        "Navigator" => Err(CompileError::new(
            "<Navigator> may only be the app root",
            element.span,
        )),
        "Route" => Err(CompileError::new(
            "<Route> may only appear directly inside <Navigator>",
            element.span,
        )),
        "Tab" => Err(CompileError::new(
            "<Tab> may only appear directly inside <Tabs>",
            element.span,
        )),
        _ => Err(CompileError::new(
            format!("{name} is not an Ink element"),
            element.opening_element.name.span(),
        )),
    }
}

fn lower_content_node(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let node = lower_node(element, states, imports, item)?;
    if matches!(node, Node::ScreenModule { .. }) {
        return Err(CompileError::new(
            "screen modules may only appear inside Route or Tab",
            element.span,
        ));
    }
    Ok(node)
}

fn lower_screen(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let title = optional_string_attribute(element, "title")?;
    let centered = boolean_attribute(element, "centered")?;
    reject_other_attributes(element, &["title", "centered"])?;
    Ok(Node::Screen {
        children: lower_element_children(element, states, imports, item, "Screen")?,
        title,
        centered,
        params: states
            .route_params
            .values()
            .next()
            .map(|params| params.fields.clone())
            .unwrap_or_default(),
        resources: Vec::new(),
        controllers: Vec::new(),
    })
}

fn lower_stack(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let axis = match optional_string_attribute(element, "axis")?.as_deref() {
        None | Some("vertical") => Axis::Vertical,
        Some("horizontal") => Axis::Horizontal,
        Some(_) => return invalid_value(element, "axis", "vertical or horizontal"),
    };
    let align = match optional_string_attribute(element, "align")?.as_deref() {
        None | Some("stretch") => Alignment::Stretch,
        Some("start") => Alignment::Start,
        Some("center") => Alignment::Center,
        Some("end") => Alignment::End,
        Some(_) => return invalid_value(element, "align", "start, center, end or stretch"),
    };
    let justify = match optional_string_attribute(element, "justify")?.as_deref() {
        None | Some("start") => Justification::Start,
        Some("center") => Justification::Center,
        Some("end") => Justification::End,
        Some("space-between") => Justification::SpaceBetween,
        Some(_) => {
            return invalid_value(element, "justify", "start, center, end or space-between");
        }
    };
    let gap = optional_number_attribute(element, "gap")?;
    reject_other_attributes(element, &["axis", "gap", "align", "justify"])?;
    Ok(Node::Stack {
        children: lower_element_children(element, states, imports, item, "Stack")?,
        axis,
        gap,
        align,
        justify,
    })
}

fn lower_text(
    element: &JSXElement<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let font_size = optional_number_attribute(element, "size")?;
    let align = text_alignment(element)?;
    let max_lines = optional_positive_integer_attribute(element, "maxLines")?;
    reject_other_attributes(element, &["size", "align", "maxLines"])?;
    Ok(Node::Text {
        parts: lower_text_parts(element, states, item, "Text")?,
        font_size,
        align,
        max_lines,
    })
}

fn lower_text_input(element: &JSXElement<'_>, states: &Bindings) -> Result<Node, CompileError> {
    let placeholder = required_string_attribute(element, "placeholder")?;
    let action = match optional_string_attribute(element, "action")?.as_deref() {
        None | Some("search") => crate::ir::TextInputAction::Search,
        Some("return") => crate::ir::TextInputAction::Return,
        Some("done") => crate::ir::TextInputAction::Done,
        Some(_) => return invalid_value(element, "action", "search, return or done"),
    };
    let state = state_attribute(element, "value", states, StateShape::String)?;
    text_change_attribute(element, "onChange", &state, states)?;
    reject_other_attributes(element, &["placeholder", "value", "onChange", "action"])?;
    if !element_children(element)?.is_empty() {
        return Err(CompileError::new(
            "TextInput cannot have children",
            element.span,
        ));
    }
    Ok(Node::TextInput {
        placeholder,
        state: state.id,
        action,
    })
}

fn lower_button(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let action = press_action(element, "Button", states, imports, item)?;
    let icon = optional_icon_attribute(element, "icon")?;
    let underline = button_underline(element, states)?;
    reject_other_attributes(element, &["onPress", "href", "icon", "underline"])?;
    let label = lower_text_parts(element, states, item, "Button")?;
    let button = |label, action, underline| Node::Button {
        label,
        icon: icon.clone(),
        underline,
        action,
    };
    Ok(match underline {
        BooleanValue::Literal(underline) => button(label, action, underline),
        BooleanValue::Dynamic(condition) => Node::Conditional {
            condition,
            consequent: Box::new(button(label.clone(), action.clone(), true)),
            alternate: Some(Box::new(button(label, action, false))),
        },
    })
}

fn lower_field(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let label = required_string_attribute(element, "label")?;
    let action = press_action(element, "Field", states, imports, item)?;
    reject_other_attributes(element, &["label", "onPress", "href"])?;
    Ok(Node::Field {
        label,
        value: lower_text_parts(element, states, item, "Field")?,
        action,
    })
}

fn press_action(
    element: &JSXElement<'_>,
    component: &str,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Option<Action>, CompileError> {
    let href = attribute(element, "href");
    if href.is_some() && attribute(element, "onPress").is_some() {
        return Err(CompileError::new(
            format!("{component} accepts either href or onPress, not both"),
            element.opening_element.span,
        ));
    }
    match href {
        Some(attribute) => {
            let span = attribute.span;
            let (path, params) = route_target(attribute, states, item)?;
            validate_route_path(&path, span)?;
            Ok(Some(Action::Navigate {
                path,
                params,
                source: SourceSpan {
                    path: imports.source_path.clone(),
                    span,
                },
            }))
        }
        None => optional_button_action_attribute(element, states, imports, item),
    }
}

fn route_target(
    attribute: &JSXAttribute<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<(String, Vec<RouteArgument>), CompileError> {
    match &attribute.value {
        Some(JSXAttributeValue::StringLiteral(path)) => Ok((path.value.to_string(), Vec::new())),
        Some(JSXAttributeValue::ExpressionContainer(container)) => {
            let Some(Expression::ObjectExpression(target)) = container.expression.as_expression()
            else {
                return Err(CompileError::new(
                    "href must be a path string or { path, params }",
                    container.span,
                ));
            };
            let mut path = None;
            let mut params = None;
            for property in &target.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return Err(CompileError::new(
                        "href objects cannot use spreads",
                        property.span(),
                    ));
                };
                let name = property_name(&property.key)?;
                match name.as_str() {
                    "path" if path.is_none() => {
                        let Expression::StringLiteral(value) = &property.value else {
                            return Err(CompileError::new(
                                "href path must be a string literal",
                                property.value.span(),
                            ));
                        };
                        path = Some(value.value.to_string());
                    }
                    "params" if params.is_none() => {
                        let Expression::ObjectExpression(values) = &property.value else {
                            return Err(CompileError::new(
                                "href params must be an object",
                                property.value.span(),
                            ));
                        };
                        params = Some(route_arguments(values, states, item)?);
                    }
                    "path" | "params" => {
                        return Err(CompileError::new(
                            format!("href field {name:?} is declared twice"),
                            property.span,
                        ));
                    }
                    _ => {
                        return Err(CompileError::new(
                            format!("unknown href field {name:?}"),
                            property.key.span(),
                        ));
                    }
                }
            }
            let path =
                path.ok_or_else(|| CompileError::new("href objects require path", target.span))?;
            Ok((path, params.unwrap_or_default()))
        }
        _ => Err(CompileError::new(
            "href must be a path string or { path, params }",
            attribute.span,
        )),
    }
}

fn route_arguments(
    object: &oxc::ast::ast::ObjectExpression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Vec<RouteArgument>, CompileError> {
    let mut arguments = Vec::new();
    let mut names = HashSet::new();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "route params cannot use spreads",
                property.span(),
            ));
        };
        let name = property_name(&property.key)?;
        if !names.insert(name.clone()) {
            return Err(CompileError::new(
                format!("route param {name:?} is declared twice"),
                property.span,
            ));
        }
        let (value, shape) = route_value(&property.value, states, item)?;
        arguments.push(RouteArgument { name, value, shape });
    }
    Ok(arguments)
}

fn route_value(
    expression: &Expression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<(Value, StateShape), CompileError> {
    let expression = unparenthesised(expression);
    let literal = match expression {
        Expression::NullLiteral(_) => Some((Value::Null, StateShape::Null)),
        Expression::NumericLiteral(value) if value.value.is_finite() => {
            Some((Value::Number(value.value), StateShape::Number))
        }
        Expression::BooleanLiteral(value) => Some((Value::Bool(value.value), StateShape::Bool)),
        Expression::StringLiteral(value) => {
            Some((Value::String(value.value.to_string()), StateShape::String))
        }
        _ => None,
    };
    if let Some(literal) = literal {
        return Ok(literal);
    }
    if let Some(value) = expression_derived_value(expression, states)? {
        return Ok(value);
    }
    if let Some(item) = item
        && let Some(path) = item_path(expression, item.name)
    {
        let shape = kind_at_path(item.kind, &path)
            .ok_or_else(|| CompileError::new("unknown list-item field", expression.span()))?;
        if scalar_kind(shape) {
            return Ok((Value::Item(path), shape.clone()));
        }
    }
    if let Some(resource) = expression_resource_value(expression, states)?
        && scalar_kind(&resource.kind)
    {
        return Ok((
            Value::Resource(resource.resource, resource.field),
            resource.kind,
        ));
    }
    let state = expression_state_value(expression, states)?;
    if !scalar_kind(&state.kind) {
        return Err(CompileError::new(
            "route params must be scalar values",
            expression.span(),
        ));
    }
    Ok((Value::State(state.id), state.kind))
}

fn lower_icon(element: &JSXElement<'_>) -> Result<Node, CompileError> {
    let name = required_icon_attribute(element, "name")?;
    let size = optional_number_attribute(element, "size")?;
    let tone = tone(element)?;
    reject_other_attributes(element, &["name", "size", "tone"])?;
    if !element_children(element)?.is_empty() {
        return Err(CompileError::new("Icon cannot have children", element.span));
    }
    Ok(Node::Icon { name, size, tone })
}

fn lower_image(
    element: &JSXElement<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let source_attribute = attribute(element, "src")
        .ok_or_else(|| CompileError::new("Image requires src", element.opening_element.span))?;
    let source = match &source_attribute.value {
        Some(JSXAttributeValue::StringLiteral(value)) => {
            let span = value.span;
            let value = value.value.as_str().to_owned();
            if value.starts_with("https://") {
                ImageSource::Remote(vec![TextPart::Literal(value)])
            } else if value.contains("://") {
                return Err(CompileError::new("remote images require HTTPS", span));
            } else {
                ImageSource::Local(value)
            }
        }
        Some(JSXAttributeValue::ExpressionContainer(container)) => {
            let Some(expression) = container.expression.as_expression() else {
                return Err(CompileError::new(
                    "Image src cannot be empty",
                    container.span,
                ));
            };
            if let Some(binding) = expression_controller_value(expression, states)? {
                if binding.controller_kind != NativeControllerKind::Photo
                    || binding.path != ["value", "source"]
                {
                    return Err(CompileError::new(
                        "Image accepts a camera source only from photoCapture().value.source",
                        expression.span(),
                    ));
                }
                ImageSource::Camera(vec![TextPart::Controller(binding.controller, binding.path)])
            } else {
                ImageSource::Remote(vec![image_source_part(expression, states, item)?])
            }
        }
        _ => {
            return Err(CompileError::new(
                "Image src must be a local path, HTTPS URL or string value",
                source_attribute.span,
            ));
        }
    };
    let fallback = optional_string_attribute(element, "fallback")?;
    if fallback
        .as_deref()
        .is_some_and(|value| value.contains("://"))
    {
        return Err(CompileError::new(
            "Image fallback must be a local PNG",
            attribute(element, "fallback")
                .expect("fallback exists")
                .span,
        ));
    }
    let bleed = boolean_attribute(element, "bleed")?;
    let width = required_number_attribute(element, "width")?;
    let height = required_number_attribute(element, "height")?;
    if width == 0.0 || height == 0.0 {
        return Err(CompileError::new(
            "Image width and height must be greater than zero",
            element.opening_element.span,
        ));
    }
    let fit = match optional_string_attribute(element, "fit")?.as_deref() {
        None | Some("cover") => ImageFit::Cover,
        Some("contain") => ImageFit::Contain,
        Some(_) => return invalid_value(element, "fit", "cover or contain"),
    };
    reject_other_attributes(
        element,
        &["src", "fallback", "bleed", "width", "height", "fit"],
    )?;
    if !element_children(element)?.is_empty() {
        return Err(CompileError::new(
            "Image cannot have children",
            element.span,
        ));
    }
    Ok(Node::Image {
        source,
        fallback,
        bleed,
        width,
        height,
        fit,
    })
}

fn lower_camera_preview(element: &JSXElement<'_>, states: &Bindings) -> Result<Node, CompileError> {
    let session = attribute(element, "session").ok_or_else(|| {
        CompileError::new(
            "CameraPreview requires session",
            element.opening_element.span,
        )
    })?;
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &session.value else {
        return Err(CompileError::new(
            "CameraPreview session must be a photoCapture() or codeScanner() value",
            session.span,
        ));
    };
    let Some(expression) = container.expression.as_expression() else {
        return Err(CompileError::new(
            "CameraPreview session cannot be empty",
            container.span,
        ));
    };
    let Expression::Identifier(identifier) = unparenthesised(expression) else {
        return Err(CompileError::new(
            "CameraPreview session must name a camera session",
            expression.span(),
        ));
    };
    let controller = states
        .controllers
        .get(identifier.name.as_str())
        .ok_or_else(|| {
            CompileError::new(
                "CameraPreview session must come from photoCapture() or codeScanner()",
                identifier.span,
            )
        })?;
    let kind = match controller.kind {
        NativeControllerKind::Photo => CameraPreviewKind::Photo,
        NativeControllerKind::Scanner => CameraPreviewKind::Scanner,
        _ => {
            return Err(CompileError::new(
                "CameraPreview session must come from photoCapture() or codeScanner()",
                identifier.span,
            ));
        }
    };
    reject_other_attributes(element, &["session"])?;
    if !element_children(element)?.is_empty() {
        return Err(CompileError::new(
            "CameraPreview cannot have children",
            element.span,
        ));
    }
    Ok(Node::CameraPreview {
        controller: controller.id,
        kind,
    })
}

fn image_source_part(
    expression: &Expression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<TextPart, CompileError> {
    if let Some(binding) = expression_resource_value(expression, states)? {
        if binding.kind != StateShape::String {
            return Err(CompileError::new(
                "Image src must be a string",
                expression.span(),
            ));
        }
        return Ok(TextPart::Resource(binding.resource, binding.field));
    }
    if let Some(item) = item
        && let Some(path) = item_path(expression, item.name)
    {
        if kind_at_path(item.kind, &path) != Some(&StateShape::String) {
            return Err(CompileError::new(
                "Image src must be a string",
                expression.span(),
            ));
        }
        return Ok(TextPart::Item(path));
    }
    let binding = expression_state_value(expression, states)?;
    if binding.kind != StateShape::String {
        return Err(CompileError::new(
            "Image src must be a string",
            expression.span(),
        ));
    }
    Ok(TextPart::State(binding.id))
}

fn lower_toggle(element: &JSXElement<'_>, states: &Bindings) -> Result<Node, CompileError> {
    let label = required_string_attribute(element, "label")?;
    let state = state_attribute(element, "value", states, StateShape::Bool)?;
    let action = action_attribute(element, "onChange", states, None)?;
    reject_other_attributes(element, &["label", "value", "onChange"])?;
    if !element_children(element)?.is_empty() {
        return Err(CompileError::new(
            "Toggle cannot have children",
            element.span,
        ));
    }
    Ok(Node::Toggle {
        label,
        state: state.id,
        action,
    })
}

fn lower_tabs(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
) -> Result<Node, CompileError> {
    reject_other_attributes(element, &[])?;
    require_import(imports, "Tab", element.span)?;
    let mut tabs = Vec::new();
    for child in element_children(element)? {
        let JSXChild::Element(child) = child else {
            return Err(CompileError::new(
                "Tabs children must be <Tab> elements",
                child.span(),
            ));
        };
        tabs.push(lower_tab(child, states, imports)?);
    }
    if tabs.is_empty() {
        return Err(CompileError::new(
            "Tabs needs at least one Tab",
            element.span,
        ));
    }
    Ok(Node::Tabs {
        state: UNASSIGNED_TAB_STATE,
        tabs,
    })
}

const UNASSIGNED_TAB_STATE: StateId = StateId(usize::MAX);

fn assign_tab_states(node: &mut Node, states: &mut Vec<State>, source_path: &Path, span: Span) {
    match node {
        Node::Tabs { state, tabs } => {
            let id = StateId(states.len());
            states.push(State {
                initial: StateValue::Number(0.0),
                shape: StateShape::Number,
                lifetime: StateLifetime::Local,
                source: SourceSpan {
                    path: source_path.to_owned(),
                    span,
                },
            });
            *state = id;
            for (index, tab) in tabs.iter_mut().enumerate() {
                tab.action = Action::SetValue {
                    state: id,
                    value: Value::Number(index as f64),
                };
                assign_tab_states(&mut tab.screen, states, source_path, span);
            }
        }
        Node::Screen { children, .. } | Node::Stack { children, .. } => {
            for child in children {
                assign_tab_states(child, states, source_path, span);
            }
        }
        Node::Navigator { routes } => {
            for route in routes {
                assign_tab_states(&mut route.screen, states, source_path, span);
            }
        }
        Node::Conditional {
            consequent,
            alternate,
            ..
        } => {
            assign_tab_states(consequent, states, source_path, span);
            if let Some(alternate) = alternate {
                assign_tab_states(alternate, states, source_path, span);
            }
        }
        Node::ForEach { template, .. } => {
            assign_tab_states(template, states, source_path, span);
        }
        _ => {}
    }
}

fn lower_navigator(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
) -> Result<Node, CompileError> {
    require_import(imports, "Navigator", element.span)?;
    reject_other_attributes(element, &[])?;
    require_import(imports, "Route", element.span)?;
    let mut paths = HashSet::new();
    let mut routes = Vec::new();

    for child in element_children(element)? {
        let JSXChild::Element(child) = child else {
            return Err(CompileError::new(
                "Navigator children must be <Route> elements",
                child.span(),
            ));
        };
        expect_element(child, "Route")?;
        let path = required_string_attribute(child, "path")?;
        let span =
            attribute(child, "path").map_or(child.opening_element.span, |attribute| attribute.span);
        validate_route_path(&path, span)?;
        if !paths.insert(path.clone()) {
            return Err(CompileError::new(
                format!("route {path:?} is declared twice"),
                span,
            ));
        }
        reject_other_attributes(child, &["path"])?;

        let children = element_children(child)?;
        if children.len() != 1 {
            return Err(CompileError::new(
                "Route must contain exactly one Screen or Tabs",
                child.span,
            ));
        }
        let JSXChild::Element(screen) = children[0] else {
            return Err(CompileError::new(
                "Route must contain exactly one Screen or Tabs",
                children[0].span(),
            ));
        };
        let screen = lower_node(screen, states, imports, None)?;
        if !matches!(
            screen,
            Node::Screen { .. } | Node::Tabs { .. } | Node::ScreenModule { .. }
        ) {
            return Err(CompileError::new(
                "Route must contain exactly one Screen or Tabs",
                children[0].span(),
            ));
        }
        routes.push(Route {
            path,
            screen: Box::new(screen),
        });
    }

    if !paths.contains("/") {
        return Err(CompileError::new(
            "Navigator requires a root <Route path=\"/\">",
            element.span,
        ));
    }
    Ok(Node::Navigator { routes })
}

fn lower_tab(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
) -> Result<Tab, CompileError> {
    expect_element(element, "Tab")?;
    let icon = required_icon_attribute(element, "icon")?;
    reject_other_attributes(element, &["icon"])?;
    let children = element_children(element)?;
    if children.len() != 1 {
        return Err(CompileError::new(
            "Tab must contain exactly one Screen",
            element.span,
        ));
    }
    let JSXChild::Element(screen) = children[0] else {
        return Err(CompileError::new(
            "Tab must contain exactly one Screen",
            children[0].span(),
        ));
    };
    let screen = lower_node(screen, states, imports, None)?;
    if !matches!(screen, Node::Screen { .. } | Node::ScreenModule { .. }) {
        return Err(CompileError::new(
            "Tab must contain exactly one Screen",
            children[0].span(),
        ));
    }
    Ok(Tab {
        icon,
        action: Action::Sequence(Vec::new()),
        screen: Box::new(screen),
    })
}

pub struct NavigationError {
    source: PathBuf,
    error: CompileError,
}

impl NavigationError {
    pub fn render(&self) -> String {
        match std::fs::read_to_string(&self.source) {
            Ok(source) => self.error.render(&self.source, &source),
            Err(_) => format!("{}: {}", self.source.display(), self.error),
        }
    }
}

pub fn validate_navigation(root: &Node) -> Result<(), NavigationError> {
    let routes = match root {
        Node::Navigator { routes } => Some(
            routes
                .iter()
                .map(|route| {
                    let params = match route.screen.as_ref() {
                        Node::Screen { params, .. } => params.clone(),
                        Node::Tabs { .. } => BTreeMap::new(),
                        _ => unreachable!("routes contain screens or tabs after expansion"),
                    };
                    (route.path.as_str(), params)
                })
                .collect::<HashMap<_, _>>(),
        ),
        _ => None,
    };
    validate_navigation_node(root, routes.as_ref())
}

fn validate_navigation_node(
    node: &Node,
    routes: Option<&HashMap<&str, BTreeMap<String, StateShape>>>,
) -> Result<(), NavigationError> {
    match node {
        Node::Screen { children, .. } | Node::Stack { children, .. } => {
            for child in children {
                validate_navigation_node(child, routes)?;
            }
        }
        Node::Button {
            action:
                Some(Action::Navigate {
                    path,
                    params,
                    source,
                }),
            ..
        }
        | Node::Field {
            action:
                Some(Action::Navigate {
                    path,
                    params,
                    source,
                }),
            ..
        } => {
            let Some(routes) = routes else {
                return Err(NavigationError {
                    source: source.path.clone(),
                    error: CompileError::new("href requires a Navigator", source.span),
                });
            };
            let Some(expected) = routes.get(path.as_str()) else {
                return Err(NavigationError {
                    source: source.path.clone(),
                    error: CompileError::new(format!("no route matches {path:?}"), source.span),
                });
            };
            let provided = params
                .iter()
                .map(|param| (param.name.as_str(), &param.shape))
                .collect::<HashMap<_, _>>();
            let missing = expected
                .iter()
                .filter(|(name, shape)| {
                    !matches!(shape, StateShape::Optional(_))
                        && !provided.contains_key(name.as_str())
                })
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>();
            let unknown = provided
                .keys()
                .filter(|name| !expected.contains_key(**name))
                .copied()
                .collect::<Vec<_>>();
            let mismatched = provided.iter().find(|(name, shape)| {
                expected
                    .get(**name)
                    .is_some_and(|expected| !shape_accepts_shape(expected, shape))
            });
            let problem = if !missing.is_empty() {
                Some(format!(
                    "route {path:?} is missing params: {}",
                    missing.join(", ")
                ))
            } else if !unknown.is_empty() {
                Some(format!(
                    "route {path:?} has unknown params: {}",
                    unknown.join(", ")
                ))
            } else {
                mismatched.map(|(name, _)| {
                    format!("route param {name:?} has the wrong type for {path:?}")
                })
            };
            if let Some(problem) = problem {
                return Err(NavigationError {
                    source: source.path.clone(),
                    error: CompileError::new(problem, source.span),
                });
            }
        }
        Node::Tabs { tabs, .. } => {
            for tab in tabs {
                validate_navigation_node(&tab.screen, routes)?;
            }
        }
        Node::Navigator { routes: children } => {
            for route in children {
                validate_navigation_node(&route.screen, routes)?;
            }
        }
        Node::Conditional {
            consequent,
            alternate,
            ..
        } => {
            validate_navigation_node(consequent, routes)?;
            if let Some(alternate) = alternate {
                validate_navigation_node(alternate, routes)?;
            }
        }
        Node::ForEach { template, .. } => validate_navigation_node(template, routes)?,
        Node::Text { .. }
        | Node::TextInput { .. }
        | Node::Button { .. }
        | Node::Field { .. }
        | Node::Icon { .. }
        | Node::Image { .. }
        | Node::CameraPreview { .. }
        | Node::Toggle { .. } => {}
        Node::ScreenModule { .. } => unreachable!("screen modules are expanded before validation"),
    }
    Ok(())
}

fn validate_route_path(path: &str, span: Span) -> Result<(), CompileError> {
    if !path.starts_with('/')
        || path.contains("//")
        || path.contains(['?', '#'])
        || (path.len() > 1 && path.ends_with('/'))
    {
        return Err(CompileError::new(
            "route paths start with / and do not use trailing slashes, queries or fragments",
            span,
        ));
    }
    Ok(())
}

fn lower_element_children(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
    parent: &str,
) -> Result<Vec<Node>, CompileError> {
    let children = element_children(element)?
        .into_iter()
        .map(|child| match child {
            JSXChild::Element(child) => lower_content_node(child, states, imports, item),
            JSXChild::ExpressionContainer(container) => {
                let Some(expression) = container.expression.as_expression() else {
                    return Err(CompileError::new(
                        format!("{parent} expressions cannot be empty"),
                        container.span,
                    ));
                };
                lower_dynamic_child(expression, states, imports, item)
            }
            _ => Err(CompileError::new(
                format!("{parent} children must be Ink elements"),
                child.span(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if children.iter().any(contains_camera_preview)
        && (parent != "Screen"
            || children.len() != 1
            || !matches!(children[0], Node::CameraPreview { .. }))
    {
        return Err(CompileError::new(
            "CameraPreview must be the only direct child of Screen",
            element.span,
        ));
    }
    Ok(children)
}

fn contains_camera_preview(node: &Node) -> bool {
    match node {
        Node::CameraPreview { .. } => true,
        Node::Screen { children, .. } | Node::Stack { children, .. } => {
            children.iter().any(contains_camera_preview)
        }
        Node::Tabs { tabs, .. } => tabs.iter().any(|tab| contains_camera_preview(&tab.screen)),
        Node::Navigator { routes } => routes
            .iter()
            .any(|route| contains_camera_preview(&route.screen)),
        Node::Conditional {
            consequent,
            alternate,
            ..
        } => {
            contains_camera_preview(consequent)
                || alternate.as_deref().is_some_and(contains_camera_preview)
        }
        Node::ForEach { template, .. } => contains_camera_preview(template),
        Node::Text { .. }
        | Node::TextInput { .. }
        | Node::Button { .. }
        | Node::Field { .. }
        | Node::Icon { .. }
        | Node::Image { .. }
        | Node::Toggle { .. }
        | Node::ScreenModule { .. } => false,
    }
}

fn lower_dynamic_child(
    expression: &Expression<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    match unparenthesised(expression) {
        Expression::LogicalExpression(expression)
            if expression.operator == LogicalOperator::And =>
        {
            let condition = condition(&expression.left, states)?;
            let consequent =
                dynamic_branch(&expression.right, states, imports, item)?.ok_or_else(|| {
                    CompileError::new(
                        "the visible branch needs an Ink element",
                        expression.right.span(),
                    )
                })?;
            Ok(Node::Conditional {
                condition,
                consequent: Box::new(consequent),
                alternate: None,
            })
        }
        Expression::ConditionalExpression(expression) => {
            let condition = condition(&expression.test, states)?;
            let consequent = dynamic_branch(&expression.consequent, states, imports, item)?;
            let alternate = dynamic_branch(&expression.alternate, states, imports, item)?;
            let (condition, consequent, alternate) = match (consequent, alternate) {
                (Some(consequent), alternate) => (condition, consequent, alternate),
                (None, Some(alternate)) => (invert_condition(condition), alternate, None),
                (None, None) => {
                    return Err(CompileError::new(
                        "a conditional needs at least one Ink element",
                        expression.span,
                    ));
                }
            };
            Ok(Node::Conditional {
                condition,
                consequent: Box::new(consequent),
                alternate: alternate.map(Box::new),
            })
        }
        Expression::CallExpression(expression) if match_call(expression, imports) => {
            lower_match(expression, states, imports, item)
        }
        Expression::CallExpression(expression) => {
            lower_collection(expression, states, imports, item)
        }
        _ => Err(CompileError::new(
            "dynamic children use match(...), state.value && <Element>, a conditional, or state.value.map(...)",
            expression.span(),
        )),
    }
}

fn match_call(call: &oxc::ast::ast::CallExpression<'_>, imports: &Imports) -> bool {
    matches!(
        &call.callee,
        Expression::Identifier(callee)
            if callee.name == "match" && imports.ink.contains("match")
    )
}

fn lower_match(
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let [value, Argument::ObjectExpression(case_object)] = call.arguments.as_slice() else {
        return Err(CompileError::new(
            "match() needs an Ink value and a cases object",
            call.span,
        ));
    };
    let Some(Expression::Identifier(value)) = value.as_expression() else {
        return Err(CompileError::new(
            "match() accepts a resource or controller variable",
            value.span(),
        ));
    };
    let binding = if let Some(resource) = states.resources.get(value.name.as_str()) {
        MatchBinding::Resource(resource.clone())
    } else if let Some(controller) = states.controllers.get(value.name.as_str()) {
        MatchBinding::Controller(controller.clone())
    } else if let Some(combined) = states.combined.get(value.name.as_str()) {
        MatchBinding::Combined(combined.clone())
    } else {
        return Err(CompileError::new(
            "match() accepts a resource or controller variable",
            value.span,
        ));
    };
    let statuses = match_statuses(&binding);
    let mut cases = HashMap::new();

    for property in &case_object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "match cases cannot use spreads",
                property.span(),
            ));
        };
        if property.kind != PropertyKind::Init
            || property.method
            || property.shorthand
            || property.computed
        {
            return Err(CompileError::new(
                "match cases use named arrow functions",
                property.span,
            ));
        }
        let status = property_name(&property.key)?;
        if !statuses.contains(&status) {
            return Err(CompileError::new(
                format!("unknown match case {status:?}"),
                property.key.span(),
            ));
        }
        if cases.contains_key(&status) {
            return Err(CompileError::new(
                format!("match case {status:?} is declared twice"),
                property.key.span(),
            ));
        }
        let Expression::ArrowFunctionExpression(function) = &property.value else {
            return Err(CompileError::new(
                "match cases must be arrow functions",
                property.value.span(),
            ));
        };
        if function.r#async || function.params.rest.is_some() {
            return Err(CompileError::new(
                "match cases must be synchronous arrow functions",
                function.span,
            ));
        }
        let parameter = match function.params.items.as_slice() {
            [] => None,
            [formal] if formal.initializer.is_none() => {
                let BindingPattern::BindingIdentifier(parameter) = &formal.pattern else {
                    return Err(CompileError::new(
                        "match case parameters must be identifiers",
                        formal.pattern.span(),
                    ));
                };
                Some(parameter.name.as_str())
            }
            _ => {
                return Err(CompileError::new(
                    "match cases accept zero or one parameter",
                    function.params.span,
                ));
            }
        };
        let Some(body) = function.body.as_expression() else {
            return Err(CompileError::new(
                "match cases must directly return an Ink element",
                function.body.span(),
            ));
        };
        let Expression::JSXElement(element) = unparenthesised(body) else {
            return Err(CompileError::new(
                "match cases must directly return an Ink element",
                body.span(),
            ));
        };
        let mut branch_bindings = states.clone();
        if let Some(parameter) = parameter {
            branch_bindings.states.remove(parameter);
            branch_bindings.computed.remove(parameter);
            branch_bindings.combined.remove(parameter);
            branch_bindings.route_params.remove(parameter);
            branch_bindings.resources.remove(parameter);
            branch_bindings.controllers.remove(parameter);
            match &binding {
                MatchBinding::Resource(resource) => {
                    branch_bindings
                        .resources
                        .insert(parameter.to_owned(), resource.clone());
                }
                MatchBinding::Controller(controller) => {
                    branch_bindings
                        .controllers
                        .insert(parameter.to_owned(), controller.clone());
                }
                MatchBinding::Combined(combined) => {
                    branch_bindings
                        .combined
                        .insert(parameter.to_owned(), combined.clone());
                }
            }
        }
        cases.insert(
            status,
            lower_content_node(element, &branch_bindings, imports, item)?,
        );
    }

    let missing = statuses
        .iter()
        .filter(|status| !cases.contains_key(status.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(CompileError::new(
            format!("match() is missing cases: {}", missing.join(", ")),
            case_object.span,
        ));
    }

    let mut statuses = statuses.iter().rev();
    let final_status = statuses.next().expect("matchable values have statuses");
    let mut node = cases
        .remove(final_status)
        .expect("exhaustive match contains the final case");
    for status in statuses {
        let consequent = cases
            .remove(status)
            .expect("exhaustive match contains every case");
        node = Node::Conditional {
            condition: match_condition(&binding, status),
            consequent: Box::new(consequent),
            alternate: Some(Box::new(node)),
        };
    }
    Ok(node)
}

fn match_statuses(binding: &MatchBinding) -> Vec<String> {
    match binding {
        MatchBinding::Resource(resource) => match resource.protocol {
            ResourceProtocol::Async => vec!["loading", "ready", "error"],
            ResourceProtocol::Cached => vec!["loading", "ready", "stale", "error"],
            ResourceProtocol::Mutation => vec!["idle", "running", "ready", "error"],
            ResourceProtocol::Background => vec!["waiting", "ready", "stale", "error"],
        },
        MatchBinding::Controller(controller) => return controller_statuses(&controller.shape),
        MatchBinding::Combined(_) => vec!["error", "loading", "ready"],
    }
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn controller_statuses(shape: &StateShape) -> Vec<String> {
    let StateShape::Object(fields) = shape else {
        unreachable!("controller state is an object")
    };
    let (_, StateShape::Union(statuses)) = fields
        .iter()
        .find(|(name, _)| name.as_str() == "status")
        .expect("controller state has a status")
    else {
        unreachable!("controller status is a literal union")
    };
    statuses
        .iter()
        .map(|status| match status {
            StateShape::Literal(StateLiteral::String(status)) => status.clone(),
            _ => unreachable!("controller statuses are string literals"),
        })
        .collect()
}

fn match_condition(binding: &MatchBinding, status: &str) -> Condition {
    let value = StateValue::String(status.to_owned());
    match binding {
        MatchBinding::Resource(resource) => Condition::ResourceEquals {
            resource: resource.id,
            field: ResourceField::Status,
            value,
            expected: true,
        },
        MatchBinding::Controller(controller) => Condition::ControllerEquals {
            controller: controller.id,
            path: vec!["status".to_owned()],
            value,
            expected: true,
        },
        MatchBinding::Combined(combined) => Condition::ValueEquals {
            value: Value::CombinedStatus(
                combined
                    .resources
                    .iter()
                    .map(|(_, resource)| resource.id)
                    .collect(),
            ),
            expected: value,
            equals: true,
        },
    }
}

fn condition(expression: &Expression<'_>, states: &Bindings) -> Result<Condition, CompileError> {
    match unparenthesised(expression) {
        Expression::UnaryExpression(expression)
            if expression.operator == UnaryOperator::LogicalNot =>
        {
            condition(&expression.argument, states).map(invert_condition)
        }
        Expression::BinaryExpression(expression) => {
            let is_length = matches!(
                unparenthesised(&expression.left),
                Expression::StaticMemberExpression(member)
                    if member.property.name.as_str() == "length"
            );
            if !is_length {
                if let Some((derived, kind)) = condition_value(&expression.left, states)? {
                    let value = literal_state_value(unparenthesised(&expression.right))?;
                    if !shape_accepts_value(&kind, &value) {
                        return Err(CompileError::new(
                            "the comparison value must match the computed or route value type",
                            expression.right.span(),
                        ));
                    }
                    let equals = match expression.operator {
                        BinaryOperator::Equality | BinaryOperator::StrictEquality => true,
                        BinaryOperator::Inequality | BinaryOperator::StrictInequality => false,
                        _ => {
                            return Err(CompileError::new(
                                "computed and route comparisons use === or !==",
                                expression.span,
                            ));
                        }
                    };
                    return Ok(Condition::ValueEquals {
                        value: derived,
                        expected: value,
                        equals,
                    });
                }
                if let Some(binding) = expression_controller_value(&expression.left, states)? {
                    if !scalar_kind(&binding.kind) {
                        return Err(CompileError::new(
                            "controller comparisons support numbers, booleans and strings",
                            expression.span,
                        ));
                    }
                    let value = literal_state_value(unparenthesised(&expression.right))?;
                    if !shape_accepts_value(&binding.kind, &value) {
                        return Err(CompileError::new(
                            "the comparison value must match the controller field type",
                            expression.right.span(),
                        ));
                    }
                    let expected = match expression.operator {
                        BinaryOperator::Equality | BinaryOperator::StrictEquality => true,
                        BinaryOperator::Inequality | BinaryOperator::StrictInequality => false,
                        _ => {
                            return Err(CompileError::new(
                                "controller comparisons use === or !==",
                                expression.span,
                            ));
                        }
                    };
                    return Ok(Condition::ControllerEquals {
                        controller: binding.controller,
                        path: binding.path,
                        value,
                        expected,
                    });
                }
                if let Some(binding) = expression_resource_value(&expression.left, states)? {
                    if !scalar_kind(&binding.kind) {
                        return Err(CompileError::new(
                            "resource comparisons support numbers, booleans and strings",
                            expression.span,
                        ));
                    }
                    let value = literal_state_value(unparenthesised(&expression.right))?;
                    if !shape_accepts_value(&binding.kind, &value) {
                        return Err(CompileError::new(
                            "the comparison value must match the resource field type",
                            expression.right.span(),
                        ));
                    }
                    validate_resource_comparison(&binding.field, &value, expression.right.span())?;
                    let expected = match expression.operator {
                        BinaryOperator::Equality | BinaryOperator::StrictEquality => true,
                        BinaryOperator::Inequality | BinaryOperator::StrictInequality => false,
                        _ => {
                            return Err(CompileError::new(
                                "resource comparisons use === or !==",
                                expression.span,
                            ));
                        }
                    };
                    return Ok(Condition::ResourceEquals {
                        resource: binding.resource,
                        field: binding.field,
                        value,
                        expected,
                    });
                }
                let binding = expression_state_value(&expression.left, states)?;
                if !scalar_kind(&binding.kind) {
                    return Err(CompileError::new(
                        "state comparisons support numbers, booleans and strings",
                        expression.span,
                    ));
                }
                let value = literal_state_value(unparenthesised(&expression.right))?;
                if !shape_accepts_value(&binding.kind, &value) {
                    return Err(CompileError::new(
                        "the comparison value must match the state type",
                        expression.right.span(),
                    ));
                }
                let expected = match expression.operator {
                    BinaryOperator::Equality | BinaryOperator::StrictEquality => true,
                    BinaryOperator::Inequality | BinaryOperator::StrictInequality => false,
                    _ => {
                        return Err(CompileError::new(
                            "state comparisons use === or !==",
                            expression.span,
                        ));
                    }
                };
                return Ok(Condition::Equals {
                    state: binding.id,
                    value,
                    expected,
                });
            }

            let state = list_length_state(&expression.left, states)?;
            let Expression::NumericLiteral(value) = unparenthesised(&expression.right) else {
                return Err(CompileError::new(
                    "list length conditions compare with zero",
                    expression.right.span(),
                ));
            };
            if value.value != 0.0 {
                return Err(CompileError::new(
                    "list length conditions compare with zero",
                    value.span,
                ));
            }
            let expected = match expression.operator {
                BinaryOperator::Equality | BinaryOperator::StrictEquality => true,
                BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
                | BinaryOperator::GreaterThan => false,
                _ => {
                    return Err(CompileError::new(
                        "list length supports === 0, !== 0 or > 0",
                        expression.span,
                    ));
                }
            };
            Ok(Condition::ListEmpty { state, expected })
        }
        expression => {
            if let Some((value, kind)) = condition_value(expression, states)? {
                if scalar_base_kind(&kind) != Some(StateShape::Bool) {
                    return Err(CompileError::new(
                        "conditional computed or route value must be boolean",
                        expression.span(),
                    ));
                }
                return Ok(Condition::ValueEquals {
                    value,
                    expected: StateValue::Bool(true),
                    equals: true,
                });
            }
            if let Some(binding) = expression_controller_value(expression, states)? {
                if binding.kind != StateShape::Bool {
                    return Err(CompileError::new(
                        "conditional controller field must be boolean",
                        expression.span(),
                    ));
                }
                return Ok(Condition::ControllerEquals {
                    controller: binding.controller,
                    path: binding.path,
                    value: StateValue::Bool(true),
                    expected: true,
                });
            }
            if let Some(binding) = expression_resource_value(expression, states)? {
                if binding.kind != StateShape::Bool {
                    return Err(CompileError::new(
                        "conditional resource field must be boolean",
                        expression.span(),
                    ));
                }
                return Ok(Condition::ResourceEquals {
                    resource: binding.resource,
                    field: binding.field,
                    value: StateValue::Bool(true),
                    expected: true,
                });
            }
            let binding = expression_state_value(expression, states)?;
            if binding.kind != StateShape::Bool {
                return Err(CompileError::new(
                    "conditional state must be boolean",
                    expression.span(),
                ));
            }
            Ok(Condition::Bool {
                state: binding.id,
                expected: true,
            })
        }
    }
}

fn condition_value(
    expression: &Expression<'_>,
    states: &Bindings,
) -> Result<Option<(Value, StateShape)>, CompileError> {
    expression_derived_value(expression, states)
}

fn invert_condition(condition: Condition) -> Condition {
    match condition {
        Condition::ValueEquals {
            value,
            expected,
            equals,
        } => Condition::ValueEquals {
            value,
            expected,
            equals: !equals,
        },
        Condition::Bool { state, expected } => Condition::Bool {
            state,
            expected: !expected,
        },
        Condition::ListEmpty { state, expected } => Condition::ListEmpty {
            state,
            expected: !expected,
        },
        Condition::Equals {
            state,
            value,
            expected,
        } => Condition::Equals {
            state,
            value,
            expected: !expected,
        },
        Condition::ResourceEquals {
            resource,
            field,
            value,
            expected,
        } => Condition::ResourceEquals {
            resource,
            field,
            value,
            expected: !expected,
        },
        Condition::ControllerEquals {
            controller,
            path,
            value,
            expected,
        } => Condition::ControllerEquals {
            controller,
            path,
            value,
            expected: !expected,
        },
    }
}

fn list_length_state(
    expression: &Expression<'_>,
    states: &Bindings,
) -> Result<StateId, CompileError> {
    let Expression::StaticMemberExpression(length) = unparenthesised(expression) else {
        return Err(CompileError::new(
            "expected list.value.length",
            expression.span(),
        ));
    };
    if length.property.name.as_str() != "length" {
        return Err(CompileError::new("expected list.value.length", length.span));
    }
    let binding = expression_state_value(&length.object, states)?;
    if !matches!(binding.kind, StateShape::List(_)) {
        return Err(CompileError::new(
            "length requires list state",
            expression.span(),
        ));
    }
    Ok(binding.id)
}

fn dynamic_branch(
    expression: &Expression<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Option<Node>, CompileError> {
    match unparenthesised(expression) {
        Expression::JSXElement(element) => {
            lower_content_node(element, states, imports, item).map(Some)
        }
        Expression::LogicalExpression(_) | Expression::ConditionalExpression(_) => {
            lower_dynamic_child(expression, states, imports, item).map(Some)
        }
        Expression::NullLiteral(_) => Ok(None),
        expression => Err(CompileError::new(
            "conditional branches must be an Ink element or null",
            expression.span(),
        )),
    }
}

fn lower_collection(
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    imports: &Imports,
    current_item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    if current_item.is_some() {
        return Err(CompileError::new(
            "nested list rendering is not supported yet",
            call.span,
        ));
    }
    let Expression::StaticMemberExpression(map) = &call.callee else {
        return Err(CompileError::new(
            "collections use state.value.map((item) => <Element>)",
            call.span,
        ));
    };
    if map.property.name.as_str() != "map" {
        return Err(CompileError::new(
            "collections use state.value.map((item) => <Element>)",
            map.span,
        ));
    }
    let (collection, collection_kind) =
        if let Some(controller) = expression_controller_value(&map.object, states)? {
            (
                Collection::Controller(controller.controller, controller.path),
                controller.kind,
            )
        } else if let Some(resource) = expression_resource_value(&map.object, states)? {
            let ResourceField::Value(path) = resource.field else {
                return Err(CompileError::new(
                    "map requires a resource value list",
                    map.object.span(),
                ));
            };
            (Collection::Resource(resource.resource, path), resource.kind)
        } else {
            let binding = expression_state_value(&map.object, states)?;
            (Collection::State(binding.id), binding.kind)
        };
    let StateShape::List(item_kind) = &collection_kind else {
        return Err(CompileError::new(
            "map requires a list value",
            map.object.span(),
        ));
    };
    let [Argument::ArrowFunctionExpression(function)] = call.arguments.as_slice() else {
        return Err(CompileError::new("map needs one arrow function", call.span));
    };
    let [formal] = function.params.items.as_slice() else {
        return Err(CompileError::new(
            "map's arrow function needs one item parameter",
            function.params.span,
        ));
    };
    let BindingPattern::BindingIdentifier(parameter) = &formal.pattern else {
        return Err(CompileError::new(
            "map's item parameter must be an identifier",
            formal.pattern.span(),
        ));
    };
    if function.r#async || function.params.rest.is_some() || formal.initializer.is_some() {
        return Err(CompileError::new(
            "map needs a synchronous arrow function with one item parameter",
            function.span,
        ));
    }
    let Some(body) = function.body.as_expression() else {
        return Err(CompileError::new(
            "map's arrow function must directly return an Ink element",
            function.body.span(),
        ));
    };
    let Expression::JSXElement(element) = unparenthesised(body) else {
        return Err(CompileError::new(
            "map's arrow function must directly return an Ink element",
            body.span(),
        ));
    };
    let item = ItemBinding {
        name: parameter.name.as_str(),
        kind: item_kind,
    };
    Ok(Node::ForEach {
        collection,
        template: Box::new(lower_content_node(element, states, imports, Some(item))?),
    })
}

fn action_attribute(
    element: &JSXElement<'_>,
    name: &str,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Action, CompileError> {
    let attribute = attribute(element, name).ok_or_else(|| {
        CompileError::new(
            format!(
                "{} requires {name}",
                element_name(&element.opening_element.name).unwrap_or("element")
            ),
            element.opening_element.span,
        )
    })?;
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value else {
        return Err(CompileError::new(
            format!("{name} must be an arrow function"),
            attribute.span,
        ));
    };
    let JSXExpression::ArrowFunctionExpression(function) = &container.expression else {
        return Err(CompileError::new(
            format!("{name} must be an arrow function"),
            container.span,
        ));
    };
    if function.r#async || !function.params.items.is_empty() {
        return Err(CompileError::new(
            format!("{name} must be a synchronous zero-argument arrow function"),
            function.span,
        ));
    }
    lower_action(&function.body, states, item)
}

fn optional_button_action_attribute(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Option<Action>, CompileError> {
    let Some(attribute) = attribute(element, "onPress") else {
        return Ok(None);
    };
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value else {
        return Err(CompileError::new(
            "onPress must be an arrow function",
            attribute.span,
        ));
    };
    let JSXExpression::ArrowFunctionExpression(function) = &container.expression else {
        return Err(CompileError::new(
            "onPress must be an arrow function",
            container.span,
        ));
    };
    if function.r#async || function.params.rest.is_some() || !function.params.items.is_empty() {
        return Err(CompileError::new(
            "onPress must be a synchronous zero-argument arrow function",
            function.span,
        ));
    }

    let actions = match &function.body {
        ArrowFunctionBody::CallExpression(call) => {
            vec![lower_button_call(call, states, imports, item)?]
        }
        ArrowFunctionBody::FunctionBody(body) => body
            .statements
            .iter()
            .map(|statement| {
                let Statement::ExpressionStatement(statement) = statement else {
                    return Err(CompileError::new(
                        "button actions contain state mutations and back()",
                        statement.span(),
                    ));
                };
                let Expression::CallExpression(call) = unparenthesised(&statement.expression)
                else {
                    return Err(CompileError::new(
                        "button actions contain state mutations and back()",
                        statement.span,
                    ));
                };
                lower_button_call(call, states, imports, item)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(CompileError::new(
                "button actions contain state mutations and back()",
                function.body.span(),
            ));
        }
    };
    match actions.as_slice() {
        [] => Err(CompileError::new(
            "button actions cannot be empty",
            function.body.span(),
        )),
        [action] => Ok(Some(action.clone())),
        _ => Ok(Some(Action::Sequence(actions))),
    }
}

fn lower_button_call(
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Action, CompileError> {
    if let Expression::Identifier(callee) = &call.callee
        && callee.name.as_str() == "back"
    {
        require_import(imports, "back", call.span)?;
        if !call.arguments.is_empty() || call.type_arguments.is_some() {
            return Err(CompileError::new("back() takes no arguments", call.span));
        }
        return Ok(Action::Back);
    }
    lower_state_action(call, states, item)
}

fn text_change_attribute(
    element: &JSXElement<'_>,
    name: &str,
    state: &StateBinding,
    states: &Bindings,
) -> Result<(), CompileError> {
    let attribute = attribute(element, name).ok_or_else(|| {
        CompileError::new(
            format!("TextInput requires {name}"),
            element.opening_element.span,
        )
    })?;
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value else {
        return Err(CompileError::new(
            format!("{name} must update the TextInput state"),
            attribute.span,
        ));
    };
    let JSXExpression::ArrowFunctionExpression(function) = &container.expression else {
        return Err(CompileError::new(
            format!("{name} must update the TextInput state"),
            container.span,
        ));
    };
    let [formal] = function.params.items.as_slice() else {
        return Err(CompileError::new(
            format!("{name} must have one value parameter"),
            function.params.span,
        ));
    };
    let BindingPattern::BindingIdentifier(parameter) = &formal.pattern else {
        return Err(CompileError::new(
            format!("{name} must have one value parameter"),
            formal.span,
        ));
    };
    let ArrowFunctionBody::CallExpression(call) = &function.body else {
        return Err(CompileError::new(
            format!("{name} must call state.set(value)"),
            function.body.span(),
        ));
    };
    let Expression::StaticMemberExpression(callee) = &call.callee else {
        return Err(CompileError::new(
            format!("{name} must call state.set(value)"),
            call.callee.span(),
        ));
    };
    let Expression::Identifier(state_object) = &callee.object else {
        return Err(CompileError::new(
            format!("{name} must call state.set(value)"),
            callee.object.span(),
        ));
    };
    let [Argument::Identifier(value)] = call.arguments.as_slice() else {
        return Err(CompileError::new(
            format!("{name} must call state.set(value)"),
            call.span,
        ));
    };
    let bound_state = states.get(state_object.name.as_str());
    if function.r#async
        || function.params.rest.is_some()
        || formal.initializer.is_some()
        || callee.property.name.as_str() != "set"
        || bound_state != Some(state)
        || value.name != parameter.name
    {
        return Err(CompileError::new(
            format!("{name} must update the TextInput state with state.set(value)"),
            function.span,
        ));
    }
    Ok(())
}

fn lower_action(
    body: &ArrowFunctionBody<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Action, CompileError> {
    let ArrowFunctionBody::CallExpression(call) = body else {
        return Err(CompileError::new(
            "an action supports one state mutation call",
            body.span(),
        ));
    };
    lower_state_action(call, states, item)
}

fn lower_state_action(
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Action, CompileError> {
    if let Expression::Identifier(callee) = &call.callee
        && states.extension_functions.get(callee.name.as_str())
            == Some(&ExtensionFunction::OpenDialler)
    {
        return lower_open_dialler_action(call, states, item);
    }
    let Expression::StaticMemberExpression(callee) = &call.callee else {
        return Err(CompileError::new(
            "expected a state mutation",
            call.callee.span(),
        ));
    };
    let Expression::Identifier(state_object) = &callee.object else {
        return Err(CompileError::new(
            "expected a state mutation",
            callee.object.span(),
        ));
    };
    let name = state_object.name.as_str();
    let method = callee.property.name.as_str();
    if let Some(binding) = states.controllers.get(name) {
        if call.type_arguments.is_some() {
            return Err(CompileError::new(
                "native controller actions do not take type arguments",
                call.span,
            ));
        }
        let payload = lower_controller_action(binding.kind, method, call, states, item)?;
        return Ok(Action::Controller {
            controller: binding.id,
            operation: method.to_owned(),
            payload,
        });
    }
    if let Some(binding) = states.resources.get(name) {
        if !call.arguments.is_empty() || call.type_arguments.is_some() {
            return Err(CompileError::new(
                "resource actions take no arguments",
                call.span,
            ));
        }
        if binding.protocol == ResourceProtocol::Background {
            return Err(CompileError::new(
                "background resources do not expose actions",
                call.span,
            ));
        }
        if binding.protocol == ResourceProtocol::Mutation {
            return match method {
                "run" => Ok(Action::ReloadResource {
                    resource: binding.id,
                }),
                _ => Err(CompileError::new("mutations expose run()", call.span)),
            };
        }
        return match method {
            "reload" => Ok(Action::ReloadResource {
                resource: binding.id,
            }),
            "request" => binding
                .request
                .clone()
                .map(|operation| Action::Native { operation })
                .ok_or_else(|| CompileError::new("this resource cannot request access", call.span)),
            _ => Err(CompileError::new(
                format!("unknown resource action {name}.{method}()"),
                call.span,
            )),
        };
    }
    let binding = states.get(name).cloned().ok_or_else(|| {
        CompileError::new(
            format!("unknown state, resource or controller {name}"),
            state_object.span,
        )
    })?;
    let StateShape::List(item_kind) = &binding.kind else {
        if method != "set" || call.arguments.len() != 1 {
            return Err(CompileError::new("expected state.set(...)", call.span));
        }
        return lower_scalar_set(call, &binding, states);
    };

    match method {
        "set" => {
            let [value] = call.arguments.as_slice() else {
                return Err(CompileError::new("list.set needs one list", call.span));
            };
            let Some(value) = value.as_expression() else {
                return Err(CompileError::new(
                    "list values cannot use spread syntax",
                    value.span(),
                ));
            };
            Ok(Action::SetList {
                state: binding.id,
                value: lower_value(value, &binding.kind, states, item)?,
            })
        }
        "append" => {
            let [value] = call.arguments.as_slice() else {
                return Err(CompileError::new("list.append needs one item", call.span));
            };
            let Some(value) = value.as_expression() else {
                return Err(CompileError::new(
                    "list items cannot use spread syntax",
                    value.span(),
                ));
            };
            Ok(Action::AppendList {
                state: binding.id,
                value: lower_value(value, item_kind, states, item)?,
            })
        }
        "remove" => {
            let [value] = call.arguments.as_slice() else {
                return Err(CompileError::new(
                    "list.remove needs one mapped item",
                    call.span,
                ));
            };
            let Some(value) = value.as_expression() else {
                return Err(CompileError::new(
                    "list.remove needs the mapped item",
                    value.span(),
                ));
            };
            require_current_item(value, item_kind, item)?;
            Ok(Action::RemoveListItem { state: binding.id })
        }
        "replace" => {
            let [current, value] = call.arguments.as_slice() else {
                return Err(CompileError::new(
                    "list.replace needs a mapped item and its replacement",
                    call.span,
                ));
            };
            let (Some(current), Some(value)) = (current.as_expression(), value.as_expression())
            else {
                return Err(CompileError::new(
                    "list.replace does not support spread syntax",
                    call.span,
                ));
            };
            require_current_item(current, item_kind, item)?;
            Ok(Action::ReplaceListItem {
                state: binding.id,
                value: lower_value(value, item_kind, states, item)?,
            })
        }
        "clear" if call.arguments.is_empty() => Ok(Action::ClearList { state: binding.id }),
        _ => Err(CompileError::new(
            "lists support set, append, remove, replace and clear",
            call.span,
        )),
    }
}

fn lower_open_dialler_action(
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Action, CompileError> {
    if call.type_arguments.is_some() {
        return Err(CompileError::new(
            "openDialler() does not take type arguments",
            call.span,
        ));
    }
    let [argument] = call.arguments.as_slice() else {
        return Err(CompileError::new(
            "openDialler() takes one phone number",
            call.span,
        ));
    };
    let expression = argument.as_expression().ok_or_else(|| {
        CompileError::new("phone numbers cannot use spread syntax", argument.span())
    })?;
    if let Expression::StringLiteral(value) = expression {
        let number = value.value.as_str();
        if number.trim().is_empty() || number.len() > 64 || number.chars().any(char::is_control) {
            return Err(CompileError::new(
                "phone numbers must contain 1–64 non-control characters",
                value.span,
            ));
        }
    }
    Ok(Action::Native {
        operation: NativeOperation {
            module: "light-sdk".to_owned(),
            operation: "open-dialler".to_owned(),
            payload: vec![
                PayloadPart::Literal("{\"phoneNumber\":".to_owned()),
                notification_string_part(expression, states, item, "phone number")?,
                PayloadPart::Literal("}".to_owned()),
            ],
            timeout_ms: 10_000,
        },
    })
}

fn lower_controller_action(
    kind: NativeControllerKind,
    method: &str,
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Vec<PayloadPart>, CompileError> {
    match kind {
        NativeControllerKind::Level | NativeControllerKind::Pitch => {
            if !matches!(method, "start" | "stop") || !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "level and pitch controllers support start() and stop()",
                    call.span,
                ));
            }
            Ok(vec![PayloadPart::Literal(String::new())])
        }
        NativeControllerKind::Recorder => {
            if !matches!(method, "start" | "stop" | "cancel" | "delete")
                || !call.arguments.is_empty()
            {
                return Err(CompileError::new(
                    "audioRecorder supports start(), stop(), cancel() and delete()",
                    call.span,
                ));
            }
            Ok(vec![PayloadPart::Literal("{}".to_owned())])
        }
        NativeControllerKind::Player => player_action_payload(method, call, states, item),
        NativeControllerKind::Notifications => {
            notification_action_payload(method, call, states, item)
        }
        NativeControllerKind::NotificationTap => {
            if method != "consume" || !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "notificationTap supports consume()",
                    call.span,
                ));
            }
            Ok(vec![PayloadPart::Literal("{}".to_owned())])
        }
        NativeControllerKind::Photo | NativeControllerKind::Scanner => {
            if method != "open" || !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "camera sessions support open()",
                    call.span,
                ));
            }
            Ok(vec![PayloadPart::Literal("{}".to_owned())])
        }
        NativeControllerKind::RingtoneInstaller => ringtone_action_payload(method, call, states),
        NativeControllerKind::LightPush => light_push_action_payload(method, call, states, item),
    }
}

fn ringtone_action_payload(
    method: &str,
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
) -> Result<Vec<PayloadPart>, CompileError> {
    if method != "set" || !(1..=2).contains(&call.arguments.len()) {
        return Err(CompileError::new(
            "ringtoneInstaller supports set(source, kind?)",
            call.span,
        ));
    }
    let Argument::StringLiteral(source) = &call.arguments[0] else {
        return Err(CompileError::new(
            "ringtone sources must be local string literals",
            call.arguments[0].span(),
        ));
    };
    let relative = source.value.as_str();
    if !relative.starts_with("./") {
        return Err(CompileError::new(
            "ringtone sources must start with ./",
            source.span,
        ));
    }
    let path = states
        .source_path
        .parent()
        .expect("a source file has a parent")
        .join(relative)
        .canonicalize()
        .map_err(|_| {
            CompileError::new(
                format!("could not find ringtone asset {relative:?}"),
                source.span,
            )
        })?;
    if !path.is_file() {
        return Err(CompileError::new(
            format!("ringtone asset {relative:?} is not a file"),
            source.span,
        ));
    }
    let kind = match call.arguments.get(1) {
        None => "ringtone",
        Some(Argument::StringLiteral(value))
            if matches!(value.value.as_str(), "ringtone" | "notification" | "alarm") =>
        {
            value.value.as_str()
        }
        Some(value) => {
            return Err(CompileError::new(
                "ringtone kind must be ringtone, notification or alarm",
                value.span(),
            ));
        }
    };
    Ok(vec![PayloadPart::Literal(
        serde_json::json!({
            "source": format!("ink-file://{}", path.display()),
            "kind": kind,
        })
        .to_string(),
    )])
}

fn light_push_action_payload(
    method: &str,
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Vec<PayloadPart>, CompileError> {
    if matches!(method, "retry" | "unregister" | "clear") {
        if !call.arguments.is_empty() {
            return Err(CompileError::new(
                format!("{method}() takes no arguments"),
                call.span,
            ));
        }
        return Ok(vec![PayloadPart::Literal("{}".to_owned())]);
    }
    if method == "dismiss" {
        let [argument] = call.arguments.as_slice() else {
            return Err(CompileError::new(
                "dismiss() takes one group key",
                call.span,
            ));
        };
        let expression = argument.as_expression().ok_or_else(|| {
            CompileError::new("group keys cannot use spread syntax", argument.span())
        })?;
        return Ok(vec![
            PayloadPart::Literal("{\"groupKey\":".to_owned()),
            notification_string_part(expression, states, item, "group key")?,
            PayloadPart::Literal("}".to_owned()),
        ]);
    }
    if method != "register" || !(1..=2).contains(&call.arguments.len()) {
        return Err(CompileError::new(
            "lightPush supports register(baseUrl, bearerToken?), retry(), unregister(), dismiss(groupKey) and clear()",
            call.span,
        ));
    }
    let base = call.arguments[0].as_expression().ok_or_else(|| {
        CompileError::new(
            "subscription URLs cannot use spread syntax",
            call.arguments[0].span(),
        )
    })?;
    let mut payload = vec![PayloadPart::Literal("{\"subscriptionBaseUrl\":".to_owned())];
    payload.push(notification_string_part(
        base,
        states,
        item,
        "subscription base URL",
    )?);
    if let Some(argument) = call.arguments.get(1) {
        let token = argument.as_expression().ok_or_else(|| {
            CompileError::new("bearer tokens cannot use spread syntax", argument.span())
        })?;
        payload.push(PayloadPart::Literal(",\"bearerToken\":".to_owned()));
        payload.push(notification_string_part(
            token,
            states,
            item,
            "bearer token",
        )?);
    }
    payload.push(PayloadPart::Literal("}".to_owned()));
    Ok(payload)
}

fn notification_action_payload(
    method: &str,
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Vec<PayloadPart>, CompileError> {
    if method == "cancel" {
        let [argument] = call.arguments.as_slice() else {
            return Err(CompileError::new(
                "cancel() takes one notification ID",
                call.span,
            ));
        };
        let expression = argument.as_expression().ok_or_else(|| {
            CompileError::new("notification IDs cannot use spread syntax", argument.span())
        })?;
        let mut payload = vec![PayloadPart::Literal("{\"id\":".to_owned())];
        payload.push(notification_string_part(
            expression,
            states,
            item,
            "notification ID",
        )?);
        payload.push(PayloadPart::Literal("}".to_owned()));
        return Ok(payload);
    }
    if method != "schedule" {
        return Err(CompileError::new(
            "localNotifications supports schedule() and cancel()",
            call.span,
        ));
    }
    let [Argument::ObjectExpression(notification)] = call.arguments.as_slice() else {
        return Err(CompileError::new(
            "schedule() takes one notification object",
            call.span,
        ));
    };
    let allowed = [
        "id",
        "title",
        "body",
        "href",
        "data",
        "delayMs",
        "triggerAtMs",
    ];
    let mut values = BTreeMap::new();
    for property in &notification.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "notifications cannot use spread properties",
                property.span(),
            ));
        };
        let name = property_name(&property.key)?;
        if !allowed.contains(&name.as_str()) {
            return Err(CompileError::new(
                format!("unknown notification property {name:?}"),
                property.key.span(),
            ));
        }
        if values.insert(name.clone(), &property.value).is_some() {
            return Err(CompileError::new(
                format!("notification property {name:?} is declared twice"),
                property.key.span(),
            ));
        }
    }
    for required in ["id", "title", "body"] {
        if !values.contains_key(required) {
            return Err(CompileError::new(
                format!("notifications require {required}"),
                notification.span,
            ));
        }
    }
    if values.contains_key("delayMs") == values.contains_key("triggerAtMs") {
        return Err(CompileError::new(
            "notifications require exactly one of delayMs or triggerAtMs",
            notification.span,
        ));
    }
    let mut payload = vec![PayloadPart::Literal("{".to_owned())];
    let mut first = true;
    for name in allowed {
        let Some(expression) = values.get(name) else {
            continue;
        };
        let comma = if first { "" } else { "," };
        first = false;
        payload.push(PayloadPart::Literal(format!(
            "{comma}{}:",
            serde_json::to_string(name).expect("a field name is valid JSON"),
        )));
        payload.push(if matches!(name, "delayMs" | "triggerAtMs") {
            notification_number_part(expression, states)?
        } else {
            notification_string_part(expression, states, item, name)?
        });
    }
    payload.push(PayloadPart::Literal("}".to_owned()));
    Ok(payload)
}

fn notification_string_part(
    expression: &Expression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
    label: &str,
) -> Result<PayloadPart, CompileError> {
    if let Expression::StringLiteral(value) = expression {
        let text = value.value.as_str();
        if label == "id" || label == "notification ID" {
            let mut characters = text.chars();
            if text.len() > 64
                || !characters
                    .next()
                    .is_some_and(|value| value.is_ascii_alphanumeric())
                || !characters.all(|value| value.is_ascii_alphanumeric() || "._-".contains(value))
            {
                return Err(CompileError::new(
                    "notification IDs must match [A-Za-z0-9][A-Za-z0-9._-]{0,63}",
                    value.span,
                ));
            }
        }
        return Ok(PayloadPart::Literal(
            serde_json::to_string(text).expect("a source string is valid JSON"),
        ));
    }
    if let Some(item) = item
        && let Some(path) = item_path(expression, item.name)
    {
        if kind_at_path(item.kind, &path) != Some(&StateShape::String) {
            return Err(CompileError::new(
                format!("{label} must be a string"),
                expression.span(),
            ));
        }
        return Ok(PayloadPart::Item(path));
    }
    let binding = expression_state_value(expression, states)?;
    if binding.kind != StateShape::String {
        return Err(CompileError::new(
            format!("{label} must be a string"),
            expression.span(),
        ));
    }
    Ok(PayloadPart::State(binding.id))
}

fn notification_number_part(
    expression: &Expression<'_>,
    states: &Bindings,
) -> Result<PayloadPart, CompileError> {
    if let Expression::NumericLiteral(value) = expression {
        if !value.value.is_finite() || value.value < 0.0 {
            return Err(CompileError::new(
                "notification times must be non-negative finite numbers",
                value.span,
            ));
        }
        return Ok(PayloadPart::Literal(value.value.to_string()));
    }
    let binding = expression_state_value(expression, states)?;
    if binding.kind != StateShape::Number {
        return Err(CompileError::new(
            "notification times must be numbers",
            expression.span(),
        ));
    }
    Ok(PayloadPart::State(binding.id))
}

fn player_action_payload(
    method: &str,
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Vec<PayloadPart>, CompileError> {
    if method == "play"
        && call.arguments.len() == 1
        && let Some(expression) = call.arguments[0].as_expression()
    {
        if let Expression::Identifier(value) = expression
            && let Some(item) = item.filter(|item| item.name == value.name.as_str())
        {
            require_audio_item_shape(item.kind, expression.span())?;
            return Ok(vec![
                PayloadPart::Literal("{\"item\":".to_owned()),
                PayloadPart::Item(Vec::new()),
                PayloadPart::Literal("}".to_owned()),
            ]);
        }
        if matches!(expression, Expression::StaticMemberExpression(_)) {
            let binding = expression_state_value(expression, states)?;
            require_audio_item_shape(&binding.kind, expression.span())?;
            return Ok(vec![
                PayloadPart::Literal("{\"item\":".to_owned()),
                PayloadPart::State(binding.id),
                PayloadPart::Literal("}".to_owned()),
            ]);
        }
    }
    if method == "setQueue" && matches!(call.arguments.len(), 1 | 2) {
        let Some(expression) = call.arguments[0].as_expression() else {
            return Err(CompileError::new(
                "audio queues cannot use spreads",
                call.arguments[0].span(),
            ));
        };
        if matches!(expression, Expression::StaticMemberExpression(_)) {
            let binding = expression_state_value(expression, states)?;
            let StateShape::List(item_shape) = &binding.kind else {
                return Err(CompileError::new(
                    "setQueue() requires an audio item list",
                    expression.span(),
                ));
            };
            require_audio_item_shape(item_shape, expression.span())?;
            let start_index = match call.arguments.get(1) {
                None => 0,
                Some(Argument::NumericLiteral(index)) => {
                    let index = integer(index.value, index.span, "startIndex")?;
                    if index < 0 {
                        return Err(CompileError::new(
                            "startIndex cannot be negative",
                            call.arguments[1].span(),
                        ));
                    }
                    index
                }
                Some(argument) => {
                    return Err(CompileError::new(
                        "startIndex must be a number literal",
                        argument.span(),
                    ));
                }
            };
            return Ok(vec![
                PayloadPart::Literal("{\"items\":".to_owned()),
                PayloadPart::State(binding.id),
                PayloadPart::Literal(format!(",\"startIndex\":{start_index}}}")),
            ]);
        }
    }
    let payload = match method {
        "play" => match call.arguments.as_slice() {
            [] => serde_json::json!({}),
            [Argument::ObjectExpression(item)] => {
                serde_json::json!({ "item": audio_item(item, states)? })
            }
            _ => {
                return Err(CompileError::new(
                    "play() accepts one audio item",
                    call.span,
                ));
            }
        },
        "setQueue" => {
            let (items, start_index) = match call.arguments.as_slice() {
                [Argument::ArrayExpression(items)] => (items, 0),
                [
                    Argument::ArrayExpression(items),
                    Argument::NumericLiteral(index),
                ] => (items, integer(index.value, index.span, "startIndex")?),
                _ => {
                    return Err(CompileError::new(
                        "setQueue() accepts an audio item array and optional start index",
                        call.span,
                    ));
                }
            };
            let items = items
                .elements
                .iter()
                .map(|element| {
                    let Some(Expression::ObjectExpression(item)) = element.as_expression() else {
                        return Err(CompileError::new(
                            "audio queues contain item object literals",
                            element.span(),
                        ));
                    };
                    audio_item(item, states)
                })
                .collect::<Result<Vec<_>, _>>()?;
            if start_index < 0 || start_index as usize >= items.len() {
                return Err(CompileError::new(
                    "startIndex must reference an audio item",
                    call.span,
                ));
            }
            serde_json::json!({ "items": items, "startIndex": start_index })
        }
        "seekTo" | "setSpeed" => {
            let [Argument::NumericLiteral(value)] = call.arguments.as_slice() else {
                return Err(CompileError::new(
                    format!("{method}() takes one number literal"),
                    call.span,
                ));
            };
            if !value.value.is_finite() {
                return Err(CompileError::new("audio values must be finite", value.span));
            }
            if method == "setSpeed" && !(0.25..=4.0).contains(&value.value) {
                return Err(CompileError::new(
                    "playback speed must be between 0.25 and 4",
                    value.span,
                ));
            }
            serde_json::json!({ "value": value.value })
        }
        "playRecording" | "pause" | "toggle" | "stop" | "skipBack" | "skipForward" | "previous"
        | "next" => {
            if !call.arguments.is_empty() {
                return Err(CompileError::new(
                    format!("{method}() takes no arguments"),
                    call.span,
                ));
            }
            serde_json::json!({})
        }
        _ => {
            return Err(CompileError::new(
                format!("unknown audio player action {method}()"),
                call.span,
            ));
        }
    };
    Ok(vec![PayloadPart::Literal(payload.to_string())])
}

fn require_audio_item_shape(shape: &StateShape, span: Span) -> Result<(), CompileError> {
    let StateShape::Object(fields) = shape else {
        return Err(CompileError::new("audio items must be objects", span));
    };
    if fields.get("src") != Some(&StateShape::String)
        || fields.get("title") != Some(&StateShape::String)
    {
        return Err(CompileError::new(
            "audio items require string src and title fields",
            span,
        ));
    }
    for optional in ["id", "artist", "album", "artwork"] {
        if fields
            .get(optional)
            .is_some_and(|shape| shape != &StateShape::String)
        {
            return Err(CompileError::new(
                format!("audio item field {optional} must be a string"),
                span,
            ));
        }
    }
    Ok(())
}

fn audio_item(
    item: &oxc::ast::ast::ObjectExpression<'_>,
    states: &Bindings,
) -> Result<serde_json::Value, CompileError> {
    let mut values = serde_json::Map::new();
    for property in &item.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(CompileError::new(
                "audio items cannot use spread properties",
                property.span(),
            ));
        };
        let name = property_name(&property.key)?;
        if !matches!(
            name.as_str(),
            "id" | "src" | "title" | "artist" | "album" | "artwork"
        ) {
            return Err(CompileError::new(
                format!("unknown audio item property {name:?}"),
                property.key.span(),
            ));
        }
        if values.contains_key(&name) {
            return Err(CompileError::new(
                format!("audio item property {name:?} is declared twice"),
                property.key.span(),
            ));
        }
        let Expression::StringLiteral(value) = &property.value else {
            return Err(CompileError::new(
                "audio item values must be string literals",
                property.value.span(),
            ));
        };
        let mut value = value.value.to_string();
        if name == "src" && value.starts_with("./") {
            let source = states
                .source_path
                .parent()
                .expect("a source file has a parent")
                .join(&value);
            let source = source.canonicalize().map_err(|_| {
                CompileError::new(
                    format!("could not find audio asset {value:?}"),
                    property.value.span(),
                )
            })?;
            if !source.is_file() {
                return Err(CompileError::new(
                    format!("audio asset {value:?} is not a file"),
                    property.value.span(),
                ));
            }
            value = format!("ink-file://{}", source.display());
        }
        values.insert(name, serde_json::Value::String(value));
    }
    if !values.contains_key("src") || !values.contains_key("title") {
        return Err(CompileError::new(
            "audio items require src and title",
            item.span,
        ));
    }
    Ok(serde_json::Value::Object(values))
}

fn lower_scalar_set(
    call: &oxc::ast::ast::CallExpression<'_>,
    binding: &StateBinding,
    states: &Bindings,
) -> Result<Action, CompileError> {
    match &call.arguments[0] {
        Argument::BinaryExpression(value) if binding.kind == StateShape::Number => {
            let read_state = expression_state_value(&value.left, states)?;
            if read_state.id != binding.id {
                return Err(CompileError::new(
                    "an action must update the state value it reads",
                    value.left.span(),
                ));
            }
            let Expression::NumericLiteral(amount) = &value.right else {
                return Err(CompileError::new(
                    "the increment must be a number literal",
                    value.right.span(),
                ));
            };
            let mut by = amount.value;
            match value.operator {
                BinaryOperator::Addition => {}
                BinaryOperator::Subtraction => by = -by,
                _ => {
                    return Err(CompileError::new(
                        "state increments support + and -",
                        value.span,
                    ));
                }
            }
            Ok(Action::Increment {
                state: binding.id,
                by,
            })
        }
        Argument::UnaryExpression(value)
            if value.operator == UnaryOperator::LogicalNot && binding.kind == StateShape::Bool =>
        {
            let read_state = expression_state_value(&value.argument, states)?;
            if read_state.id != binding.id {
                return Err(CompileError::new(
                    "an action must update the state value it reads",
                    value.argument.span(),
                ));
            }
            Ok(Action::Toggle { state: binding.id })
        }
        value => {
            let Some(value) = value.as_expression() else {
                return Err(CompileError::new(
                    "state.set value cannot use spread syntax",
                    value.span(),
                ));
            };
            let value = lower_value(value, &binding.kind, states, None)?;
            Ok(Action::SetValue {
                state: binding.id,
                value,
            })
        }
    }
}

fn require_current_item(
    expression: &Expression<'_>,
    expected: &StateShape,
    item: Option<ItemBinding<'_>>,
) -> Result<(), CompileError> {
    let Some(item) = item else {
        return Err(CompileError::new(
            "remove and replace are used inside a list map",
            expression.span(),
        ));
    };
    if item.kind != expected || item_path(expression, item.name) != Some(Vec::new()) {
        return Err(CompileError::new(
            "expected the current mapped item",
            expression.span(),
        ));
    }
    Ok(())
}

fn lower_value(
    expression: &Expression<'_>,
    expected: &StateShape,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Value, CompileError> {
    let expression = unparenthesised(expression);
    if let Some((value, kind)) = expression_derived_value(expression, states)? {
        if shape_accepts_shape(expected, &kind) {
            return Ok(value);
        }
        return Err(CompileError::new(
            "the derived value does not match the target state type",
            expression.span(),
        ));
    }
    let literal = match expression {
        Expression::NullLiteral(_) => Some(StateValue::Null),
        Expression::NumericLiteral(value) if value.value.is_finite() => {
            Some(StateValue::Number(value.value))
        }
        Expression::BooleanLiteral(value) => Some(StateValue::Bool(value.value)),
        Expression::StringLiteral(value) => Some(StateValue::String(value.value.to_string())),
        _ => None,
    };
    if let Some(value) = literal {
        if !shape_accepts_value(expected, &value) {
            return Err(CompileError::new(
                "value does not match the declared type",
                expression.span(),
            ));
        }
        return Ok(match value {
            StateValue::Null => Value::Null,
            StateValue::Number(value) => Value::Number(value),
            StateValue::Bool(value) => Value::Bool(value),
            StateValue::String(value) => Value::String(value),
            StateValue::List(_) | StateValue::Object(_) => unreachable!("a scalar literal"),
        });
    }
    if let Some(item) = item
        && let Some(path) = item_path(expression, item.name)
    {
        let Some(kind) = kind_at_path(item.kind, &path) else {
            return Err(CompileError::new(
                "unknown list-item field",
                expression.span(),
            ));
        };
        if kind != expected {
            return Err(CompileError::new(
                "list-item value has the wrong type",
                expression.span(),
            ));
        }
        return Ok(Value::Item(path));
    }
    if let Expression::StaticMemberExpression(_) = expression {
        let binding = expression_state_value(expression, states)?;
        if &binding.kind != expected {
            return Err(CompileError::new(
                "state value has the wrong type",
                expression.span(),
            ));
        }
        return Ok(Value::State(binding.id));
    }
    match (expression, expected) {
        (Expression::ArrayExpression(array), StateShape::List(item_kind)) => {
            let values = array
                .elements
                .iter()
                .map(|value| {
                    let Some(value) = value.as_expression() else {
                        return Err(CompileError::new(
                            "list values cannot use holes or spread syntax",
                            value.span(),
                        ));
                    };
                    lower_value(value, item_kind, states, item)
                })
                .collect::<Result<_, _>>()?;
            Ok(Value::List(values))
        }
        (Expression::ObjectExpression(object), StateShape::Object(fields)) => {
            let mut values = Vec::with_capacity(object.properties.len());
            let mut seen = HashSet::new();
            for property in &object.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return Err(CompileError::new(
                        "list items cannot use spread properties",
                        property.span(),
                    ));
                };
                if property.kind != PropertyKind::Init
                    || property.method
                    || property.shorthand
                    || property.computed
                {
                    return Err(CompileError::new(
                        "list items use explicit properties",
                        property.span,
                    ));
                }
                let name = property_name(&property.key)?;
                let expected = fields.get(&name).ok_or_else(|| {
                    CompileError::new(
                        format!("unknown list-item property {name:?}"),
                        property.key.span(),
                    )
                })?;
                if !seen.insert(name.clone()) {
                    return Err(CompileError::new(
                        format!("list-item property {name:?} is declared twice"),
                        property.key.span(),
                    ));
                }
                values.push((name, lower_value(&property.value, expected, states, item)?));
            }
            for (name, shape) in fields {
                if !seen.contains(name) && matches!(shape, StateShape::Optional(_)) {
                    values.push((name.clone(), Value::Null));
                }
            }
            if fields.iter().any(|(name, shape)| {
                !matches!(shape, StateShape::Optional(_)) && !seen.contains(name)
            }) {
                return Err(CompileError::new(
                    "list item is missing a required property",
                    object.span,
                ));
            }
            Ok(Value::Object(values))
        }
        _ => Err(CompileError::new(
            "collection value does not match the list's item type",
            expression.span(),
        )),
    }
}

fn state_attribute<'a>(
    element: &JSXElement<'a>,
    name: &str,
    states: &Bindings,
    expected: StateShape,
) -> Result<StateBinding, CompileError> {
    let attribute = attribute(element, name).ok_or_else(|| {
        CompileError::new(
            format!(
                "{} requires {name}",
                element_name(&element.opening_element.name).unwrap_or("element")
            ),
            element.opening_element.span,
        )
    })?;
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value else {
        return Err(CompileError::new(
            format!("{name} must be a state value"),
            attribute.span,
        ));
    };
    let binding = state_value(&container.expression, states)?;
    if binding.kind != expected {
        return Err(CompileError::new(
            format!("{name} has the wrong state type"),
            container.span,
        ));
    }
    Ok(binding)
}

fn state_value(
    expression: &JSXExpression<'_>,
    states: &Bindings,
) -> Result<StateBinding, CompileError> {
    let JSXExpression::StaticMemberExpression(member) = expression else {
        return Err(CompileError::new(
            "expected {state.value}",
            expression.span(),
        ));
    };
    member_state_value(member, states)
}

fn expression_state_value(
    expression: &Expression<'_>,
    states: &Bindings,
) -> Result<StateBinding, CompileError> {
    let Expression::StaticMemberExpression(member) = unparenthesised(expression) else {
        return Err(CompileError::new("expected state.value", expression.span()));
    };
    member_state_value(member, states)
}

fn member_state_value(
    member: &oxc::ast::ast::StaticMemberExpression<'_>,
    states: &Bindings,
) -> Result<StateBinding, CompileError> {
    let Expression::Identifier(object) = &member.object else {
        return Err(CompileError::new(
            "expected state.value",
            member.object.span(),
        ));
    };
    let name = object.name.as_str();
    let property = member.property.name.as_str();
    let binding = (property == "value").then(|| states.get(name)).flatten();
    binding.cloned().ok_or_else(|| {
        CompileError::new(
            format!("unknown state value {name}.{property}"),
            member.span,
        )
    })
}

fn optional_number_attribute(
    element: &JSXElement<'_>,
    name: &str,
) -> Result<Option<f32>, CompileError> {
    let Some(attribute) = attribute(element, name) else {
        return Ok(None);
    };
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value else {
        return Err(CompileError::new(
            format!("{name} must be a number literal"),
            attribute.span,
        ));
    };
    let JSXExpression::NumericLiteral(value) = &container.expression else {
        return Err(CompileError::new(
            format!("{name} must be a number literal"),
            container.span,
        ));
    };
    if !value.value.is_finite() || value.value < 0.0 || value.value > f32::MAX as f64 {
        return Err(CompileError::new(
            format!("{name} must be a finite positive number"),
            value.span,
        ));
    }
    Ok(Some(value.value as f32))
}

fn optional_positive_integer_attribute(
    element: &JSXElement<'_>,
    name: &str,
) -> Result<Option<u32>, CompileError> {
    let Some(attribute) = attribute(element, name) else {
        return Ok(None);
    };
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value else {
        return Err(CompileError::new(
            format!("{name} must be a positive integer"),
            attribute.span,
        ));
    };
    let JSXExpression::NumericLiteral(value) = &container.expression else {
        return Err(CompileError::new(
            format!("{name} must be a positive integer"),
            container.span,
        ));
    };
    let value = integer(value.value, value.span, name)?;
    u32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .map(Some)
        .ok_or_else(|| {
            CompileError::new(format!("{name} must be a positive integer"), container.span)
        })
}

fn required_number_attribute(element: &JSXElement<'_>, name: &str) -> Result<f32, CompileError> {
    optional_number_attribute(element, name)?.ok_or_else(|| {
        CompileError::new(
            format!(
                "{} requires {name}",
                element_name(&element.opening_element.name).unwrap_or("element")
            ),
            element.opening_element.span,
        )
    })
}

fn optional_string_attribute(
    element: &JSXElement<'_>,
    name: &str,
) -> Result<Option<String>, CompileError> {
    let Some(attribute) = attribute(element, name) else {
        return Ok(None);
    };
    let Some(JSXAttributeValue::StringLiteral(value)) = &attribute.value else {
        return Err(CompileError::new(
            format!("{name} must be a string literal"),
            attribute.span,
        ));
    };
    Ok(Some(value.value.as_str().to_owned()))
}

fn required_string_attribute(element: &JSXElement<'_>, name: &str) -> Result<String, CompileError> {
    optional_string_attribute(element, name)?.ok_or_else(|| {
        CompileError::new(
            format!(
                "{} requires {name}",
                element_name(&element.opening_element.name).unwrap_or("element")
            ),
            element.opening_element.span,
        )
    })
}

fn optional_icon_attribute(
    element: &JSXElement<'_>,
    name: &str,
) -> Result<Option<String>, CompileError> {
    let Some(value) = optional_string_attribute(element, name)? else {
        return Ok(None);
    };
    validate_icon(element, name, value).map(Some)
}

fn required_icon_attribute(element: &JSXElement<'_>, name: &str) -> Result<String, CompileError> {
    let value = required_string_attribute(element, name)?;
    validate_icon(element, name, value)
}

fn validate_icon(
    element: &JSXElement<'_>,
    attribute_name: &str,
    value: String,
) -> Result<String, CompileError> {
    let value = value.replace('-', "_");
    if crate::icons::exists(&value) {
        Ok(value)
    } else {
        let span = attribute(element, attribute_name)
            .map_or(element.opening_element.span, |attribute| attribute.span);
        Err(CompileError::new(
            format!("unknown Material Symbol {value:?}"),
            span,
        ))
    }
}

fn boolean_attribute(element: &JSXElement<'_>, name: &str) -> Result<bool, CompileError> {
    let Some(attribute) = attribute(element, name) else {
        return Ok(false);
    };
    match &attribute.value {
        None => Ok(true),
        Some(JSXAttributeValue::ExpressionContainer(container)) => {
            let JSXExpression::BooleanLiteral(value) = &container.expression else {
                return Err(CompileError::new(
                    format!("{name} must be a boolean literal"),
                    container.span,
                ));
            };
            Ok(value.value)
        }
        _ => Err(CompileError::new(
            format!("{name} must be a boolean literal"),
            attribute.span,
        )),
    }
}

fn button_underline(
    element: &JSXElement<'_>,
    states: &Bindings,
) -> Result<BooleanValue, CompileError> {
    let Some(attribute) = attribute(element, "underline") else {
        return Ok(BooleanValue::Literal(false));
    };
    match &attribute.value {
        None => Ok(BooleanValue::Literal(true)),
        Some(JSXAttributeValue::ExpressionContainer(container)) => {
            if let JSXExpression::BooleanLiteral(value) = &container.expression {
                return Ok(BooleanValue::Literal(value.value));
            }
            let Some(expression) = container.expression.as_expression() else {
                return Err(CompileError::new(
                    "underline needs a boolean value",
                    container.span,
                ));
            };
            condition(expression, states).map(BooleanValue::Dynamic)
        }
        _ => Err(CompileError::new(
            "underline needs a boolean value",
            attribute.span,
        )),
    }
}

fn text_alignment(element: &JSXElement<'_>) -> Result<TextAlignment, CompileError> {
    match optional_string_attribute(element, "align")?.as_deref() {
        None | Some("start") => Ok(TextAlignment::Start),
        Some("center") => Ok(TextAlignment::Center),
        Some("end") => Ok(TextAlignment::End),
        Some("justify") => Ok(TextAlignment::Justify),
        Some(_) => invalid_value(element, "align", "start, center, end or justify"),
    }
}

fn tone(element: &JSXElement<'_>) -> Result<Tone, CompileError> {
    match optional_string_attribute(element, "tone")?.as_deref() {
        None | Some("primary") => Ok(Tone::Primary),
        Some("muted") => Ok(Tone::Muted),
        Some(_) => invalid_value(element, "tone", "primary or muted"),
    }
}

fn invalid_value<T>(
    element: &JSXElement<'_>,
    name: &str,
    expected: &str,
) -> Result<T, CompileError> {
    let span =
        attribute(element, name).map_or(element.opening_element.span, |attribute| attribute.span);
    Err(CompileError::new(
        format!("{name} must be {expected}"),
        span,
    ))
}

fn lower_text_parts(
    element: &JSXElement<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
    parent: &str,
) -> Result<Vec<TextPart>, CompileError> {
    let mut parts = Vec::new();
    for (index, child) in element.children.iter().enumerate() {
        match child {
            JSXChild::Text(text) => {
                let value = normalise_inline_text(
                    text.value.as_str(),
                    index
                        .checked_sub(1)
                        .and_then(|index| element.children.get(index))
                        .is_some_and(|child| matches!(child, JSXChild::ExpressionContainer(_))),
                    element
                        .children
                        .get(index + 1)
                        .is_some_and(|child| matches!(child, JSXChild::ExpressionContainer(_))),
                );
                if !value.is_empty() {
                    parts.push(TextPart::Literal(value));
                }
            }
            JSXChild::ExpressionContainer(container) => {
                let Some(expression) = container.expression.as_expression() else {
                    return Err(CompileError::new(
                        format!("{parent} expressions must reference state or a list item"),
                        container.span,
                    ));
                };
                parts.push(expression_text_part(expression, states, item)?);
            }
            _ => {
                return Err(CompileError::new(
                    format!("{parent} supports text and state or list-item values"),
                    child.span(),
                ));
            }
        }
    }
    if parts.is_empty() {
        return Err(CompileError::new("text cannot be empty", element.span));
    }
    Ok(parts)
}

fn expression_text_part(
    expression: &Expression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<TextPart, CompileError> {
    if let Some((value, kind)) = expression_derived_value(expression, states)? {
        if !scalar_kind(&kind) {
            return Err(CompileError::new(
                "Text can only display scalar derived values",
                expression.span(),
            ));
        }
        return Ok(TextPart::Value(value));
    }
    if let Ok(state) = list_length_state(expression, states) {
        return Ok(TextPart::ListLength(state));
    }
    if let Some(binding) = expression_controller_value(expression, states)? {
        if !scalar_kind(&binding.kind) {
            return Err(CompileError::new(
                "Text can only display scalar controller fields",
                expression.span(),
            ));
        }
        return Ok(TextPart::Controller(binding.controller, binding.path));
    }
    if let Some(binding) = expression_resource_value(expression, states)? {
        if !scalar_kind(&binding.kind) {
            return Err(CompileError::new(
                "Text can only display scalar resource fields",
                expression.span(),
            ));
        }
        return Ok(TextPart::Resource(binding.resource, binding.field));
    }
    if let Some(item) = item
        && let Some(path) = item_path(expression, item.name)
    {
        let Some(kind) = kind_at_path(item.kind, &path) else {
            return Err(CompileError::new(
                "unknown list-item field",
                expression.span(),
            ));
        };
        if !scalar_kind(kind) {
            return Err(CompileError::new(
                "Text can only display scalar list-item fields",
                expression.span(),
            ));
        }
        return Ok(TextPart::Item(path));
    }

    let binding = expression_state_value(expression, states)?;
    if !scalar_kind(&binding.kind) {
        return Err(CompileError::new(
            "Text can only display number, boolean or string state",
            expression.span(),
        ));
    }
    Ok(TextPart::State(binding.id))
}

fn expression_controller_value(
    expression: &Expression<'_>,
    states: &Bindings,
) -> Result<Option<ControllerValueBinding>, CompileError> {
    let Some((name, path)) = member_path(expression) else {
        return Ok(None);
    };
    let Some(controller) = states.controllers.get(name) else {
        return Ok(None);
    };
    if path.is_empty() {
        return Err(CompileError::new(
            "use a field from the controller",
            expression.span(),
        ));
    }
    let kind = kind_at_path(&controller.shape, &path)
        .ok_or_else(|| CompileError::new("unknown controller field", expression.span()))?;
    Ok(Some(ControllerValueBinding {
        controller: controller.id,
        path,
        kind: kind.clone(),
        controller_kind: controller.kind,
    }))
}

fn expression_resource_value(
    expression: &Expression<'_>,
    states: &Bindings,
) -> Result<Option<ResourceValueBinding>, CompileError> {
    let Some((name, path)) = member_path(expression) else {
        return Ok(None);
    };
    if let Some(combined) = states.combined.get(name) {
        let [value, resource_name, rest @ ..] = path.as_slice() else {
            return Err(CompileError::new(
                "combined results are read from .value.<name>",
                expression.span(),
            ));
        };
        if value != "value" {
            return Err(CompileError::new(
                "combined results are read from .value.<name>",
                expression.span(),
            ));
        }
        let resource = combined
            .resources
            .iter()
            .find(|(name, _)| name == resource_name)
            .map(|(_, resource)| resource)
            .ok_or_else(|| CompileError::new("unknown combined resource", expression.span()))?;
        let kind = kind_at_path(&resource.shape, rest).ok_or_else(|| {
            CompileError::new("unknown combined resource value field", expression.span())
        })?;
        return Ok(Some(ResourceValueBinding {
            resource: resource.id,
            field: ResourceField::Value(rest.to_vec()),
            kind: kind.clone(),
        }));
    }
    let Some(resource) = states.resources.get(name) else {
        return Ok(None);
    };
    let (field, kind) = match (resource.protocol, path.as_slice()) {
        (ResourceProtocol::Background | ResourceProtocol::Cached, [field]) if field == "status" => {
            (ResourceField::Status, StateShape::String)
        }
        (ResourceProtocol::Background | ResourceProtocol::Cached, [field])
            if field == "updatedAtMs" =>
        {
            (ResourceField::UpdatedAtMs, StateShape::Number)
        }
        (ResourceProtocol::Background | ResourceProtocol::Cached, [field, property])
            if field == "error" && property == "kind" =>
        {
            (ResourceField::ErrorKind, StateShape::String)
        }
        (ResourceProtocol::Background | ResourceProtocol::Cached, [field, property])
            if field == "error" && property == "message" =>
        {
            (ResourceField::ErrorMessage, StateShape::String)
        }
        (ResourceProtocol::Background | ResourceProtocol::Cached, [field, property])
            if field == "error" && property == "retryable" =>
        {
            (ResourceField::ErrorRetryable, StateShape::Bool)
        }
        (ResourceProtocol::Background | ResourceProtocol::Cached, [field, property])
            if field == "error" && property == "attemptedAtMs" =>
        {
            (ResourceField::ErrorAttemptedAtMs, StateShape::Number)
        }
        (ResourceProtocol::Background | ResourceProtocol::Cached, [field, path @ ..])
            if field == "value" =>
        {
            let kind = kind_at_path(&resource.shape, path).ok_or_else(|| {
                CompileError::new("unknown background value field", expression.span())
            })?;
            (ResourceField::Value(path.to_vec()), kind.clone())
        }
        (ResourceProtocol::Background | ResourceProtocol::Cached, _) => {
            return Err(CompileError::new(
                "background resource fields are status, value, updatedAtMs or error details",
                expression.span(),
            ));
        }
        (ResourceProtocol::Async | ResourceProtocol::Mutation, [field]) if field == "status" => {
            (ResourceField::Status, StateShape::String)
        }
        (ResourceProtocol::Async | ResourceProtocol::Mutation, [field, property])
            if field == "error" && property == "kind" =>
        {
            (ResourceField::ErrorKind, StateShape::String)
        }
        (ResourceProtocol::Async | ResourceProtocol::Mutation, [field, property])
            if field == "error" && property == "message" =>
        {
            (ResourceField::ErrorMessage, StateShape::String)
        }
        (ResourceProtocol::Async | ResourceProtocol::Mutation, [field, property])
            if field == "error" && property == "retryable" =>
        {
            (ResourceField::ErrorRetryable, StateShape::Bool)
        }
        (ResourceProtocol::Async | ResourceProtocol::Mutation, [field, path @ ..])
            if field == "value" =>
        {
            let kind = kind_at_path(&resource.shape, path).ok_or_else(|| {
                CompileError::new("unknown resource value field", expression.span())
            })?;
            (ResourceField::Value(path.to_vec()), kind.clone())
        }
        (ResourceProtocol::Async | ResourceProtocol::Mutation, _) => {
            return Err(CompileError::new(
                "resource fields are status, value, or error details",
                expression.span(),
            ));
        }
    };
    Ok(Some(ResourceValueBinding {
        resource: resource.id,
        field,
        kind,
    }))
}

fn member_path<'a>(expression: &'a Expression<'a>) -> Option<(&'a str, Vec<String>)> {
    match unparenthesised(expression) {
        Expression::Identifier(identifier) => Some((identifier.name.as_str(), Vec::new())),
        Expression::StaticMemberExpression(member) => {
            let (name, mut path) = member_path(&member.object)?;
            path.push(member.property.name.to_string());
            Some((name, path))
        }
        _ => None,
    }
}

fn validate_resource_comparison(
    field: &ResourceField,
    value: &StateValue,
    span: Span,
) -> Result<(), CompileError> {
    let StateValue::String(value) = value else {
        return Ok(());
    };
    let allowed = match field {
        ResourceField::Status => Some(
            &[
                "idle", "loading", "running", "waiting", "ready", "stale", "error",
            ][..],
        ),
        ResourceField::ErrorKind => Some(
            &[
                "unavailable",
                "permission-denied",
                "permission-blocked",
                "location-disabled",
                "nfc-disabled",
                "timeout",
                "protocol",
                "http",
                "invalid-data",
                "storage",
                "scheduler",
                "unexpected",
            ][..],
        ),
        _ => None,
    };
    if allowed.is_some_and(|allowed| !allowed.contains(&value.as_str())) {
        return Err(CompileError::new(
            "unknown resource status or error kind",
            span,
        ));
    }
    Ok(())
}

fn item_path(expression: &Expression<'_>, name: &str) -> Option<Vec<String>> {
    match unparenthesised(expression) {
        Expression::Identifier(identifier) if identifier.name == name => Some(Vec::new()),
        Expression::StaticMemberExpression(member) => {
            let mut path = item_path(&member.object, name)?;
            path.push(member.property.name.to_string());
            Some(path)
        }
        _ => None,
    }
}

fn kind_at_path<'a>(mut kind: &'a StateShape, path: &[String]) -> Option<&'a StateShape> {
    for field in path {
        while let StateShape::Optional(inner) = kind {
            kind = inner;
        }
        let StateShape::Object(fields) = kind else {
            return None;
        };
        kind = fields.get(field)?;
    }
    Some(kind)
}

fn scalar_kind(kind: &StateShape) -> bool {
    match kind {
        StateShape::Null
        | StateShape::Number
        | StateShape::Bool
        | StateShape::String
        | StateShape::Literal(_) => true,
        StateShape::Optional(kind) => scalar_kind(kind),
        StateShape::Union(kinds) => kinds.iter().all(scalar_kind),
        StateShape::List(_) | StateShape::Object(_) => false,
    }
}

fn reject_other_attributes(element: &JSXElement<'_>, allowed: &[&str]) -> Result<(), CompileError> {
    for item in &element.opening_element.attributes {
        let JSXAttributeItem::Attribute(attribute) = item else {
            return Err(CompileError::new(
                "spread props are not supported",
                item.span(),
            ));
        };
        let name = attribute_name(attribute)?;
        if !allowed.contains(&name) {
            return Err(CompileError::new(
                format!(
                    "{} does not accept {name}",
                    element_name(&element.opening_element.name)?
                ),
                attribute.span,
            ));
        }
    }
    Ok(())
}

fn attribute<'a>(element: &'a JSXElement<'a>, name: &str) -> Option<&'a JSXAttribute<'a>> {
    element.opening_element.attributes.iter().find_map(|item| {
        let JSXAttributeItem::Attribute(attribute) = item else {
            return None;
        };
        (attribute_name(attribute).ok() == Some(name)).then_some(attribute.as_ref())
    })
}

fn attribute_name<'a>(attribute: &'a JSXAttribute<'a>) -> Result<&'a str, CompileError> {
    let JSXAttributeName::Identifier(name) = &attribute.name else {
        return Err(CompileError::new(
            "namespaced props are not supported",
            attribute.name.span(),
        ));
    };
    Ok(name.name.as_str())
}

fn expect_element(element: &JSXElement<'_>, expected: &str) -> Result<(), CompileError> {
    let actual = element_name(&element.opening_element.name)?;
    if actual == expected {
        Ok(())
    } else {
        Err(CompileError::new(
            format!("expected <{expected}>, found <{actual}>"),
            element.opening_element.name.span(),
        ))
    }
}

fn element_name<'a>(name: &'a JSXElementName<'a>) -> Result<&'a str, CompileError> {
    match name {
        JSXElementName::Identifier(name) => Ok(name.name.as_str()),
        JSXElementName::IdentifierReference(name) => Ok(name.name.as_str()),
        _ => Err(CompileError::new(
            "namespaced Ink elements are not supported",
            name.span(),
        )),
    }
}

fn element_children<'a>(
    element: &'a JSXElement<'a>,
) -> Result<Vec<&'a JSXChild<'a>>, CompileError> {
    Ok(element
        .children
        .iter()
        .filter(|child| match child {
            JSXChild::Text(text) => !text.value.as_str().trim().is_empty(),
            _ => true,
        })
        .collect())
}

fn require_import(imports: &Imports, name: &str, span: Span) -> Result<(), CompileError> {
    if imports.ink.contains(name) {
        Ok(())
    } else {
        Err(CompileError::new(
            format!("{name} must be imported from \"ink\""),
            span,
        ))
    }
}

fn integer(value: f64, span: Span, label: &str) -> Result<i64, CompileError> {
    if value.is_finite()
        && value.fract() == 0.0
        && value >= i64::MIN as f64
        && value <= i64::MAX as f64
    {
        Ok(value as i64)
    } else {
        Err(CompileError::new(
            format!("{label} must be an integer"),
            span,
        ))
    }
}

fn normalise_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalise_inline_text(value: &str, after_expression: bool, before_expression: bool) -> String {
    let mut text = normalise_text(value);
    if text.is_empty() {
        return text;
    }
    if after_expression && value.chars().next().is_some_and(char::is_whitespace) {
        text.insert(0, ' ');
    }
    if before_expression && value.chars().next_back().is_some_and(char::is_whitespace) {
        text.push(' ');
    }
    text
}

fn unparenthesised<'a>(mut expression: &'a Expression<'a>) -> &'a Expression<'a> {
    while let Expression::ParenthesizedExpression(parenthesised) = expression {
        expression = &parenthesised.expression;
    }
    expression
}
