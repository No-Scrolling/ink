use std::collections::{HashMap, HashSet};

use oxc::{
    ast::ast::{
        Argument, ArrowFunctionBody, BindingPattern, ExportDefaultDeclarationKind, Expression,
        Function, JSXAttribute, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild,
        JSXElement, JSXElementName, JSXExpression, Statement, VariableDeclarationKind,
    },
    span::{GetSpan, Span},
    syntax::operator::{BinaryOperator, UnaryOperator},
};

use crate::{
    diagnostic::CompileError,
    ir::{
        Action, Alignment, App, Axis, ImageFit, Justification, Node, State, StateId, StateValue,
        Tab, TextAlignment, TextPart, Tone,
    },
};

const INK_IMPORTS: [&str; 11] = [
    "Button",
    "Icon",
    "Image",
    "Screen",
    "Stack",
    "Tab",
    "Tabs",
    "Text",
    "TextInput",
    "Toggle",
    "state",
];

#[derive(Clone, Copy)]
struct StateBinding {
    id: StateId,
    value: StateValue,
}

pub fn lower(program: &oxc::ast::ast::Program<'_>) -> Result<App, CompileError> {
    let imports = validate_imports(program)?;
    let function = app_function(program)?;
    lower_function(function, &imports)
}

fn validate_imports(program: &oxc::ast::ast::Program<'_>) -> Result<HashSet<String>, CompileError> {
    let allowed = HashSet::from(INK_IMPORTS);
    let mut imported = HashSet::new();

    for statement in &program.body {
        let Statement::ImportDeclaration(declaration) = statement else {
            continue;
        };

        if declaration.source.value.as_str() != "ink" {
            return Err(CompileError::new(
                "Ink apps may only import from \"ink\" in v0",
                declaration.source.span,
            ));
        }

        let Some(specifiers) = &declaration.specifiers else {
            return Err(CompileError::new(
                "side-effect imports are not supported",
                declaration.span,
            ));
        };

        for specifier in specifiers {
            let oxc::ast::ast::ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier
            else {
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
            imported.insert(imported_name.to_owned());
        }
    }

    Ok(imported)
}

fn app_function<'a>(
    program: &'a oxc::ast::ast::Program<'a>,
) -> Result<&'a Function<'a>, CompileError> {
    let mut function = None;

    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(_) => {}
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

fn lower_function(function: &Function<'_>, imports: &HashSet<String>) -> Result<App, CompileError> {
    if function.r#async || function.generator || !function.params.items.is_empty() {
        return Err(CompileError::new(
            "the app function must be synchronous and take no arguments",
            function.span,
        ));
    }
    let body = function
        .body
        .as_ref()
        .ok_or_else(|| CompileError::new("the app function needs a body", function.span))?;
    let mut states = Vec::new();
    let mut state_names = HashMap::new();
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
                require_import(imports, "state", declaration.span)?;
                for declarator in &declaration.declarations {
                    let BindingPattern::BindingIdentifier(binding) = &declarator.id else {
                        return Err(CompileError::new(
                            "state declarations need a simple identifier",
                            declarator.id.span(),
                        ));
                    };
                    let initial = state_initialiser(declarator.init.as_ref(), declarator.span)?;
                    let name = binding.name.as_str();
                    let binding = StateBinding {
                        id: StateId(states.len()),
                        value: initial,
                    };
                    if state_names.insert(name, binding).is_some() {
                        return Err(CompileError::new(
                            format!("state {name} is declared twice"),
                            declarator.span,
                        ));
                    }
                    states.push(State { initial });
                }
            }
            Statement::ReturnStatement(statement) if root.is_none() => {
                let Some(argument) = &statement.argument else {
                    return Err(CompileError::new(
                        "the app function must return <Screen> or <Tabs>",
                        statement.span,
                    ));
                };
                let Expression::JSXElement(element) = unparenthesised(argument) else {
                    return Err(CompileError::new(
                        "the app function must return <Screen> or <Tabs>",
                        argument.span(),
                    ));
                };
                let node = lower_node(element, &state_names, imports)?;
                if !matches!(node, Node::Screen { .. } | Node::Tabs { .. }) {
                    return Err(CompileError::new(
                        "the app root must be <Screen> or <Tabs>",
                        element.span,
                    ));
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

    let root = root.ok_or_else(|| {
        CompileError::new("the app function must return <Screen> or <Tabs>", body.span)
    })?;
    Ok(App { states, root })
}

fn state_initialiser(
    initialiser: Option<&Expression<'_>>,
    span: Span,
) -> Result<StateValue, CompileError> {
    let Some(Expression::CallExpression(call)) = initialiser else {
        return Err(CompileError::new(
            "state must be initialised with state(number) or state(boolean)",
            span,
        ));
    };
    let Expression::Identifier(callee) = &call.callee else {
        return Err(CompileError::new(
            "expected state(value)",
            call.callee.span(),
        ));
    };
    if callee.name.as_str() != "state" || call.arguments.len() != 1 {
        return Err(CompileError::new("expected state(value)", call.span));
    }
    match &call.arguments[0] {
        Argument::NumericLiteral(value) => Ok(StateValue::Int(integer(
            value.value,
            value.span,
            "state value",
        )?)),
        Argument::BooleanLiteral(value) => Ok(StateValue::Bool(value.value)),
        value => Err(CompileError::new(
            "state currently accepts one integer or boolean literal",
            value.span(),
        )),
    }
}

fn lower_node(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateBinding>,
    imports: &HashSet<String>,
) -> Result<Node, CompileError> {
    let name = element_name(&element.opening_element.name)?;
    require_import(imports, name, element.opening_element.name.span())?;
    match name {
        "Screen" => lower_screen(element, states, imports),
        "Stack" => lower_stack(element, states, imports),
        "Text" => lower_text(element, states),
        "TextInput" => lower_text_input(element),
        "Button" => lower_button(element, states),
        "Icon" => lower_icon(element),
        "Image" => lower_image(element),
        "Toggle" => lower_toggle(element, states),
        "Tabs" => lower_tabs(element, states, imports),
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

fn lower_screen(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateBinding>,
    imports: &HashSet<String>,
) -> Result<Node, CompileError> {
    let title = optional_string_attribute(element, "title")?;
    let centered = boolean_attribute(element, "centered")?;
    reject_other_attributes(element, &["title", "centered"])?;
    Ok(Node::Screen {
        children: lower_element_children(element, states, imports, "Screen")?,
        title,
        centered,
    })
}

fn lower_stack(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateBinding>,
    imports: &HashSet<String>,
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
        children: lower_element_children(element, states, imports, "Stack")?,
        axis,
        gap,
        align,
        justify,
    })
}

fn lower_text(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateBinding>,
) -> Result<Node, CompileError> {
    let font_size = optional_number_attribute(element, "size")?;
    let align = text_alignment(element)?;
    reject_other_attributes(element, &["size", "align"])?;
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
                parts.push(TextPart::State(
                    state_value(&container.expression, states)?.id,
                ));
            }
            _ => {
                return Err(CompileError::new(
                    "Text supports literal text and {state.value}",
                    child.span(),
                ));
            }
        }
    }

    if parts.is_empty() {
        return Err(CompileError::new("Text cannot be empty", element.span));
    }
    Ok(Node::Text {
        parts,
        font_size,
        align,
    })
}

fn lower_text_input(element: &JSXElement<'_>) -> Result<Node, CompileError> {
    let placeholder = required_string_attribute(element, "placeholder")?;
    reject_other_attributes(element, &["placeholder"])?;
    if !element_children(element)?.is_empty() {
        return Err(CompileError::new(
            "TextInput cannot have children",
            element.span,
        ));
    }
    Ok(Node::TextInput { placeholder })
}

fn lower_button(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateBinding>,
) -> Result<Node, CompileError> {
    let action = optional_action_attribute(element, "onPress", states)?;
    let icon = optional_icon_attribute(element, "icon")?;
    let underline = boolean_attribute(element, "underline")?;
    reject_other_attributes(element, &["onPress", "icon", "underline"])?;
    let label = literal_children(element, "Button labels must be literal text")?;
    Ok(Node::Button {
        label,
        icon,
        underline,
        action,
    })
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

fn lower_image(element: &JSXElement<'_>) -> Result<Node, CompileError> {
    let source = required_string_attribute(element, "src")?;
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
    reject_other_attributes(element, &["src", "width", "height", "fit"])?;
    if !element_children(element)?.is_empty() {
        return Err(CompileError::new(
            "Image cannot have children",
            element.span,
        ));
    }
    Ok(Node::Image {
        source,
        width,
        height,
        fit,
    })
}

fn lower_toggle(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateBinding>,
) -> Result<Node, CompileError> {
    let label = required_string_attribute(element, "label")?;
    let state = state_attribute(element, "value", states, StateValue::Bool(false))?;
    let action = action_attribute(element, "onChange", states)?;
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
    states: &HashMap<&str, StateBinding>,
    imports: &HashSet<String>,
) -> Result<Node, CompileError> {
    let state = state_attribute(element, "value", states, StateValue::Int(0))?;
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

fn lower_tab(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateBinding>,
    imports: &HashSet<String>,
) -> Result<Tab, CompileError> {
    expect_element(element, "Tab")?;
    let icon = required_icon_attribute(element, "icon")?;
    let action = action_attribute(element, "onPress", states)?;
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
    expect_element(screen, "Screen")?;
    require_import(imports, "Screen", screen.span)?;
    Ok(Tab {
        icon,
        action,
        screen: Box::new(lower_screen(screen, states, imports)?),
    })
}

fn lower_element_children(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateBinding>,
    imports: &HashSet<String>,
    parent: &str,
) -> Result<Vec<Node>, CompileError> {
    element_children(element)?
        .into_iter()
        .map(|child| {
            let JSXChild::Element(child) = child else {
                return Err(CompileError::new(
                    format!("{parent} children must be Ink elements"),
                    child.span(),
                ));
            };
            lower_node(child, states, imports)
        })
        .collect()
}

fn action_attribute(
    element: &JSXElement<'_>,
    name: &str,
    states: &HashMap<&str, StateBinding>,
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
    lower_action(&function.body, states)
}

fn optional_action_attribute(
    element: &JSXElement<'_>,
    name: &str,
    states: &HashMap<&str, StateBinding>,
) -> Result<Option<Action>, CompileError> {
    if attribute(element, name).is_none() {
        Ok(None)
    } else {
        action_attribute(element, name, states).map(Some)
    }
}

fn lower_action(
    body: &ArrowFunctionBody<'_>,
    states: &HashMap<&str, StateBinding>,
) -> Result<Action, CompileError> {
    let ArrowFunctionBody::CallExpression(call) = body else {
        return Err(CompileError::new(
            "an action currently supports one state.set(...) call",
            body.span(),
        ));
    };
    let Expression::StaticMemberExpression(callee) = &call.callee else {
        return Err(CompileError::new(
            "expected state.set(...)",
            call.callee.span(),
        ));
    };
    let Expression::Identifier(state_object) = &callee.object else {
        return Err(CompileError::new(
            "expected state.set(...)",
            callee.object.span(),
        ));
    };
    if callee.property.name.as_str() != "set" || call.arguments.len() != 1 {
        return Err(CompileError::new("expected state.set(...)", call.span));
    }
    let name = state_object.name.as_str();
    let binding = states
        .get(name)
        .copied()
        .ok_or_else(|| CompileError::new(format!("unknown state {name}"), state_object.span))?;

    match &call.arguments[0] {
        Argument::NumericLiteral(value) if matches!(binding.value, StateValue::Int(_)) => {
            Ok(Action::SetInt {
                state: binding.id,
                value: integer(value.value, value.span, "state value")?,
            })
        }
        Argument::BooleanLiteral(value) if matches!(binding.value, StateValue::Bool(_)) => {
            Ok(Action::SetBool {
                state: binding.id,
                value: value.value,
            })
        }
        Argument::BinaryExpression(value) if matches!(binding.value, StateValue::Int(_)) => {
            let read_state = expression_state_value(&value.left, states)?;
            if read_state.id != binding.id {
                return Err(CompileError::new(
                    "an action must update the state value it reads",
                    value.left.span(),
                ));
            }
            let Expression::NumericLiteral(amount) = &value.right else {
                return Err(CompileError::new(
                    "the increment must be an integer literal",
                    value.right.span(),
                ));
            };
            let mut by = integer(amount.value, amount.span, "increment")?;
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
            if value.operator == UnaryOperator::LogicalNot
                && matches!(binding.value, StateValue::Bool(_)) =>
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

fn state_attribute<'a>(
    element: &JSXElement<'a>,
    name: &str,
    states: &HashMap<&str, StateBinding>,
    expected: StateValue,
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
    if std::mem::discriminant(&binding.value) != std::mem::discriminant(&expected) {
        return Err(CompileError::new(
            format!("{name} has the wrong state type"),
            container.span,
        ));
    }
    Ok(binding)
}

fn state_value(
    expression: &JSXExpression<'_>,
    states: &HashMap<&str, StateBinding>,
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
    states: &HashMap<&str, StateBinding>,
) -> Result<StateBinding, CompileError> {
    let Expression::StaticMemberExpression(member) = expression else {
        return Err(CompileError::new("expected state.value", expression.span()));
    };
    member_state_value(member, states)
}

fn member_state_value(
    member: &oxc::ast::ast::StaticMemberExpression<'_>,
    states: &HashMap<&str, StateBinding>,
) -> Result<StateBinding, CompileError> {
    let Expression::Identifier(object) = &member.object else {
        return Err(CompileError::new(
            "expected state.value",
            member.object.span(),
        ));
    };
    if member.property.name.as_str() != "value" {
        return Err(CompileError::new("expected state.value", member.span));
    }
    states.get(object.name.as_str()).copied().ok_or_else(|| {
        CompileError::new(
            format!("unknown state {}", object.name.as_str()),
            object.span,
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
            format!("unknown Material icon {value:?}"),
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

fn literal_children(element: &JSXElement<'_>, error: &str) -> Result<String, CompileError> {
    let label = element
        .children
        .iter()
        .map(|child| match child {
            JSXChild::Text(text) => Ok(normalise_text(text.value.as_str())),
            _ => Err(CompileError::new(error, child.span())),
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(" ")
        .trim()
        .to_owned();
    if label.is_empty() {
        return Err(CompileError::new("text cannot be empty", element.span));
    }
    Ok(label)
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

fn require_import(imports: &HashSet<String>, name: &str, span: Span) -> Result<(), CompileError> {
    if imports.contains(name) {
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
