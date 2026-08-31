use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use ink_app_format as format;

use crate::{
    design::{
        BUTTON_ICON_SIZE, DEFAULT_ICON_SIZE, HEADER_BACK_ICON_SIZE, TAB_ICON_SIZE, TOGGLE_ICON_SIZE,
    },
    icons, ir,
    schema::{hash_bytes, state_shape_name},
};

pub fn generate(app: &ir::App, root: &Path) -> Result<Vec<u8>> {
    let mut emitter = Emitter::new(root);
    let root = emitter.node(&app.root)?;
    let application = format::Application {
        states: app.states.iter().map(state_definition).collect(),
        state_dependencies: crate::dependency::analyse(&app.root),
        resources: app
            .resources
            .iter()
            .map(resource_definition)
            .collect::<Result<_>>()?,
        application_resources: app
            .application_resources
            .iter()
            .map(|id| resource_id(*id))
            .collect::<Result<_>>()?,
        controllers: app
            .controllers
            .iter()
            .map(controller_definition)
            .collect::<Result<_>>()?,
        application_controllers: app
            .application_controllers
            .iter()
            .map(|id| controller_id(*id))
            .collect::<Result<_>>()?,
        masks: emitter.masks,
        images: emitter.images,
        root,
    };
    let bytes = format::encode(&application)?;
    let decoded = format::decode(&bytes)?;
    anyhow::ensure!(
        decoded == application,
        "Ink application round trip changed data"
    );
    Ok(bytes)
}

struct Emitter<'a> {
    root: &'a Path,
    masks: Vec<format::MaskAsset>,
    images: Vec<format::ImageAsset>,
    mask_ids: HashMap<(String, u32), format::MaskId>,
    image_ids: HashMap<PathBuf, format::ImageId>,
}

impl<'a> Emitter<'a> {
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            masks: Vec::new(),
            images: Vec::new(),
            mask_ids: HashMap::new(),
            image_ids: HashMap::new(),
        }
    }

    fn node(&mut self, node: &ir::Node) -> Result<format::Node> {
        Ok(match node {
            ir::Node::Screen {
                children,
                title,
                centered,
                params: _,
                resources,
                controllers,
            } => format::Node::Screen {
                children: self.children(children)?,
                title: title.clone(),
                centred: *centered,
                resources: resources
                    .iter()
                    .map(|id| resource_id(*id))
                    .collect::<Result<_>>()?,
                controllers: controllers
                    .iter()
                    .map(|id| controller_id(*id))
                    .collect::<Result<_>>()?,
            },
            ir::Node::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            } => format::Node::Stack {
                children: self.children(children)?,
                axis: axis_value(*axis),
                gap: *gap,
                align: alignment(*align),
                justify: justification(*justify),
            },
            ir::Node::Text {
                parts,
                font_size,
                align,
                max_lines,
            } => format::Node::Text {
                parts: text_parts(parts)?,
                font_size: *font_size,
                align: text_align(*align),
                max_lines: *max_lines,
            },
            ir::Node::TextInput {
                placeholder,
                state,
                action,
                auto_focus,
            } => format::Node::TextInput {
                placeholder: placeholder.clone(),
                state: state_id(*state)?,
                action: text_input_action(*action),
                auto_focus: *auto_focus,
            },
            ir::Node::Button {
                label,
                icon,
                underline,
                action,
            } => format::Node::Button {
                label: text_parts(label)?,
                icon: icon
                    .as_deref()
                    .map(|name| self.mask(name, BUTTON_ICON_SIZE, false))
                    .transpose()?,
                underline: *underline,
                action: action.as_ref().map(action_value).transpose()?,
            },
            ir::Node::Field {
                label,
                value,
                action,
            } => format::Node::Field {
                label: label.clone(),
                value: text_parts(value)?,
                action: action.as_ref().map(action_value).transpose()?,
            },
            ir::Node::Icon { name, size, tone } => {
                let size = size.unwrap_or(DEFAULT_ICON_SIZE);
                format::Node::Icon {
                    mask: self.mask(name, size, false)?,
                    size,
                    tone: tone_value(*tone),
                }
            }
            ir::Node::Image {
                source,
                fallback,
                bleed,
                width,
                height,
                fit,
            } => format::Node::Image {
                source: match source {
                    ir::ImageSource::Local(source) => {
                        format::ImageSource::Asset(self.image(source)?)
                    }
                    ir::ImageSource::Remote(parts) => {
                        format::ImageSource::Remote(text_parts(parts)?)
                    }
                    ir::ImageSource::Camera(parts) => {
                        format::ImageSource::Native("camera".to_owned(), text_parts(parts)?)
                    }
                },
                fallback: fallback
                    .as_deref()
                    .map(|source| self.image(source))
                    .transpose()?,
                bleed: *bleed,
                width: *width,
                height: *height,
                fit: image_fit(*fit),
            },
            ir::Node::CameraPreview { controller, kind } => format::Node::CameraPreview {
                controller: controller_id(*controller)?,
                kind: camera_preview_kind(*kind),
            },
            ir::Node::Toggle {
                label,
                state,
                action,
            } => format::Node::Toggle {
                label: label.clone(),
                state: state_id(*state)?,
                action: action_value(action)?,
                off: self.toggle_mask(false)?,
                on: self.toggle_mask(true)?,
            },
            ir::Node::Tabs { state, tabs } => {
                let mut encoded = Vec::with_capacity(tabs.len());
                for tab in tabs {
                    encoded.push(format::Tab {
                        icon: self.mask(&tab.icon, TAB_ICON_SIZE, true)?,
                        action: action_value(&tab.action)?,
                        screen: self.node(&tab.screen)?,
                    });
                }
                format::Node::Tabs {
                    state: state_id(*state)?,
                    tabs: encoded,
                }
            }
            ir::Node::Navigator { routes } => {
                let mut encoded = Vec::with_capacity(routes.len());
                for route in routes {
                    encoded.push(format::Route {
                        path: route.path.clone(),
                        screen: self.node(&route.screen)?,
                    });
                }
                format::Node::Navigator {
                    routes: encoded,
                    back: self.mask("arrow_back_ios", HEADER_BACK_ICON_SIZE, false)?,
                }
            }
            ir::Node::Conditional {
                condition,
                consequent,
                alternate,
            } => format::Node::Conditional {
                condition: condition_value(condition)?,
                consequent: Box::new(self.node(consequent)?),
                alternate: alternate
                    .as_deref()
                    .map(|node| self.node(node).map(Box::new))
                    .transpose()?,
            },
            ir::Node::ForEach {
                collection,
                template,
            } => format::Node::ForEach {
                collection: collection_value(collection)?,
                template: Box::new(self.node(template)?),
            },
            ir::Node::ScreenModule { .. } => {
                unreachable!("screen modules are expanded before bundle generation")
            }
        })
    }

    fn children(&mut self, children: &[ir::Node]) -> Result<Vec<format::Node>> {
        children.iter().map(|child| self.node(child)).collect()
    }

    fn mask(&mut self, name: &str, size: f32, filled: bool) -> Result<format::MaskId> {
        let key = (
            if filled {
                format!("filled:{name}")
            } else {
                name.to_owned()
            },
            size.to_bits(),
        );
        if let Some(id) = self.mask_ids.get(&key) {
            return Ok(*id);
        }
        let icon = if filled {
            icons::raster_filled(name, size)?
        } else {
            icons::raster(name, size)?
        };
        self.push_mask(key, icon)
    }

    fn toggle_mask(&mut self, filled: bool) -> Result<format::MaskId> {
        let key = (
            if filled {
                "ink_toggle_on"
            } else {
                "ink_toggle_off"
            }
            .to_owned(),
            TOGGLE_ICON_SIZE.to_bits(),
        );
        if let Some(id) = self.mask_ids.get(&key) {
            return Ok(*id);
        }
        self.push_mask(key, icons::toggle_circle(filled))
    }

    fn push_mask(&mut self, key: (String, u32), icon: icons::RasterIcon) -> Result<format::MaskId> {
        let id = format::MaskId(index(self.masks.len(), "mask")?);
        self.masks.push(format::MaskAsset {
            id: icon.id,
            width: icon.width,
            height: icon.height,
            pixels: icon.pixels,
        });
        self.mask_ids.insert(key, id);
        Ok(id)
    }

    fn image(&mut self, source: &str) -> Result<format::ImageId> {
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
        if let Some(id) = self.image_ids.get(&path) {
            return Ok(*id);
        }
        let bytes = std::fs::read(&path)
            .with_context(|| format!("could not read image {}", path.display()))?;
        let pixels = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .with_context(|| format!("could not decode image {}", path.display()))?
            .into_rgba8();
        let id = format::ImageId(index(self.images.len(), "image")?);
        self.images.push(format::ImageAsset {
            id: hash_bytes(&bytes),
            width: pixels.width(),
            height: pixels.height(),
            compressed_pixels: miniz_oxide::deflate::compress_to_vec_zlib(pixels.as_raw(), 9),
        });
        self.image_ids.insert(path, id);
        Ok(id)
    }
}

fn state_definition(state: &ir::State) -> format::StateDefinition {
    let shape = state_shape(&state.shape);
    let persisted = match &state.lifetime {
        ir::StateLifetime::Local | ir::StateLifetime::Shared(_) => None,
        ir::StateLifetime::Persisted(key) => Some(format::PersistedState {
            key: key.clone(),
            schema: hash_bytes(state_shape_name(&state.shape).as_bytes()),
            shape: shape.clone(),
        }),
    };
    format::StateDefinition {
        initial: state_value(&state.initial),
        shape,
        persisted,
    }
}

fn resource_definition(resource: &ir::Resource) -> Result<format::ResourceDefinition> {
    Ok(format::ResourceDefinition {
        shape: state_shape(&resource.shape),
        read: native_operation(&ir::NativeOperation {
            module: resource.module.clone(),
            operation: resource.operation.clone(),
            payload: resource.payload.clone(),
            timeout_ms: resource.timeout_ms,
        })?,
        reload_on_resume: resource.reload_on_resume,
        protocol: match resource.protocol {
            ir::ResourceProtocol::Async => format::ResourceProtocol::Async,
            ir::ResourceProtocol::Cached => format::ResourceProtocol::Cached,
            ir::ResourceProtocol::Mutation => format::ResourceProtocol::Mutation,
            ir::ResourceProtocol::Background => format::ResourceProtocol::Background,
        },
    })
}

fn controller_definition(controller: &ir::Controller) -> Result<format::ControllerDefinition> {
    Ok(format::ControllerDefinition {
        state: state_id(controller.state)?,
        module: controller.module.clone(),
        kind: controller.kind.clone(),
        config: controller.config.clone(),
    })
}

fn state_shape(shape: &ir::StateShape) -> format::StateShape {
    match shape {
        ir::StateShape::Null => format::StateShape::Null,
        ir::StateShape::Number => format::StateShape::Number,
        ir::StateShape::Bool => format::StateShape::Bool,
        ir::StateShape::String => format::StateShape::String,
        ir::StateShape::Literal(value) => format::StateShape::Literal(match value {
            ir::StateLiteral::Number(value) => format::StateLiteral::Number(*value),
            ir::StateLiteral::Bool(value) => format::StateLiteral::Bool(*value),
            ir::StateLiteral::String(value) => format::StateLiteral::String(value.clone()),
        }),
        ir::StateShape::Optional(shape) => {
            format::StateShape::Optional(Box::new(state_shape(shape)))
        }
        ir::StateShape::Union(shapes) => {
            format::StateShape::Union(shapes.iter().map(state_shape).collect())
        }
        ir::StateShape::List(item) => format::StateShape::List(Box::new(state_shape(item))),
        ir::StateShape::Object(fields) => format::StateShape::Object(
            fields
                .iter()
                .map(|(name, shape)| (name.clone(), state_shape(shape)))
                .collect(),
        ),
    }
}

fn state_value(value: &ir::StateValue) -> format::StateValue {
    match value {
        ir::StateValue::Null => format::StateValue::Null,
        ir::StateValue::Number(value) => format::StateValue::Number(*value),
        ir::StateValue::Bool(value) => format::StateValue::Bool(*value),
        ir::StateValue::String(value) => format::StateValue::String(value.clone()),
        ir::StateValue::List(values) => {
            format::StateValue::List(values.iter().map(state_value).collect())
        }
        ir::StateValue::Object(fields) => format::StateValue::Object(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), state_value(value)))
                .collect(),
        ),
    }
}

fn text_parts(parts: &[ir::TextPart]) -> Result<Vec<format::TextPart>> {
    parts.iter().map(text_part).collect()
}

fn text_part(part: &ir::TextPart) -> Result<format::TextPart> {
    Ok(match part {
        ir::TextPart::Literal(value) => format::TextPart::Literal(value.clone()),
        ir::TextPart::State(id) => format::TextPart::State(state_id(*id)?),
        ir::TextPart::Resource(id, field) => {
            format::TextPart::Resource(resource_id(*id)?, resource_field(field))
        }
        ir::TextPart::Controller(id, path) => {
            format::TextPart::Controller(controller_id(*id)?, path.clone())
        }
        ir::TextPart::ListLength(id) => format::TextPart::ListLength(state_id(*id)?),
        ir::TextPart::Item(path) => format::TextPart::Item(path.clone()),
        ir::TextPart::Value(value) => format::TextPart::Value(value_value(value)?),
    })
}

fn action_value(action: &ir::Action) -> Result<format::Action> {
    Ok(match action {
        ir::Action::Increment { state, by } => format::Action::Increment {
            state: state_id(*state)?,
            by: *by,
        },
        ir::Action::SetValue { state, value } => format::Action::SetValue {
            state: state_id(*state)?,
            value: value_value(value)?,
        },
        ir::Action::Toggle { state } => format::Action::Toggle {
            state: state_id(*state)?,
        },
        ir::Action::SetList { state, value } => format::Action::SetList {
            state: state_id(*state)?,
            value: value_value(value)?,
        },
        ir::Action::AppendList { state, value } => format::Action::AppendList {
            state: state_id(*state)?,
            value: value_value(value)?,
        },
        ir::Action::RemoveListItem { state } => format::Action::RemoveCurrentListItem {
            state: state_id(*state)?,
        },
        ir::Action::ReplaceListItem { state, value } => format::Action::ReplaceCurrentListItem {
            state: state_id(*state)?,
            value: value_value(value)?,
        },
        ir::Action::ClearList { state } => format::Action::ClearList {
            state: state_id(*state)?,
        },
        ir::Action::ReloadResource { resource } => format::Action::ReloadResource {
            resource: resource_id(*resource)?,
        },
        ir::Action::Controller {
            controller,
            operation,
            payload,
        } => format::Action::Controller {
            controller: controller_id(*controller)?,
            operation: operation.clone(),
            payload: payload.iter().map(payload_part).collect::<Result<_>>()?,
        },
        ir::Action::Native { operation } => format::Action::Native {
            operation: native_operation(operation)?,
        },
        ir::Action::Navigate { path, params, .. } => format::Action::Navigate {
            path: path.clone(),
            params: params
                .iter()
                .map(|param| Ok((param.name.clone(), value_value(&param.value)?)))
                .collect::<Result<_>>()?,
        },
        ir::Action::Back => format::Action::Back,
        ir::Action::Sequence(actions) => {
            format::Action::Sequence(actions.iter().map(action_value).collect::<Result<_>>()?)
        }
    })
}

fn condition_value(condition: &ir::Condition) -> Result<format::Condition> {
    Ok(match condition {
        ir::Condition::ValueEquals {
            value,
            expected,
            equals,
        } => format::Condition::ValueEquals {
            value: value_value(value)?,
            expected: state_value(expected),
            equals: *equals,
        },
        ir::Condition::Bool { state, expected } => format::Condition::Bool {
            state: state_id(*state)?,
            expected: *expected,
        },
        ir::Condition::ListEmpty { state, expected } => format::Condition::ListEmpty {
            state: state_id(*state)?,
            expected: *expected,
        },
        ir::Condition::Equals {
            state,
            value,
            expected,
        } => format::Condition::Equals {
            state: state_id(*state)?,
            value: state_value(value),
            expected: *expected,
        },
        ir::Condition::ResourceEquals {
            resource,
            field,
            value,
            expected,
        } => format::Condition::ResourceEquals {
            resource: resource_id(*resource)?,
            field: resource_field(field),
            value: state_value(value),
            expected: *expected,
        },
        ir::Condition::ControllerEquals {
            controller,
            path,
            value,
            expected,
        } => format::Condition::ControllerEquals {
            controller: controller_id(*controller)?,
            path: path.clone(),
            value: state_value(value),
            expected: *expected,
        },
    })
}

fn value_value(value: &ir::Value) -> Result<format::Value> {
    Ok(match value {
        ir::Value::Null => format::Value::Null,
        ir::Value::Number(value) => format::Value::Number(*value),
        ir::Value::Bool(value) => format::Value::Bool(*value),
        ir::Value::String(value) => format::Value::String(value.clone()),
        ir::Value::State(id) => format::Value::State(state_id(*id)?),
        ir::Value::Item(path) => format::Value::Item(path.clone()),
        ir::Value::Resource(id, field) => {
            format::Value::Resource(resource_id(*id)?, resource_field(field))
        }
        ir::Value::Controller(id, path) => {
            format::Value::Controller(controller_id(*id)?, path.clone())
        }
        ir::Value::CombinedStatus(ids) => format::Value::CombinedStatus(
            ids.iter()
                .map(|id| resource_id(*id))
                .collect::<Result<_>>()?,
        ),
        ir::Value::CombinedErrorResource(resources) => format::Value::CombinedErrorResource(
            resources
                .iter()
                .map(|(name, id)| Ok((name.clone(), resource_id(*id)?)))
                .collect::<Result<_>>()?,
        ),
        ir::Value::CombinedErrorField(ids, field) => format::Value::CombinedErrorField(
            ids.iter()
                .map(|id| resource_id(*id))
                .collect::<Result<_>>()?,
            resource_field(field),
        ),
        ir::Value::ListLength(id) => format::Value::ListLength(state_id(*id)?),
        ir::Value::Binary {
            left,
            operator,
            right,
        } => format::Value::Binary {
            left: Box::new(value_value(left)?),
            operator: match operator {
                ir::ValueOperator::Add => format::ValueOperator::Add,
                ir::ValueOperator::Subtract => format::ValueOperator::Subtract,
                ir::ValueOperator::Multiply => format::ValueOperator::Multiply,
                ir::ValueOperator::Divide => format::ValueOperator::Divide,
            },
            right: Box::new(value_value(right)?),
        },
        ir::Value::RouteParam(name) => format::Value::RouteParam(name.clone()),
        ir::Value::List(values) => {
            format::Value::List(values.iter().map(value_value).collect::<Result<_>>()?)
        }
        ir::Value::Object(fields) => format::Value::Object(
            fields
                .iter()
                .map(|(name, value)| Ok((name.clone(), value_value(value)?)))
                .collect::<Result<_>>()?,
        ),
    })
}

fn native_operation(operation: &ir::NativeOperation) -> Result<format::NativeOperation> {
    Ok(format::NativeOperation {
        module: operation.module.clone(),
        operation: operation.operation.clone(),
        payload: operation
            .payload
            .iter()
            .map(payload_part)
            .collect::<Result<_>>()?,
        timeout_ms: operation.timeout_ms,
    })
}

fn payload_part(part: &ir::PayloadPart) -> Result<format::PayloadPart> {
    Ok(match part {
        ir::PayloadPart::Literal(value) => format::PayloadPart::Literal(value.clone()),
        ir::PayloadPart::State(id) => format::PayloadPart::State(state_id(*id)?),
        ir::PayloadPart::Item(path) => format::PayloadPart::Item(path.clone()),
    })
}

fn collection_value(collection: &ir::Collection) -> Result<format::Collection> {
    Ok(match collection {
        ir::Collection::State(id) => format::Collection::State(state_id(*id)?),
        ir::Collection::Resource(id, path) => {
            format::Collection::Resource(resource_id(*id)?, path.clone())
        }
        ir::Collection::Controller(id, path) => {
            format::Collection::Controller(controller_id(*id)?, path.clone())
        }
    })
}

fn resource_field(field: &ir::ResourceField) -> format::ResourceField {
    match field {
        ir::ResourceField::Status => format::ResourceField::Status,
        ir::ResourceField::Value(path) => format::ResourceField::Value(path.clone()),
        ir::ResourceField::ErrorKind => format::ResourceField::ErrorKind,
        ir::ResourceField::ErrorMessage => format::ResourceField::ErrorMessage,
        ir::ResourceField::ErrorRetryable => format::ResourceField::ErrorRetryable,
        ir::ResourceField::UpdatedAtMs => format::ResourceField::UpdatedAtMs,
        ir::ResourceField::ErrorAttemptedAtMs => format::ResourceField::ErrorAttemptedAtMs,
    }
}

fn axis_value(axis: ir::Axis) -> format::Axis {
    match axis {
        ir::Axis::Vertical => format::Axis::Vertical,
        ir::Axis::Horizontal => format::Axis::Horizontal,
    }
}

fn alignment(alignment: ir::Alignment) -> format::Alignment {
    match alignment {
        ir::Alignment::Start => format::Alignment::Start,
        ir::Alignment::Center => format::Alignment::Centre,
        ir::Alignment::End => format::Alignment::End,
        ir::Alignment::Stretch => format::Alignment::Stretch,
    }
}

fn justification(justification: ir::Justification) -> format::Justification {
    match justification {
        ir::Justification::Start => format::Justification::Start,
        ir::Justification::Center => format::Justification::Centre,
        ir::Justification::End => format::Justification::End,
        ir::Justification::SpaceBetween => format::Justification::SpaceBetween,
    }
}

fn text_align(alignment: ir::TextAlignment) -> format::TextAlign {
    match alignment {
        ir::TextAlignment::Start => format::TextAlign::Start,
        ir::TextAlignment::Center => format::TextAlign::Centre,
        ir::TextAlignment::End => format::TextAlign::End,
        ir::TextAlignment::Justify => format::TextAlign::Justify,
    }
}

fn text_input_action(action: ir::TextInputAction) -> format::TextInputAction {
    match action {
        ir::TextInputAction::Return => format::TextInputAction::Return,
        ir::TextInputAction::Search => format::TextInputAction::Search,
        ir::TextInputAction::Done => format::TextInputAction::Done,
    }
}

fn tone_value(tone: ir::Tone) -> format::Tone {
    match tone {
        ir::Tone::Primary => format::Tone::Primary,
        ir::Tone::Muted => format::Tone::Muted,
    }
}

fn image_fit(fit: ir::ImageFit) -> format::ImageFit {
    match fit {
        ir::ImageFit::Cover => format::ImageFit::Cover,
        ir::ImageFit::Contain => format::ImageFit::Contain,
    }
}

fn camera_preview_kind(kind: ir::CameraPreviewKind) -> format::CameraPreviewKind {
    match kind {
        ir::CameraPreviewKind::Photo => format::CameraPreviewKind::Photo,
        ir::CameraPreviewKind::Scanner => format::CameraPreviewKind::Scanner,
    }
}

fn state_id(id: ir::StateId) -> Result<format::StateId> {
    Ok(format::StateId(index(id.0, "state")?))
}

fn resource_id(id: ir::ResourceId) -> Result<format::ResourceId> {
    Ok(format::ResourceId(index(id.0, "resource")?))
}

fn controller_id(id: ir::ControllerId) -> Result<format::ControllerId> {
    Ok(format::ControllerId(index(id.0, "controller")?))
}

fn index(value: usize, kind: &str) -> Result<u32> {
    u32::try_from(value).with_context(|| format!("too many Ink {kind}s"))
}
