use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use oxc::{
    ast::ast::{
        Argument, ArrowFunctionBody, BindingPattern, ExportDefaultDeclarationKind, Expression,
        Function, ImportDeclarationSpecifier, JSXAttribute, JSXAttributeItem, JSXAttributeName,
        JSXAttributeValue, JSXChild, JSXElement, JSXElementName, JSXExpression, ObjectPropertyKind,
        PropertyKey, PropertyKind, Statement, TSSignature, TSType, VariableDeclarationKind,
    },
    span::{GetSpan, Span},
    syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator},
};

use crate::{
    diagnostic::CompileError,
    ir::{
        Action, Alignment, AndroidPermission, App, Axis, Collection, Condition, Controller,
        ControllerId, Extension, ImageFit, ImageSource, Justification, NativeOperation, Node,
        PayloadPart, Resource, ResourceField, ResourceId, Route, SourceSpan, State, StateId,
        StateLifetime, StateShape, StateValue, Tab, TextAlignment, TextPart, Tone, Value,
    },
    resolver::ModuleResolver,
};

const INK_IMPORTS: [&str; 17] = [
    "Button",
    "Icon",
    "Image",
    "Navigator",
    "Route",
    "Screen",
    "SelectorButton",
    "Stack",
    "Tab",
    "Tabs",
    "Text",
    "TextInput",
    "Toggle",
    "back",
    "persistedState",
    "sharedState",
    "state",
];

#[derive(Clone, PartialEq, Eq)]
struct StateBinding {
    id: StateId,
    kind: StateShape,
}

struct ResourceBinding {
    id: ResourceId,
    shape: StateShape,
    request: Option<NativeOperation>,
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

struct ControllerBinding {
    id: ControllerId,
    kind: AudioControllerKind,
    shape: StateShape,
}

struct ControllerValueBinding {
    controller: ControllerId,
    path: Vec<String>,
    kind: StateShape,
}

struct ControllerInitialiser {
    definition: Controller,
    kind: AudioControllerKind,
    initial: StateValue,
    shape: StateShape,
}

#[derive(Default)]
struct Bindings {
    states: HashMap<String, StateBinding>,
    resources: HashMap<String, ResourceBinding>,
    controllers: HashMap<String, ControllerBinding>,
    source_path: PathBuf,
}

impl Bindings {
    fn get(&self, name: &str) -> Option<&StateBinding> {
        self.states.get(name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExtensionFunction {
    LightSdkVersion,
    LightSdkPermission,
    Json,
    MicrophonePermission,
    LevelMeter,
    PitchDetector,
    AudioPlayer,
    AudioRecorder,
    LocationPermission,
    CurrentLocation,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AudioControllerKind {
    Level,
    Pitch,
    Player,
    Recorder,
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
                if ink.contains(local) || screens.contains_key(local) {
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
                    let function = match (extension, imported.as_str()) {
                        (Extension::LightSdk, "lightSdkVersion") => {
                            ExtensionFunction::LightSdkVersion
                        }
                        (Extension::LightSdk, "lightSdkPermission") => {
                            ExtensionFunction::LightSdkPermission
                        }
                        (Extension::Network, "json") => ExtensionFunction::Json,
                        (Extension::Audio, "microphonePermission") => {
                            ExtensionFunction::MicrophonePermission
                        }
                        (Extension::Audio, "levelMeter") => ExtensionFunction::LevelMeter,
                        (Extension::Audio, "pitchDetector") => ExtensionFunction::PitchDetector,
                        (Extension::Audio, "audioPlayer") => ExtensionFunction::AudioPlayer,
                        (Extension::Audio, "audioRecorder") => ExtensionFunction::AudioRecorder,
                        (Extension::Location, "locationPermission") => {
                            ExtensionFunction::LocationPermission
                        }
                        (Extension::Location, "currentLocation") => {
                            ExtensionFunction::CurrentLocation
                        }
                        _ => {
                            return Err(CompileError::new(
                                format!("{imported} is not exported by this Ink extension"),
                                specifier.span,
                            ));
                        }
                    };
                    let local = specifier.local.name.to_string();
                    if ink.contains(&local)
                        || screens.contains_key(&local)
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
            if screens.contains_key(local_name) || !ink.insert(imported_name.to_owned()) {
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
                    if let Some(mut controller) =
                        controller_initialiser(declarator.init.as_ref(), imports)?
                    {
                        let state = StateId(states.len());
                        let id = ControllerId(controllers.len());
                        controller.definition.state = state;
                        if matches!(
                            controller.kind,
                            AudioControllerKind::Level
                                | AudioControllerKind::Pitch
                                | AudioControllerKind::Recorder
                        ) {
                            android_permissions.insert(AndroidPermission::Microphone);
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
    let scoped_resources = (0..resources.len()).map(ResourceId).collect::<Vec<_>>();
    let scoped_controllers = (0..controllers.len()).map(ControllerId).collect::<Vec<_>>();
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
        ModuleKind::App => scoped_controllers,
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
            Vec::new()
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
        ExtensionFunction::LevelMeter | ExtensionFunction::PitchDetector
    ) {
        return Ok(None);
    }
    if function != ExtensionFunction::Json && call.type_arguments.is_some() {
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
        ExtensionFunction::Json => network_json_resource(call, imports, states)?,
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
        ExtensionFunction::LevelMeter
        | ExtensionFunction::PitchDetector
        | ExtensionFunction::AudioPlayer
        | ExtensionFunction::AudioRecorder => unreachable!(),
    };
    Ok(Some(resource))
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
        },
        request: None,
        android_permission: Some(AndroidPermission::Location),
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
    ) {
        return Ok(None);
    }
    if call.type_arguments.is_some() {
        return Err(CompileError::new(
            "audio controllers do not take type arguments",
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
                ("status", StateShape::String),
                ("rms", StateShape::Number),
                ("peak", StateShape::Number),
                ("error", StateShape::String),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("rms", StateValue::Number(0.0)),
                ("peak", StateValue::Number(0.0)),
                ("error", StateValue::String(String::new())),
            ]);
            (AudioControllerKind::Level, "{}".to_owned(), shape, initial)
        }
        ExtensionFunction::PitchDetector => {
            let reference_hz = pitch_reference(call)?;
            let shape = object_shape([
                ("status", StateShape::String),
                ("frequencyHz", StateShape::Number),
                ("note", StateShape::String),
                ("octave", StateShape::Number),
                ("cents", StateShape::Number),
                ("confidence", StateShape::Number),
                ("error", StateShape::String),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("frequencyHz", StateValue::Number(0.0)),
                ("note", StateValue::String(String::new())),
                ("octave", StateValue::Number(0.0)),
                ("cents", StateValue::Number(0.0)),
                ("confidence", StateValue::Number(0.0)),
                ("error", StateValue::String(String::new())),
            ]);
            (
                AudioControllerKind::Pitch,
                format!("{{\"referenceHz\":{reference_hz}}}"),
                shape,
                initial,
            )
        }
        ExtensionFunction::AudioPlayer => {
            let (usage, playback) = player_options(call)?;
            let shape = object_shape([
                ("status", StateShape::String),
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
                ("errorKind", StateShape::String),
                ("errorMessage", StateShape::String),
                ("errorRetryable", StateShape::Bool),
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
                ("errorKind", StateValue::String(String::new())),
                ("errorMessage", StateValue::String(String::new())),
                ("errorRetryable", StateValue::Bool(false)),
            ]);
            (
                AudioControllerKind::Player,
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
                ("status", StateShape::String),
                ("durationMs", StateShape::Number),
                ("id", StateShape::String),
                ("src", StateShape::String),
                ("recordingDurationMs", StateShape::Number),
                ("errorKind", StateShape::String),
                ("errorMessage", StateShape::String),
                ("errorRetryable", StateShape::Bool),
            ]);
            let initial = object_value([
                ("status", StateValue::String("idle".to_owned())),
                ("durationMs", StateValue::Number(0.0)),
                ("id", StateValue::String(String::new())),
                ("src", StateValue::String(String::new())),
                ("recordingDurationMs", StateValue::Number(0.0)),
                ("errorKind", StateValue::String(String::new())),
                ("errorMessage", StateValue::String(String::new())),
                ("errorRetryable", StateValue::Bool(false)),
            ]);
            (
                AudioControllerKind::Recorder,
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
            module: "audio".to_owned(),
            kind: match kind {
                AudioControllerKind::Level => "level",
                AudioControllerKind::Pitch => "pitch",
                AudioControllerKind::Player => "player",
                AudioControllerKind::Recorder => "recorder",
            }
            .to_owned(),
            config,
        },
        kind,
        initial,
        shape,
    }))
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

fn object_value<const N: usize>(fields: [(&str, StateValue); N]) -> StateValue {
    StateValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn network_json_resource(
    call: &oxc::ast::ast::CallExpression<'_>,
    imports: &Imports,
    states: &Bindings,
) -> Result<ResourceInitialiser, CompileError> {
    let Some(arguments) = &call.type_arguments else {
        return Err(CompileError::new(
            "json<T>() needs the response data type",
            call.span,
        ));
    };
    let [response_type] = arguments.params.as_slice() else {
        return Err(CompileError::new(
            "json<T>() accepts one response data type",
            arguments.span,
        ));
    };
    let shape = match response_type {
        TSType::TSTypeReference(reference) if reference.type_arguments.is_none() => {
            let oxc::ast::ast::TSTypeName::IdentifierReference(name) = &reference.type_name else {
                return Err(CompileError::new(
                    "network response types use a local type alias or an inline data type",
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
                })?
        }
        response_type => state_kind_from_type(response_type)?,
    };
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
                            format!("unknown network option {name:?}"),
                            property.key.span(),
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(CompileError::new(
                "json<T>() accepts a URL and optional options object",
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
    payload.push(PayloadPart::Literal("}".to_owned()));
    Ok(ResourceInitialiser {
        definition: Resource {
            module: "network".to_owned(),
            operation: "json".to_owned(),
            payload,
            shape,
            timeout_ms,
        },
        request: None,
        android_permission: None,
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
        (Some(declared), Some(inferred)) if declared != inferred => {
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
            "state values must be literal numbers, booleans, strings, objects or lists",
            value.span(),
        )),
    }
}

fn state_kind(value: &StateValue) -> Option<StateShape> {
    match value {
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
        TSType::TSNumberKeyword(_) => Ok(StateShape::Number),
        TSType::TSBooleanKeyword(_) => Ok(StateShape::Bool),
        TSType::TSStringKeyword(_) => Ok(StateShape::String),
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
                if property.computed || property.optional {
                    return Err(CompileError::new(
                        "state object properties must be required names",
                        property.span,
                    ));
                }
                let name = property_name(&property.key)?;
                let annotation = property.type_annotation.as_ref().ok_or_else(|| {
                    CompileError::new("state properties need a type", property.span)
                })?;
                if fields
                    .insert(
                        name.clone(),
                        state_kind_from_type(&annotation.type_annotation)?,
                    )
                    .is_some()
                {
                    return Err(CompileError::new(
                        format!("state object property {name:?} is declared twice"),
                        property.key.span(),
                    ));
                }
            }
            Ok(StateShape::Object(fields))
        }
        _ => Err(CompileError::new(
            "state types use number, boolean, string, arrays and object literals",
            kind.span(),
        )),
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
    require_import(imports, name, element.opening_element.name.span())?;
    match name {
        "Screen" => lower_screen(element, states, imports, item),
        "Stack" => lower_stack(element, states, imports, item),
        "Text" => lower_text(element, states, item),
        "TextInput" => lower_text_input(element, states),
        "Button" => lower_button(element, states, imports, item),
        "SelectorButton" => lower_selector_button(element, states, imports, item),
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
    reject_other_attributes(element, &["size", "align"])?;
    Ok(Node::Text {
        parts: lower_text_parts(element, states, item, "Text")?,
        font_size,
        align,
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

fn lower_selector_button(
    element: &JSXElement<'_>,
    states: &Bindings,
    imports: &Imports,
    item: Option<ItemBinding<'_>>,
) -> Result<Node, CompileError> {
    let label = required_string_attribute(element, "label")?;
    let action = press_action(element, "SelectorButton", states, imports, item)?;
    reject_other_attributes(element, &["label", "onPress", "href"])?;
    Ok(Node::SelectorButton {
        label,
        value: lower_text_parts(element, states, item, "SelectorButton")?,
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
    let href = optional_string_attribute(element, "href")?;
    if href.is_some() && attribute(element, "onPress").is_some() {
        return Err(CompileError::new(
            format!("{component} accepts either href or onPress, not both"),
            element.opening_element.span,
        ));
    }
    match href {
        Some(path) => {
            let span = attribute(element, "href")
                .map_or(element.opening_element.span, |attribute| attribute.span);
            validate_route_path(&path, span)?;
            Ok(Some(Action::Navigate {
                path,
                source: SourceSpan {
                    path: imports.source_path.clone(),
                    span,
                },
            }))
        }
        None => optional_button_action_attribute(element, states, imports, item),
    }
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
            ImageSource::Remote(vec![image_source_part(expression, states, item)?])
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
    let state = state_attribute(element, "value", states, StateShape::Number)?;
    reject_other_attributes(element, &["value"])?;
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
        state: state.id,
        tabs,
    })
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
    let action = action_attribute(element, "onPress", states, None)?;
    reject_other_attributes(element, &["icon", "onPress"])?;
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
        action,
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
                .map(|route| route.path.as_str())
                .collect::<HashSet<_>>(),
        ),
        _ => None,
    };
    validate_navigation_node(root, routes.as_ref())
}

fn validate_navigation_node(
    node: &Node,
    routes: Option<&HashSet<&str>>,
) -> Result<(), NavigationError> {
    match node {
        Node::Screen { children, .. } | Node::Stack { children, .. } => {
            for child in children {
                validate_navigation_node(child, routes)?;
            }
        }
        Node::Button {
            action: Some(Action::Navigate { path, source }),
            ..
        }
        | Node::SelectorButton {
            action: Some(Action::Navigate { path, source }),
            ..
        } => {
            let Some(routes) = routes else {
                return Err(NavigationError {
                    source: source.path.clone(),
                    error: CompileError::new("href requires a Navigator", source.span),
                });
            };
            if !routes.contains(path.as_str()) {
                return Err(NavigationError {
                    source: source.path.clone(),
                    error: CompileError::new(format!("no route matches {path:?}"), source.span),
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
        | Node::SelectorButton { .. }
        | Node::Icon { .. }
        | Node::Image { .. }
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
    element_children(element)?
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
        .collect()
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
        Expression::CallExpression(expression) => {
            lower_collection(expression, states, imports, item)
        }
        _ => Err(CompileError::new(
            "dynamic children use state.value && <Element>, a conditional, or state.value.map(...)",
            expression.span(),
        )),
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
                if let Some(binding) = expression_controller_value(&expression.left, states)? {
                    if !scalar_kind(&binding.kind) {
                        return Err(CompileError::new(
                            "controller comparisons support numbers, booleans and strings",
                            expression.span,
                        ));
                    }
                    let value = literal_state_value(unparenthesised(&expression.right))?;
                    if state_kind(&value).as_ref() != Some(&binding.kind) {
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
                    if state_kind(&value).as_ref() != Some(&binding.kind) {
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
                if state_kind(&value).as_ref() != Some(&binding.kind) {
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

fn invert_condition(condition: Condition) -> Condition {
    match condition {
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
        if let Some(resource) = expression_resource_value(&map.object, states)? {
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
                "audio controller actions do not take type arguments",
                call.span,
            ));
        }
        let payload = lower_audio_controller_action(binding.kind, method, call, states, item)?;
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

fn lower_audio_controller_action(
    kind: AudioControllerKind,
    method: &str,
    call: &oxc::ast::ast::CallExpression<'_>,
    states: &Bindings,
    item: Option<ItemBinding<'_>>,
) -> Result<Vec<PayloadPart>, CompileError> {
    match kind {
        AudioControllerKind::Level | AudioControllerKind::Pitch => {
            if !matches!(method, "start" | "stop") || !call.arguments.is_empty() {
                return Err(CompileError::new(
                    "level and pitch controllers support start() and stop()",
                    call.span,
                ));
            }
            Ok(vec![PayloadPart::Literal(String::new())])
        }
        AudioControllerKind::Recorder => {
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
        AudioControllerKind::Player => player_action_payload(method, call, states, item),
    }
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
        Argument::NumericLiteral(value) if binding.kind == StateShape::Number => {
            Ok(Action::SetNumber {
                state: binding.id,
                value: value.value,
            })
        }
        Argument::BooleanLiteral(value) if binding.kind == StateShape::Bool => {
            Ok(Action::SetBool {
                state: binding.id,
                value: value.value,
            })
        }
        Argument::StringLiteral(value) if binding.kind == StateShape::String => {
            Ok(Action::SetString {
                state: binding.id,
                value: value.value.to_string(),
            })
        }
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
        value => Err(CompileError::new(
            "state.set value does not match the state's type",
            value.span(),
        )),
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
        (Expression::NumericLiteral(value), StateShape::Number) => Ok(Value::Number(value.value)),
        (Expression::BooleanLiteral(value), StateShape::Bool) => Ok(Value::Bool(value.value)),
        (Expression::StringLiteral(value), StateShape::String) => {
            Ok(Value::String(value.value.to_string()))
        }
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
            if seen.len() != fields.len() {
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
        Some(_) => invalid_value(element, "align", "start, center or end"),
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
            "use a field from the audio controller",
            expression.span(),
        ));
    }
    let kind = kind_at_path(&controller.shape, &path)
        .ok_or_else(|| CompileError::new("unknown audio controller field", expression.span()))?;
    Ok(Some(ControllerValueBinding {
        controller: controller.id,
        path,
        kind: kind.clone(),
    }))
}

fn expression_resource_value(
    expression: &Expression<'_>,
    states: &Bindings,
) -> Result<Option<ResourceValueBinding>, CompileError> {
    let Some((name, path)) = member_path(expression) else {
        return Ok(None);
    };
    let Some(resource) = states.resources.get(name) else {
        return Ok(None);
    };
    let (field, kind) = match path.as_slice() {
        [field] if field == "status" => (ResourceField::Status, StateShape::String),
        [field, property] if field == "error" && property == "kind" => {
            (ResourceField::ErrorKind, StateShape::String)
        }
        [field, property] if field == "error" && property == "message" => {
            (ResourceField::ErrorMessage, StateShape::String)
        }
        [field, property] if field == "error" && property == "retryable" => {
            (ResourceField::ErrorRetryable, StateShape::Bool)
        }
        [field, path @ ..] if field == "value" => {
            let kind = kind_at_path(&resource.shape, path).ok_or_else(|| {
                CompileError::new("unknown resource value field", expression.span())
            })?;
            (ResourceField::Value(path.to_vec()), kind.clone())
        }
        _ => {
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
        ResourceField::Status => Some(&["loading", "ready", "error"][..]),
        ResourceField::ErrorKind => Some(
            &[
                "unavailable",
                "permission-denied",
                "permission-blocked",
                "location-disabled",
                "timeout",
                "protocol",
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
        let StateShape::Object(fields) = kind else {
            return None;
        };
        kind = fields.get(field)?;
    }
    Some(kind)
}

const fn scalar_kind(kind: &StateShape) -> bool {
    matches!(
        kind,
        StateShape::Number | StateShape::Bool | StateShape::String
    )
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
