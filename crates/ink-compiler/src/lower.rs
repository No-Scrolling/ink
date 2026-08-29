use std::collections::{HashMap, HashSet};

use oxc::{
    ast::ast::{
        Argument, ArrowFunctionBody, BindingPattern, ExportDefaultDeclarationKind, Expression,
        Function, JSXAttribute, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild,
        JSXElement, JSXElementName, JSXExpression, Statement, VariableDeclarationKind,
    },
    span::{GetSpan, Span},
    syntax::operator::BinaryOperator,
};

use crate::{
    diagnostic::CompileError,
    ir::{Action, App, Node, State, StateId, TextPart},
};

const INK_IMPORTS: [&str; 5] = ["Button", "Column", "Screen", "Text", "state"];

pub fn lower(program: &oxc::ast::ast::Program<'_>) -> Result<App, CompileError> {
    validate_imports(program)?;
    let function = app_function(program)?;
    lower_function(function)
}

fn validate_imports(program: &oxc::ast::ast::Program<'_>) -> Result<(), CompileError> {
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
                    format!("{imported_name} is not an Ink v0 primitive"),
                    specifier.span,
                ));
            }
            imported.insert(imported_name);
        }
    }

    for required in INK_IMPORTS {
        if !imported.contains(required) {
            return Err(CompileError::new(
                format!("missing {required} import from \"ink\""),
                program.span,
            ));
        }
    }

    Ok(())
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

fn lower_function(function: &Function<'_>) -> Result<App, CompileError> {
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
                for declarator in &declaration.declarations {
                    let BindingPattern::BindingIdentifier(binding) = &declarator.id else {
                        return Err(CompileError::new(
                            "state declarations need a simple identifier",
                            declarator.id.span(),
                        ));
                    };
                    let initial = state_initialiser(declarator.init.as_ref(), declarator.span)?;
                    let name = binding.name.as_str();
                    let id = StateId(states.len());
                    if state_names.insert(name, id).is_some() {
                        return Err(CompileError::new(
                            format!("state {name} is declared twice"),
                            binding.span,
                        ));
                    }
                    states.push(State { initial });
                }
            }
            Statement::ReturnStatement(statement) if root.is_none() => {
                let Some(argument) = &statement.argument else {
                    return Err(CompileError::new(
                        "the app function must return <Screen>",
                        statement.span,
                    ));
                };
                let Expression::JSXElement(element) = unparenthesised(argument) else {
                    return Err(CompileError::new(
                        "the app function must return <Screen>",
                        argument.span(),
                    ));
                };
                root = Some(lower_screen(element, &state_names)?);
            }
            _ => {
                return Err(CompileError::new(
                    "the app body may contain state declarations followed by one return",
                    statement.span(),
                ));
            }
        }
    }

    let root =
        root.ok_or_else(|| CompileError::new("the app function must return <Screen>", body.span))?;
    Ok(App { states, root })
}

fn state_initialiser(
    initialiser: Option<&Expression<'_>>,
    span: Span,
) -> Result<i64, CompileError> {
    let Some(Expression::CallExpression(call)) = initialiser else {
        return Err(CompileError::new(
            "state must be initialised with state(integer)",
            span,
        ));
    };
    let Expression::Identifier(callee) = &call.callee else {
        return Err(CompileError::new(
            "expected state(integer)",
            call.callee.span(),
        ));
    };
    if callee.name.as_str() != "state" || call.arguments.len() != 1 {
        return Err(CompileError::new("expected state(integer)", call.span));
    }
    let Argument::NumericLiteral(value) = &call.arguments[0] else {
        return Err(CompileError::new(
            "state currently accepts one integer literal",
            call.arguments[0].span(),
        ));
    };
    integer(value.value, value.span, "state value")
}

fn lower_screen(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateId>,
) -> Result<Node, CompileError> {
    expect_element(element, "Screen")?;
    if !element.opening_element.attributes.is_empty() {
        return Err(CompileError::new(
            "Screen does not accept props in v0",
            element.opening_element.span,
        ));
    }
    let children = element_children(element)?;
    if children.len() != 1 {
        return Err(CompileError::new(
            "Screen must contain exactly one root element",
            element.span,
        ));
    }
    let JSXChild::Element(child) = children[0] else {
        return Err(CompileError::new(
            "Screen's root child must be an element",
            children[0].span(),
        ));
    };
    lower_node(child.as_ref(), states)
}

fn lower_node(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateId>,
) -> Result<Node, CompileError> {
    match element_name(&element.opening_element.name)? {
        "Column" => lower_column(element, states),
        "Text" => lower_text(element, states),
        "Button" => lower_button(element, states),
        name => Err(CompileError::new(
            format!("{name} cannot appear here"),
            element.opening_element.name.span(),
        )),
    }
}

fn lower_column(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateId>,
) -> Result<Node, CompileError> {
    let gap = optional_number_attribute(element, "gap")?;
    reject_other_attributes(element, &["gap"])?;
    let mut children = Vec::new();
    for child in element_children(element)? {
        let JSXChild::Element(child) = child else {
            return Err(CompileError::new(
                "Column children must be Ink elements",
                child.span(),
            ));
        };
        children.push(lower_node(child.as_ref(), states)?);
    }
    Ok(Node::Column { children, gap })
}

fn lower_text(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateId>,
) -> Result<Node, CompileError> {
    let font_size = optional_number_attribute(element, "size")?;
    reject_other_attributes(element, &["size"])?;
    let mut parts = Vec::new();

    for child in &element.children {
        match child {
            JSXChild::Text(text) => {
                let value = normalise_text(text.value.as_str());
                if !value.is_empty() {
                    parts.push(TextPart::Literal(value));
                }
            }
            JSXChild::ExpressionContainer(container) => {
                parts.push(TextPart::State(state_value(&container.expression, states)?));
            }
            _ => {
                return Err(CompileError::new(
                    "Text supports literal text and {state.value} in v0",
                    child.span(),
                ));
            }
        }
    }

    if parts.is_empty() {
        return Err(CompileError::new("Text cannot be empty", element.span));
    }
    Ok(Node::Text { parts, font_size })
}

fn lower_button(
    element: &JSXElement<'_>,
    states: &HashMap<&str, StateId>,
) -> Result<Node, CompileError> {
    reject_other_attributes(element, &["onPress"])?;
    let on_press = attribute(element, "onPress").ok_or_else(|| {
        CompileError::new("Button requires onPress", element.opening_element.span)
    })?;
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &on_press.value else {
        return Err(CompileError::new(
            "onPress must be an arrow function",
            on_press.span,
        ));
    };
    let JSXExpression::ArrowFunctionExpression(function) = &container.expression else {
        return Err(CompileError::new(
            "onPress must be an arrow function",
            container.span,
        ));
    };
    if function.r#async || !function.params.items.is_empty() {
        return Err(CompileError::new(
            "onPress must be a synchronous zero-argument arrow function",
            function.span,
        ));
    }
    let action = lower_action(&function.body, states)?;
    let label = element
        .children
        .iter()
        .map(|child| match child {
            JSXChild::Text(text) => Ok(normalise_text(text.value.as_str())),
            _ => Err(CompileError::new(
                "Button labels must be literal text",
                child.span(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(" ")
        .trim()
        .to_owned();
    if label.is_empty() {
        return Err(CompileError::new("Button needs a label", element.span));
    }
    Ok(Node::Button { label, action })
}

fn lower_action(
    body: &ArrowFunctionBody<'_>,
    states: &HashMap<&str, StateId>,
) -> Result<Action, CompileError> {
    let ArrowFunctionBody::CallExpression(call) = body else {
        return Err(CompileError::new(
            "onPress currently supports one state.set(...) call",
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
    let state = states
        .get(name)
        .copied()
        .ok_or_else(|| CompileError::new(format!("unknown state {name}"), state_object.span))?;
    let Argument::BinaryExpression(value) = &call.arguments[0] else {
        return Err(CompileError::new(
            "state.set currently accepts state.value + integer",
            call.arguments[0].span(),
        ));
    };
    let read_state = expression_state_value(&value.left, states)?;
    if read_state != state {
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
                "state updates support + and - in v0",
                value.span,
            ));
        }
    }
    Ok(Action::Increment { state, by })
}

fn state_value(
    expression: &JSXExpression<'_>,
    states: &HashMap<&str, StateId>,
) -> Result<StateId, CompileError> {
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
    states: &HashMap<&str, StateId>,
) -> Result<StateId, CompileError> {
    let Expression::StaticMemberExpression(member) = expression else {
        return Err(CompileError::new("expected state.value", expression.span()));
    };
    member_state_value(member, states)
}

fn member_state_value(
    member: &oxc::ast::ast::StaticMemberExpression<'_>,
    states: &HashMap<&str, StateId>,
) -> Result<StateId, CompileError> {
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
                    "{} does not accept {name} in v0",
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
            format!("expected <{expected}>, found <{actual}>",),
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
    let core = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if core.is_empty() {
        return core;
    }

    let leading_space = value.chars().next().is_some_and(char::is_whitespace);
    let trailing_space = value.chars().next_back().is_some_and(char::is_whitespace);
    format!(
        "{}{}{}",
        if leading_space { " " } else { "" },
        core,
        if trailing_space { " " } else { "" },
    )
}

fn unparenthesised<'a>(mut expression: &'a Expression<'a>) -> &'a Expression<'a> {
    while let Expression::ParenthesizedExpression(parenthesised) = expression {
        expression = &parenthesised.expression;
    }
    expression
}
