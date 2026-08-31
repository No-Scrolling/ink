use std::fmt;

use ink_app_format as wire;

use super::*;

#[derive(Debug)]
pub struct AppDefinitionError(wire::FormatError);

impl fmt::Display for AppDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for AppDefinitionError {}

pub fn decode(bytes: &[u8]) -> Result<AppDefinition, AppDefinitionError> {
    let mut application = wire::decode(bytes).map_err(AppDefinitionError)?;
    let masks = std::mem::take(&mut application.masks);
    let images = std::mem::take(&mut application.images);
    Ok(Converter::new(masks, images).application(application))
}

struct Converter {
    masks: Vec<Mask>,
    images: Vec<ImageAsset>,
}

impl Converter {
    fn new(masks: Vec<wire::MaskAsset>, images: Vec<wire::ImageAsset>) -> Self {
        Self {
            masks: masks
                .into_iter()
                .map(|mask| Mask::owned(mask.id, mask.width, mask.height, mask.pixels))
                .collect(),
            images: images
                .into_iter()
                .map(|image| {
                    ImageAsset::owned(image.id, image.width, image.height, image.compressed_pixels)
                })
                .collect(),
        }
    }

    fn application(&self, application: wire::Application) -> AppDefinition {
        AppDefinition::new(
            application
                .states
                .into_iter()
                .map(state_definition)
                .collect(),
            application
                .resources
                .into_iter()
                .map(resource_definition)
                .collect(),
            application
                .application_resources
                .into_iter()
                .map(resource_id)
                .collect(),
            application
                .controllers
                .into_iter()
                .map(controller_definition)
                .collect(),
            application
                .application_controllers
                .into_iter()
                .map(controller_id)
                .collect(),
            self.node(application.root),
        )
    }

    fn node(&self, node: wire::Node) -> Node {
        match node {
            wire::Node::Screen {
                children,
                title,
                centred,
                resources,
                controllers,
            } => Node::screen(
                children.into_iter().map(|node| self.node(node)).collect(),
                title,
                centred,
                resources.into_iter().map(resource_id).collect(),
                controllers.into_iter().map(controller_id).collect(),
            ),
            wire::Node::Stack {
                children,
                axis,
                gap,
                align,
                justify,
            } => Node::stack(
                children.into_iter().map(|node| self.node(node)).collect(),
                axis_value(axis),
                gap,
                alignment(align),
                justification(justify),
            ),
            wire::Node::Text {
                parts,
                font_size,
                align,
            } => Node::text(
                parts.into_iter().map(text_part).collect(),
                font_size,
                text_align(align),
            ),
            wire::Node::TextInput {
                placeholder,
                state,
                action,
            } => Node::text_input(placeholder, state_id(state), text_input_action(action)),
            wire::Node::Button {
                label,
                icon,
                underline,
                action,
            } => Node::button(
                label.into_iter().map(text_part).collect(),
                icon.map(|id| self.mask(id)),
                underline,
                action.map(action_value),
            ),
            wire::Node::SelectorButton {
                label,
                value,
                action,
            } => Node::selector_button(
                label,
                value.into_iter().map(text_part).collect(),
                action.map(action_value),
            ),
            wire::Node::Icon { mask, size, tone } => {
                Node::icon(self.mask(mask), size, tone_value(tone))
            }
            wire::Node::Image {
                source,
                fallback,
                bleed,
                width,
                height,
                fit,
            } => Node::image(
                self.image_source(source),
                fallback.map(|id| self.image(id)),
                bleed,
                width,
                height,
                image_fit(fit),
            ),
            wire::Node::CameraPreview { controller, kind } => {
                Node::camera_preview(controller_id(controller), camera_preview_kind(kind))
            }
            wire::Node::Toggle {
                label,
                state,
                action,
                off,
                on,
            } => Node::toggle(
                label,
                state_id(state),
                action_value(action),
                self.mask(off),
                self.mask(on),
            ),
            wire::Node::Tabs { state, tabs } => Node::tabs(
                state_id(state),
                tabs.into_iter()
                    .map(|tab| {
                        Tab::new(
                            self.mask(tab.icon),
                            action_value(tab.action),
                            self.node(tab.screen),
                        )
                    })
                    .collect(),
            ),
            wire::Node::Navigator { routes, back } => Node::navigator(
                routes
                    .into_iter()
                    .map(|route| Route::new(route.path, self.node(route.screen)))
                    .collect(),
                self.mask(back),
            ),
            wire::Node::Conditional {
                condition,
                consequent,
                alternate,
            } => Node::conditional(
                condition_value(condition),
                self.node(*consequent),
                alternate.map(|node| self.node(*node)),
            ),
            wire::Node::ForEach {
                collection,
                template,
            } => Node::for_each(collection_value(collection), self.node(*template)),
        }
    }

    fn mask(&self, id: wire::MaskId) -> Mask {
        self.masks[id.0 as usize].clone()
    }

    fn image(&self, id: wire::ImageId) -> ImageAsset {
        self.images[id.0 as usize].clone()
    }

    fn image_source(&self, source: wire::ImageSource) -> ImageSource {
        match source {
            wire::ImageSource::Asset(id) => ImageSource::Asset(self.image(id)),
            wire::ImageSource::Remote(parts) => {
                ImageSource::Remote(parts.into_iter().map(text_part).collect())
            }
            wire::ImageSource::Native(module, parts) => {
                ImageSource::Native(module, parts.into_iter().map(text_part).collect())
            }
        }
    }
}

fn state_definition(definition: wire::StateDefinition) -> StateDefinition {
    StateDefinition {
        initial: state_value(definition.initial),
        shape: state_shape(definition.shape),
        persisted: definition.persisted.map(|persisted| PersistedState {
            key: persisted.key,
            schema: persisted.schema,
            shape: state_shape(persisted.shape),
        }),
    }
}

fn resource_definition(definition: wire::ResourceDefinition) -> ResourceDefinition {
    ResourceDefinition {
        shape: state_shape(definition.shape),
        read: native_operation(definition.read),
        reload_on_resume: definition.reload_on_resume,
        protocol: match definition.protocol {
            wire::ResourceProtocol::Async => ResourceProtocol::Async,
            wire::ResourceProtocol::Cached => ResourceProtocol::Cached,
            wire::ResourceProtocol::Mutation => ResourceProtocol::Mutation,
            wire::ResourceProtocol::Background => ResourceProtocol::Background,
        },
    }
}

fn controller_definition(definition: wire::ControllerDefinition) -> ControllerDefinition {
    ControllerDefinition {
        state: state_id(definition.state),
        module: definition.module,
        kind: definition.kind,
        config: definition.config,
    }
}

fn state_shape(shape: wire::StateShape) -> StateShape {
    match shape {
        wire::StateShape::Null => StateShape::Null,
        wire::StateShape::Number => StateShape::Number,
        wire::StateShape::Bool => StateShape::Bool,
        wire::StateShape::String => StateShape::String,
        wire::StateShape::Literal(value) => StateShape::Literal(match value {
            wire::StateLiteral::Number(value) => StateLiteral::Number(value),
            wire::StateLiteral::Bool(value) => StateLiteral::Bool(value),
            wire::StateLiteral::String(value) => StateLiteral::String(value),
        }),
        wire::StateShape::Optional(shape) => StateShape::Optional(Box::new(state_shape(*shape))),
        wire::StateShape::Union(shapes) => {
            StateShape::Union(shapes.into_iter().map(state_shape).collect())
        }
        wire::StateShape::List(shape) => StateShape::List(Box::new(state_shape(*shape))),
        wire::StateShape::Object(fields) => StateShape::Object(
            fields
                .into_iter()
                .map(|(name, shape)| (name, state_shape(shape)))
                .collect(),
        ),
    }
}

fn state_value(value: wire::StateValue) -> StateValue {
    match value {
        wire::StateValue::Null => StateValue::Null,
        wire::StateValue::Number(value) => StateValue::Number(value),
        wire::StateValue::Bool(value) => StateValue::Bool(value),
        wire::StateValue::String(value) => StateValue::String(value),
        wire::StateValue::List(values) => {
            StateValue::List(values.into_iter().map(state_value).collect())
        }
        wire::StateValue::Object(fields) => StateValue::Object(
            fields
                .into_iter()
                .map(|(name, value)| (name, state_value(value)))
                .collect(),
        ),
    }
}

fn text_part(part: wire::TextPart) -> TextPart {
    match part {
        wire::TextPart::Literal(value) => TextPart::Literal(value),
        wire::TextPart::State(id) => TextPart::State(state_id(id)),
        wire::TextPart::Resource(id, field) => {
            TextPart::Resource(resource_id(id), resource_field(field))
        }
        wire::TextPart::Controller(id, path) => TextPart::Controller(controller_id(id), path),
        wire::TextPart::ListLength(id) => TextPart::ListLength(state_id(id)),
        wire::TextPart::Item(path) => TextPart::Item(path),
        wire::TextPart::Value(value) => TextPart::Value(value_value(value)),
    }
}

fn action_value(action: wire::Action) -> Action {
    match action {
        wire::Action::Increment { state, by } => Action::Increment {
            state: state_id(state),
            by,
        },
        wire::Action::SetValue { state, value } => Action::SetValue {
            state: state_id(state),
            value: value_value(value),
        },
        wire::Action::Toggle { state } => Action::Toggle {
            state: state_id(state),
        },
        wire::Action::SetList { state, value } => Action::SetList {
            state: state_id(state),
            value: value_value(value),
        },
        wire::Action::AppendList { state, value } => Action::AppendList {
            state: state_id(state),
            value: value_value(value),
        },
        wire::Action::RemoveCurrentListItem { state } => Action::RemoveCurrentListItem {
            state: state_id(state),
        },
        wire::Action::ReplaceCurrentListItem { state, value } => Action::ReplaceCurrentListItem {
            state: state_id(state),
            value: value_value(value),
        },
        wire::Action::ClearList { state } => Action::ClearList {
            state: state_id(state),
        },
        wire::Action::ReloadResource { resource } => Action::ReloadResource {
            resource: resource_id(resource),
        },
        wire::Action::Controller {
            controller,
            operation,
            payload,
        } => Action::Controller {
            controller: controller_id(controller),
            operation,
            payload: payload.into_iter().map(payload_part).collect(),
        },
        wire::Action::Native { operation } => Action::Native {
            operation: native_operation(operation),
        },
        wire::Action::Navigate { path, params } => Action::Navigate {
            path,
            params: params
                .into_iter()
                .map(|(name, value)| (name, value_value(value)))
                .collect(),
        },
        wire::Action::Back => Action::Back,
        wire::Action::Sequence(actions) => {
            Action::Sequence(actions.into_iter().map(action_value).collect())
        }
    }
}

fn condition_value(condition: wire::Condition) -> Condition {
    match condition {
        wire::Condition::ValueEquals {
            value,
            expected,
            equals,
        } => Condition::ValueEquals {
            value: value_value(value),
            expected: state_value(expected),
            equals,
        },
        wire::Condition::Bool { state, expected } => Condition::Bool {
            state: state_id(state),
            expected,
        },
        wire::Condition::ListEmpty { state, expected } => Condition::ListEmpty {
            state: state_id(state),
            expected,
        },
        wire::Condition::Equals {
            state,
            value,
            expected,
        } => Condition::Equals {
            state: state_id(state),
            value: state_value(value),
            expected,
        },
        wire::Condition::ResourceEquals {
            resource,
            field,
            value,
            expected,
        } => Condition::ResourceEquals {
            resource: resource_id(resource),
            field: resource_field(field),
            value: state_value(value),
            expected,
        },
        wire::Condition::ControllerEquals {
            controller,
            path,
            value,
            expected,
        } => Condition::ControllerEquals {
            controller: controller_id(controller),
            path,
            value: state_value(value),
            expected,
        },
    }
}

fn value_value(value: wire::Value) -> Value {
    match value {
        wire::Value::Null => Value::Null,
        wire::Value::Number(value) => Value::Number(value),
        wire::Value::Bool(value) => Value::Bool(value),
        wire::Value::String(value) => Value::String(value),
        wire::Value::State(id) => Value::State(state_id(id)),
        wire::Value::Item(path) => Value::Item(path),
        wire::Value::Resource(id, field) => Value::Resource(resource_id(id), resource_field(field)),
        wire::Value::Controller(id, path) => Value::Controller(controller_id(id), path),
        wire::Value::CombinedStatus(ids) => {
            Value::CombinedStatus(ids.into_iter().map(resource_id).collect())
        }
        wire::Value::CombinedErrorResource(resources) => Value::CombinedErrorResource(
            resources
                .into_iter()
                .map(|(name, id)| (name, resource_id(id)))
                .collect(),
        ),
        wire::Value::CombinedErrorField(ids, field) => Value::CombinedErrorField(
            ids.into_iter().map(resource_id).collect(),
            resource_field(field),
        ),
        wire::Value::ListLength(id) => Value::ListLength(state_id(id)),
        wire::Value::Binary {
            left,
            operator,
            right,
        } => Value::Binary {
            left: Box::new(value_value(*left)),
            operator: match operator {
                wire::ValueOperator::Add => ValueOperator::Add,
                wire::ValueOperator::Subtract => ValueOperator::Subtract,
                wire::ValueOperator::Multiply => ValueOperator::Multiply,
                wire::ValueOperator::Divide => ValueOperator::Divide,
            },
            right: Box::new(value_value(*right)),
        },
        wire::Value::RouteParam(name) => Value::RouteParam(name),
        wire::Value::List(values) => Value::List(values.into_iter().map(value_value).collect()),
        wire::Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(name, value)| (name, value_value(value)))
                .collect(),
        ),
    }
}

fn native_operation(operation: wire::NativeOperation) -> NativeOperation {
    NativeOperation {
        module: operation.module,
        operation: operation.operation,
        payload: operation.payload.into_iter().map(payload_part).collect(),
        timeout_ms: operation.timeout_ms,
    }
}

fn payload_part(part: wire::PayloadPart) -> PayloadPart {
    match part {
        wire::PayloadPart::Literal(value) => PayloadPart::Literal(value),
        wire::PayloadPart::State(id) => PayloadPart::State(state_id(id)),
        wire::PayloadPart::Item(path) => PayloadPart::Item(path),
    }
}

fn collection_value(collection: wire::Collection) -> Collection {
    match collection {
        wire::Collection::State(id) => Collection::State(state_id(id)),
        wire::Collection::Resource(id, path) => Collection::Resource(resource_id(id), path),
        wire::Collection::Controller(id, path) => Collection::Controller(controller_id(id), path),
    }
}

fn resource_field(field: wire::ResourceField) -> ResourceField {
    match field {
        wire::ResourceField::Status => ResourceField::Status,
        wire::ResourceField::Value(path) => ResourceField::Value(path),
        wire::ResourceField::ErrorKind => ResourceField::ErrorKind,
        wire::ResourceField::ErrorMessage => ResourceField::ErrorMessage,
        wire::ResourceField::ErrorRetryable => ResourceField::ErrorRetryable,
        wire::ResourceField::UpdatedAtMs => ResourceField::UpdatedAtMs,
        wire::ResourceField::ErrorAttemptedAtMs => ResourceField::ErrorAttemptedAtMs,
    }
}

fn axis_value(axis: wire::Axis) -> Axis {
    match axis {
        wire::Axis::Vertical => Axis::Vertical,
        wire::Axis::Horizontal => Axis::Horizontal,
    }
}

fn alignment(alignment: wire::Alignment) -> Alignment {
    match alignment {
        wire::Alignment::Start => Alignment::Start,
        wire::Alignment::Centre => Alignment::Centre,
        wire::Alignment::End => Alignment::End,
        wire::Alignment::Stretch => Alignment::Stretch,
    }
}

fn justification(justification: wire::Justification) -> Justification {
    match justification {
        wire::Justification::Start => Justification::Start,
        wire::Justification::Centre => Justification::Centre,
        wire::Justification::End => Justification::End,
        wire::Justification::SpaceBetween => Justification::SpaceBetween,
    }
}

fn text_align(alignment: wire::TextAlign) -> TextAlign {
    match alignment {
        wire::TextAlign::Start => TextAlign::Start,
        wire::TextAlign::Centre => TextAlign::Centre,
        wire::TextAlign::End => TextAlign::End,
    }
}

fn text_input_action(action: wire::TextInputAction) -> TextInputAction {
    match action {
        wire::TextInputAction::Return => TextInputAction::Return,
        wire::TextInputAction::Search => TextInputAction::Search,
        wire::TextInputAction::Done => TextInputAction::Done,
    }
}

fn tone_value(tone: wire::Tone) -> Tone {
    match tone {
        wire::Tone::Primary => Tone::Primary,
        wire::Tone::Muted => Tone::Muted,
    }
}

fn image_fit(fit: wire::ImageFit) -> ImageFit {
    match fit {
        wire::ImageFit::Cover => ImageFit::Cover,
        wire::ImageFit::Contain => ImageFit::Contain,
    }
}

fn camera_preview_kind(kind: wire::CameraPreviewKind) -> CameraPreviewKind {
    match kind {
        wire::CameraPreviewKind::Photo => CameraPreviewKind::Photo,
        wire::CameraPreviewKind::Scanner => CameraPreviewKind::Scanner,
    }
}

fn state_id(id: wire::StateId) -> StateId {
    StateId::new(id.0 as usize)
}

fn resource_id(id: wire::ResourceId) -> ResourceId {
    ResourceId::new(id.0 as usize)
}

fn controller_id(id: wire::ControllerId) -> ControllerId {
    ControllerId::new(id.0 as usize)
}
