use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    icons,
    ir::{
        Action, Alignment, App, Axis, CameraPreviewKind, Collection, Condition, Controller,
        ImageFit, ImageSource, Justification, NativeOperation, Node, PayloadPart, Resource,
        ResourceField, ResourceProtocol, State, StateLifetime, StateShape, StateValue,
        TextAlignment, TextInputAction, TextPart, Tone, Value,
    },
};

const DEFAULT_ICON_SIZE: f32 = 28.0;
const BUTTON_ICON_SIZE: f32 = 30.0;
const HEADER_BACK_ICON_SIZE: f32 = 28.0;
const TAB_ICON_SIZE: f32 = 52.0;
const TOGGLE_ICON_SIZE: f32 = 9.8;

pub fn generate(app: &App, root: &Path) -> Result<String> {
    let states = app.states.iter().map(state_definition);
    let resources = app.resources.iter().map(resource_definition);
    let application_resources = app.application_resources.iter().map(|resource| {
        let id = resource.0;
        quote! { ResourceId::new(#id) }
    });
    let controllers = app.controllers.iter().map(controller_definition);
    let application_controllers = app.application_controllers.iter().map(|controller| {
        let id = controller.0;
        quote! { ControllerId::new(#id) }
    });
    let uses_persistence = app
        .states
        .iter()
        .any(|state| matches!(state.lifetime, StateLifetime::Persisted(_)));
    let mut emitter = Emitter::new(root);
    let root = emitter.node(&app.root)?;
    let declarations = emitter.declarations;
    let tokens = quote! {
        #[allow(unused_imports)]
        use ink_core::{
            Action, Alignment, AppDefinition, Axis, CameraPreviewKind, Condition,
            ControllerDefinition, ControllerId, Justification, Mask, Node, StateId, Collection,
            ImageSource, NativeOperation,
            PayloadPart, ResourceDefinition, ResourceField, ResourceId, StateDefinition,
            StateValue, Route, Tab, TextAlign, TextInputAction, TextPart, Tone, Value,
        };

        pub const USES_PERSISTENCE: bool = #uses_persistence;

        #[rustfmt::skip]
        pub fn app() -> ink_core::AppDefinition {
            #(#declarations)*
            AppDefinition::new(
                vec![#(#states),*],
                vec![#(#resources),*],
                vec![#(#application_resources),*],
                vec![#(#controllers),*],
                vec![#(#application_controllers),*],
                #root,
            )
        }
    };
    let syntax = syn::parse2::<syn::File>(tokens).context("Ink generated invalid Rust")?;
    Ok(prettyplease::unparse(&syntax))
}

struct Emitter<'a> {
    root: &'a Path,
    next_node: usize,
    next_mask: usize,
    next_asset: usize,
    declarations: Vec<TokenStream>,
    masks: HashMap<(String, u32), TokenStream>,
    assets: HashMap<PathBuf, TokenStream>,
}

impl<'a> Emitter<'a> {
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            next_node: 0,
            next_mask: 0,
            next_asset: 0,
            declarations: Vec::new(),
            masks: HashMap::new(),
            assets: HashMap::new(),
        }
    }

    fn node(&mut self, value: &Node) -> Result<TokenStream> {
        let value = match value {
            Node::Screen {
                children,
                title,
                centered,
                resources,
                controllers,
            } => {
                let children = self.children(children)?;
                let title = option_string(title.as_deref());
                let resources = resources.iter().map(|resource| {
                    let id = resource.0;
                    quote! { ResourceId::new(#id) }
                });
                let controllers = controllers.iter().map(|controller| {
                    let id = controller.0;
                    quote! { ControllerId::new(#id) }
                });
                quote! {
                    Node::screen(
                        vec![#(#children),*],
                        #title,
                        #centered,
                        vec![#(#resources),*],
                        vec![#(#controllers),*],
                    )
                }
            }
            Node::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            } => {
                let children = self.children(children)?;
                let axis = axis_tokens(*axis);
                let gap = option_f32(*gap);
                let align = alignment_tokens(*align);
                let justify = justification_tokens(*justify);
                quote! { Node::stack(vec![#(#children),*], #axis, #gap, #align, #justify) }
            }
            Node::Text {
                parts,
                font_size,
                align,
            } => {
                let parts = parts.iter().map(text_part);
                let font_size = option_f32(*font_size);
                let align = text_alignment_tokens(*align);
                quote! { Node::text(vec![#(#parts),*], #font_size, #align) }
            }
            Node::TextInput {
                placeholder,
                state,
                action,
            } => {
                let state = state.0;
                let action = match action {
                    TextInputAction::Return => quote! { TextInputAction::Return },
                    TextInputAction::Search => quote! { TextInputAction::Search },
                    TextInputAction::Done => quote! { TextInputAction::Done },
                };
                quote! { Node::text_input(#placeholder, StateId::new(#state), #action) }
            }
            Node::Button {
                label,
                icon,
                underline,
                action,
            } => {
                let label = label.iter().map(text_part);
                let icon = match icon {
                    Some(name) => {
                        let mask = self.mask(name, BUTTON_ICON_SIZE)?;
                        quote! { Some(#mask) }
                    }
                    None => quote! { None },
                };
                let action = match action {
                    Some(action) => {
                        let action = action_tokens(action);
                        quote! { Some(#action) }
                    }
                    None => quote! { None },
                };
                quote! { Node::button(vec![#(#label),*], #icon, #underline, #action) }
            }
            Node::SelectorButton {
                label,
                value,
                action,
            } => {
                let value = value.iter().map(text_part);
                let action = match action {
                    Some(action) => {
                        let action = action_tokens(action);
                        quote! { Some(#action) }
                    }
                    None => quote! { None },
                };
                quote! { Node::selector_button(#label, vec![#(#value),*], #action) }
            }
            Node::Icon { name, size, tone } => {
                let size = size.unwrap_or(DEFAULT_ICON_SIZE);
                let mask = self.mask(name, size)?;
                let tone = tone_tokens(*tone);
                quote! { Node::icon(#mask, #size, #tone) }
            }
            Node::Image {
                source,
                fallback,
                bleed,
                width,
                height,
                fit,
            } => {
                let source = match source {
                    ImageSource::Local(source) => {
                        let asset = self.asset(source)?;
                        quote! { ImageSource::Asset(#asset) }
                    }
                    ImageSource::Remote(parts) => {
                        let parts = parts.iter().map(text_part);
                        quote! { ImageSource::Remote(vec![#(#parts),*]) }
                    }
                    ImageSource::Camera(parts) => {
                        let parts = parts.iter().map(text_part);
                        quote! { ImageSource::Native("camera".to_owned(), vec![#(#parts),*]) }
                    }
                };
                let fallback = match fallback {
                    Some(source) => {
                        let asset = self.asset(source)?;
                        quote! { Some(#asset) }
                    }
                    None => quote! { None },
                };
                let fit = image_fit_tokens(*fit);
                quote! { Node::image(#source, #fallback, #bleed, #width, #height, #fit) }
            }
            Node::CameraPreview { controller, kind } => {
                let controller = controller.0;
                let kind = match kind {
                    CameraPreviewKind::Photo => quote! { CameraPreviewKind::Photo },
                    CameraPreviewKind::Scanner => quote! { CameraPreviewKind::Scanner },
                };
                quote! { Node::camera_preview(ControllerId::new(#controller), #kind) }
            }
            Node::Toggle {
                label,
                state,
                action,
            } => {
                let off = self.toggle_mask(false);
                let on = self.toggle_mask(true);
                let state = state.0;
                let action = action_tokens(action);
                quote! {
                    Node::toggle(
                        #label,
                        StateId::new(#state),
                        #action,
                        #off,
                        #on,
                    )
                }
            }
            Node::Tabs { state, tabs } => {
                let mut generated_tabs = Vec::with_capacity(tabs.len());
                for tab in tabs {
                    let icon = self.filled_mask(&tab.icon, TAB_ICON_SIZE)?;
                    let action = action_tokens(&tab.action);
                    let screen = self.node(&tab.screen)?;
                    generated_tabs.push(quote! { Tab::new(#icon, #action, #screen) });
                }
                let state = state.0;
                quote! { Node::tabs(StateId::new(#state), vec![#(#generated_tabs),*]) }
            }
            Node::Navigator { routes } => {
                let mut generated_routes = Vec::with_capacity(routes.len());
                for route in routes {
                    let path = &route.path;
                    let screen = self.node(&route.screen)?;
                    generated_routes.push(quote! { Route::new(#path, #screen) });
                }
                let back = self.mask("arrow_back_ios", HEADER_BACK_ICON_SIZE)?;
                quote! { Node::navigator(vec![#(#generated_routes),*], #back) }
            }
            Node::Conditional {
                condition,
                consequent,
                alternate,
            } => {
                let condition = condition_tokens(condition);
                let consequent = self.node(consequent)?;
                let alternate = match alternate {
                    Some(alternate) => {
                        let alternate = self.node(alternate)?;
                        quote! { Some(#alternate) }
                    }
                    None => quote! { None },
                };
                quote! {
                    Node::conditional(
                        #condition,
                        #consequent,
                        #alternate,
                    )
                }
            }
            Node::ForEach {
                collection,
                template,
            } => {
                let collection = match collection {
                    Collection::State(state) => {
                        let state = state.0;
                        quote! { Collection::State(StateId::new(#state)) }
                    }
                    Collection::Resource(resource, path) => {
                        let resource = resource.0;
                        let path = path.iter().map(|field| quote! { #field.to_owned() });
                        quote! { Collection::Resource(ResourceId::new(#resource), vec![#(#path),*]) }
                    }
                };
                let template = self.node(template)?;
                quote! { Node::for_each(#collection, #template) }
            }
            Node::ScreenModule { .. } => {
                unreachable!("screen modules are expanded before code generation")
            }
        };

        let name = format_ident!("node_{}", self.next_node);
        self.next_node += 1;
        self.declarations.push(quote! { let #name = #value; });
        Ok(quote! { #name })
    }

    fn children(&mut self, children: &[Node]) -> Result<Vec<TokenStream>> {
        children.iter().map(|child| self.node(child)).collect()
    }

    fn mask(&mut self, name: &str, size: f32) -> Result<TokenStream> {
        self.material_mask(name, size, false)
    }

    fn filled_mask(&mut self, name: &str, size: f32) -> Result<TokenStream> {
        self.material_mask(name, size, true)
    }

    fn material_mask(&mut self, name: &str, size: f32, filled: bool) -> Result<TokenStream> {
        let key = (
            if filled {
                format!("filled:{name}")
            } else {
                name.to_owned()
            },
            size.to_bits(),
        );
        if let Some(mask) = self.masks.get(&key) {
            return Ok(mask.clone());
        }

        let icon = if filled {
            icons::raster_filled(name, size)?
        } else {
            icons::raster(name, size)?
        };
        let id = icon.id;
        let width = icon.width;
        let height = icon.height;
        let pixels = icon.pixels;
        let name = format_ident!("mask_{}", self.next_mask);
        self.next_mask += 1;
        self.declarations.push(quote! {
            let #name = Mask::new(#id, #width, #height, &[#(#pixels),*]);
        });
        let mask = quote! { #name };
        self.masks.insert(key, mask.clone());
        Ok(mask)
    }

    fn toggle_mask(&mut self, filled: bool) -> TokenStream {
        let key = (
            if filled {
                "ink_toggle_on"
            } else {
                "ink_toggle_off"
            }
            .to_owned(),
            TOGGLE_ICON_SIZE.to_bits(),
        );
        if let Some(mask) = self.masks.get(&key) {
            return mask.clone();
        }

        let icon = icons::toggle_circle(filled);
        let id = icon.id;
        let width = icon.width;
        let height = icon.height;
        let pixels = icon.pixels;
        let name = format_ident!("mask_{}", self.next_mask);
        self.next_mask += 1;
        self.declarations.push(quote! {
            let #name = Mask::new(#id, #width, #height, &[#(#pixels),*]);
        });
        let mask = quote! { #name };
        self.masks.insert(key, mask.clone());
        mask
    }

    fn asset(&mut self, source: &str) -> Result<TokenStream> {
        let path = self
            .root
            .join(source)
            .canonicalize()
            .with_context(|| format!("could not find image {source:?}"))?;
        if !path.starts_with(self.root) {
            anyhow::bail!("image {source:?} must be inside the Ink application");
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("png") {
            anyhow::bail!("image {source:?} must be a PNG file");
        }
        if let Some(asset) = self.assets.get(&path) {
            return Ok(asset.clone());
        }

        let bytes = std::fs::read(&path)
            .with_context(|| format!("could not read image {}", path.display()))?;
        let pixels = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .with_context(|| format!("could not decode image {}", path.display()))?
            .into_rgba8();
        let width = pixels.width();
        let height = pixels.height();
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(pixels.as_raw(), 9);
        let id = hash_bytes(&bytes);
        let name = format_ident!("image_{}", self.next_asset);
        self.next_asset += 1;
        self.declarations.push(quote! {
            let #name = ink_core::ImageAsset::new(#id, #width, #height, &[#(#compressed),*]);
        });
        let asset = quote! { #name };
        self.assets.insert(path, asset.clone());
        Ok(asset)
    }
}

fn state_value(value: &StateValue) -> TokenStream {
    match value {
        StateValue::Number(value) => quote! { StateValue::Number(#value) },
        StateValue::Bool(value) => quote! { StateValue::Bool(#value) },
        StateValue::String(value) => quote! { StateValue::String(#value.to_owned()) },
        StateValue::List(values) => {
            let values = values.iter().map(state_value);
            quote! { StateValue::List(vec![#(#values),*]) }
        }
        StateValue::Object(fields) => {
            let fields = fields.iter().map(|(name, value)| {
                let value = state_value(value);
                quote! { (#name.to_owned(), #value) }
            });
            quote! { StateValue::Object(vec![#(#fields),*]) }
        }
    }
}

fn state_definition(state: &State) -> TokenStream {
    let initial = state_value(&state.initial);
    match &state.lifetime {
        StateLifetime::Local => quote! { StateDefinition::local(#initial) },
        StateLifetime::Shared(_) => quote! { StateDefinition::shared(#initial) },
        StateLifetime::Persisted(key) => {
            let shape = state_shape(&state.shape);
            let schema = hash_bytes(state_shape_name(&state.shape).as_bytes());
            quote! { StateDefinition::persisted(#initial, #key, #schema, #shape) }
        }
    }
}

fn resource_definition(resource: &Resource) -> TokenStream {
    let module = &resource.module;
    let operation = &resource.operation;
    let payload = resource.payload.iter().map(payload_part);
    let timeout_ms = resource.timeout_ms;
    let reload_on_resume = resource.reload_on_resume;
    let shape = state_shape(&resource.shape);
    let protocol = match resource.protocol {
        ResourceProtocol::Async => quote! { ink_core::ResourceProtocol::Async },
        ResourceProtocol::Background => quote! { ink_core::ResourceProtocol::Background },
    };
    quote! {
        ResourceDefinition::with_protocol(
            #shape,
            NativeOperation::templated(#module, #operation, vec![#(#payload),*], #timeout_ms),
            #reload_on_resume,
            #protocol,
        )
    }
}

fn controller_definition(controller: &Controller) -> TokenStream {
    let state = controller.state.0;
    let module = &controller.module;
    let kind = &controller.kind;
    let config = &controller.config;
    quote! {
        ControllerDefinition::new(StateId::new(#state), #module, #kind, #config)
    }
}

fn state_shape(shape: &StateShape) -> TokenStream {
    match shape {
        StateShape::Number => quote! { ink_core::StateShape::Number },
        StateShape::Bool => quote! { ink_core::StateShape::Bool },
        StateShape::String => quote! { ink_core::StateShape::String },
        StateShape::List(item) => {
            let item = state_shape(item);
            quote! { ink_core::StateShape::List(Box::new(#item)) }
        }
        StateShape::Object(fields) => {
            let fields = fields.iter().map(|(name, shape)| {
                let shape = state_shape(shape);
                quote! { (#name.to_owned(), #shape) }
            });
            quote! { ink_core::StateShape::Object(vec![#(#fields),*]) }
        }
    }
}

fn state_shape_name(shape: &StateShape) -> String {
    match shape {
        StateShape::Number => "number".to_owned(),
        StateShape::Bool => "bool".to_owned(),
        StateShape::String => "string".to_owned(),
        StateShape::List(item) => format!("list<{}>", state_shape_name(item)),
        StateShape::Object(fields) => {
            let fields = fields
                .iter()
                .map(|(name, shape)| format!("{name}:{}", state_shape_name(shape)))
                .collect::<Vec<_>>()
                .join(",");
            format!("object{{{fields}}}")
        }
    }
}

fn text_part(part: &TextPart) -> TokenStream {
    match part {
        TextPart::Literal(value) => quote! { TextPart::literal(#value) },
        TextPart::State(state) => {
            let id = state.0;
            quote! { TextPart::state(StateId::new(#id)) }
        }
        TextPart::Resource(resource, field) => {
            let id = resource.0;
            let field = resource_field_tokens(field);
            quote! { TextPart::resource(ResourceId::new(#id), #field) }
        }
        TextPart::Controller(controller, path) => {
            let id = controller.0;
            quote! {
                TextPart::controller(ControllerId::new(#id), vec![#(#path.to_owned()),*])
            }
        }
        TextPart::ListLength(state) => {
            let id = state.0;
            quote! { TextPart::list_length(StateId::new(#id)) }
        }
        TextPart::Item(path) => {
            quote! { TextPart::Item(vec![#(#path.to_owned()),*]) }
        }
    }
}

fn action_tokens(action: &Action) -> TokenStream {
    match action {
        Action::Increment { state, by } => {
            let id = state.0;
            quote! { Action::Increment { state: StateId::new(#id), by: #by } }
        }
        Action::SetNumber { state, value } => {
            let id = state.0;
            quote! { Action::SetNumber { state: StateId::new(#id), value: #value } }
        }
        Action::SetBool { state, value } => {
            let id = state.0;
            quote! { Action::SetBool { state: StateId::new(#id), value: #value } }
        }
        Action::SetString { state, value } => {
            let id = state.0;
            quote! { Action::SetString { state: StateId::new(#id), value: #value.to_owned() } }
        }
        Action::Toggle { state } => {
            let id = state.0;
            quote! { Action::Toggle { state: StateId::new(#id) } }
        }
        Action::SetList { state, value } => {
            let id = state.0;
            let value = value_tokens(value);
            quote! { Action::SetList { state: StateId::new(#id), value: #value } }
        }
        Action::AppendList { state, value } => {
            let id = state.0;
            let value = value_tokens(value);
            quote! { Action::AppendList { state: StateId::new(#id), value: #value } }
        }
        Action::RemoveListItem { state } => {
            let id = state.0;
            quote! { Action::RemoveCurrentListItem { state: StateId::new(#id) } }
        }
        Action::ReplaceListItem { state, value } => {
            let id = state.0;
            let value = value_tokens(value);
            quote! {
                Action::ReplaceCurrentListItem {
                    state: StateId::new(#id),
                    value: #value,
                }
            }
        }
        Action::ClearList { state } => {
            let id = state.0;
            quote! { Action::ClearList { state: StateId::new(#id) } }
        }
        Action::ReloadResource { resource } => {
            let id = resource.0;
            quote! { Action::ReloadResource { resource: ResourceId::new(#id) } }
        }
        Action::Controller {
            controller,
            operation,
            payload,
        } => {
            let id = controller.0;
            let payload = payload.iter().map(payload_part);
            quote! {
                Action::Controller {
                    controller: ControllerId::new(#id),
                    operation: #operation.to_owned(),
                    payload: vec![#(#payload),*],
                }
            }
        }
        Action::Native { operation } => {
            let operation = native_operation_tokens(operation);
            quote! { Action::Native { operation: #operation } }
        }
        Action::Navigate { path, .. } => {
            quote! { Action::Navigate { path: #path.to_owned() } }
        }
        Action::Back => quote! { Action::Back },
        Action::Sequence(actions) => {
            let actions = actions.iter().map(action_tokens);
            quote! { Action::Sequence(vec![#(#actions),*]) }
        }
    }
}

fn native_operation_tokens(operation: &NativeOperation) -> TokenStream {
    let module = &operation.module;
    let name = &operation.operation;
    let payload = operation.payload.iter().map(payload_part);
    let timeout_ms = operation.timeout_ms;
    quote! { NativeOperation::templated(#module, #name, vec![#(#payload),*], #timeout_ms) }
}

fn payload_part(part: &PayloadPart) -> TokenStream {
    match part {
        PayloadPart::Literal(value) => quote! { PayloadPart::Literal(#value.to_owned()) },
        PayloadPart::State(state) => {
            let id = state.0;
            quote! { PayloadPart::State(StateId::new(#id)) }
        }
        PayloadPart::Item(path) => quote! { PayloadPart::Item(vec![#(#path.to_owned()),*]) },
    }
}

fn condition_tokens(condition: &Condition) -> TokenStream {
    match condition {
        Condition::Bool { state, expected } => {
            let id = state.0;
            quote! { Condition::Bool { state: StateId::new(#id), expected: #expected } }
        }
        Condition::ListEmpty { state, expected } => {
            let id = state.0;
            quote! { Condition::ListEmpty { state: StateId::new(#id), expected: #expected } }
        }
        Condition::Equals {
            state,
            value,
            expected,
        } => {
            let id = state.0;
            let value = state_value(value);
            quote! {
                Condition::Equals {
                    state: StateId::new(#id),
                    value: #value,
                    expected: #expected,
                }
            }
        }
        Condition::ResourceEquals {
            resource,
            field,
            value,
            expected,
        } => {
            let id = resource.0;
            let field = resource_field_tokens(field);
            let value = state_value(value);
            quote! {
                Condition::ResourceEquals {
                    resource: ResourceId::new(#id),
                    field: #field,
                    value: #value,
                    expected: #expected,
                }
            }
        }
        Condition::ControllerEquals {
            controller,
            path,
            value,
            expected,
        } => {
            let id = controller.0;
            let value = state_value(value);
            quote! {
                Condition::ControllerEquals {
                    controller: ControllerId::new(#id),
                    path: vec![#(#path.to_owned()),*],
                    value: #value,
                    expected: #expected,
                }
            }
        }
    }
}

fn resource_field_tokens(field: &ResourceField) -> TokenStream {
    match field {
        ResourceField::Status => quote! { ResourceField::Status },
        ResourceField::Value(path) => {
            quote! { ResourceField::Value(vec![#(#path.to_owned()),*]) }
        }
        ResourceField::ErrorKind => quote! { ResourceField::ErrorKind },
        ResourceField::ErrorMessage => quote! { ResourceField::ErrorMessage },
        ResourceField::ErrorRetryable => quote! { ResourceField::ErrorRetryable },
        ResourceField::UpdatedAtMs => quote! { ResourceField::UpdatedAtMs },
        ResourceField::ErrorAttemptedAtMs => quote! { ResourceField::ErrorAttemptedAtMs },
    }
}

fn value_tokens(value: &Value) -> TokenStream {
    match value {
        Value::Number(value) => quote! { Value::Number(#value) },
        Value::Bool(value) => quote! { Value::Bool(#value) },
        Value::String(value) => quote! { Value::String(#value.to_owned()) },
        Value::State(state) => {
            let id = state.0;
            quote! { Value::State(StateId::new(#id)) }
        }
        Value::Item(path) => quote! { Value::Item(vec![#(#path.to_owned()),*]) },
        Value::List(values) => {
            let values = values.iter().map(value_tokens);
            quote! { Value::List(vec![#(#values),*]) }
        }
        Value::Object(fields) => {
            let fields = fields.iter().map(|(name, value)| {
                let value = value_tokens(value);
                quote! { (#name.to_owned(), #value) }
            });
            quote! { Value::Object(vec![#(#fields),*]) }
        }
    }
}

fn axis_tokens(axis: Axis) -> TokenStream {
    match axis {
        Axis::Vertical => quote! { Axis::Vertical },
        Axis::Horizontal => quote! { Axis::Horizontal },
    }
}

fn alignment_tokens(alignment: Alignment) -> TokenStream {
    match alignment {
        Alignment::Start => quote! { Alignment::Start },
        Alignment::Center => quote! { Alignment::Centre },
        Alignment::End => quote! { Alignment::End },
        Alignment::Stretch => quote! { Alignment::Stretch },
    }
}

fn justification_tokens(justification: Justification) -> TokenStream {
    match justification {
        Justification::Start => quote! { Justification::Start },
        Justification::Center => quote! { Justification::Centre },
        Justification::End => quote! { Justification::End },
        Justification::SpaceBetween => quote! { Justification::SpaceBetween },
    }
}

fn text_alignment_tokens(alignment: TextAlignment) -> TokenStream {
    match alignment {
        TextAlignment::Start => quote! { TextAlign::Start },
        TextAlignment::Center => quote! { TextAlign::Centre },
        TextAlignment::End => quote! { TextAlign::End },
    }
}

fn tone_tokens(tone: Tone) -> TokenStream {
    match tone {
        Tone::Primary => quote! { Tone::Primary },
        Tone::Muted => quote! { Tone::Muted },
    }
}

fn image_fit_tokens(fit: ImageFit) -> TokenStream {
    match fit {
        ImageFit::Cover => quote! { ink_core::ImageFit::Cover },
        ImageFit::Contain => quote! { ink_core::ImageFit::Contain },
    }
}

fn option_f32(value: Option<f32>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value) },
        None => quote! { None },
    }
}

fn option_string(value: Option<&str>) -> TokenStream {
    match value {
        Some(value) => quote! { Some(#value.to_owned()) },
        None => quote! { None },
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut value = 0xcbf29ce484222325_u64;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x100000001b3);
    }
    value
}
