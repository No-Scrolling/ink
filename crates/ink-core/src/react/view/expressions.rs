use super::*;

#[derive(Clone, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)]
pub(super) enum Expression {
    Literal { value: Json },
    Value { source: usize, path: Vec<Json> },
    Add { args: [Box<Expression>; 2] },
    Subtract { args: [Box<Expression>; 2] },
    Multiply { args: [Box<Expression>; 2] },
    Divide { args: [Box<Expression>; 2] },
    Remainder { args: [Box<Expression>; 2] },
    Less { args: [Box<Expression>; 2] },
    Equal { args: [Box<Expression>; 2] },
    Not { value: Box<Expression> },
    Choose { condition: Box<Expression>, yes: Box<Expression>, no: Box<Expression> },
    Concat { parts: Vec<Expression> },
    Length { value: Box<Expression> },
    Pad { value: Box<Expression>, width: usize },
}

fn string(value: Json) -> Result<String> {
    Ok(match value {
        Json::String(value) => value,
        Json::Number(value) => value.as_f64().context("invalid expression number")?.to_string(),
        Json::Bool(value) => value.to_string(),
        Json::Null => "null".into(),
        _ => bail!("expression requires a primitive value"),
    })
}

impl Expression {
    pub(super) fn dependencies(&self, dependencies: &mut FxHashSet<usize>, depth: usize) -> Result<()> {
        ensure!(depth < 64, "expression is too deep");
        match self {
            Self::Value { source, .. } => { dependencies.insert(*source); }
            Self::Literal { .. } => {}
            Self::Not { value } | Self::Length { value } | Self::Pad { value, .. } => value.dependencies(dependencies, depth + 1)?,
            Self::Choose { condition, yes, no } => {
                for value in [condition, yes, no] { value.dependencies(dependencies, depth + 1)?; }
            }
            Self::Concat { parts } => {
                ensure!(parts.len() <= 1024, "expression has too many parts");
                for value in parts { value.dependencies(dependencies, depth + 1)?; }
            }
            Self::Add { args } | Self::Subtract { args } | Self::Multiply { args } | Self::Divide { args }
                | Self::Remainder { args } | Self::Less { args } | Self::Equal { args } => {
                for value in args { value.dependencies(dependencies, depth + 1)?; }
            }
        }
        Ok(())
    }

    pub(super) fn evaluate(&self, view: &BoundView) -> Result<Json> {
        Ok(match self {
            Self::Literal { value } => value.clone(),
            Self::Value { source, path } => view.selected(*source, path)?.into_owned(),
            Self::Length { value } => {
                // Collection lengths never materialise the retained records.
                if let Self::Value { source, path } = &**value {
                    if let Some(collection) = view.collections.get(source).filter(|_| path.is_empty()) {
                        return Ok(json!(collection.keys.len()));
                    }
                }
                match value.evaluate(view)? {
                    Json::String(value) => json!(value.encode_utf16().count()),
                    Json::Array(value) => json!(value.len()),
                    _ => bail!("length requires a string or array"),
                }
            }
            Self::Not { value } => json!(!value.evaluate(view)?.as_bool().context("not requires a boolean")?),
            Self::Choose { condition, yes, no } => {
                if condition.evaluate(view)?.as_bool().context("condition requires a boolean")? { yes.evaluate(view)? }
                else { no.evaluate(view)? }
            }
            Self::Concat { parts } => {
                let mut text = String::new();
                for part in parts { text.push_str(&string(part.evaluate(view)?)?); }
                json!(text)
            }
            Self::Pad { value, width } => {
                ensure!(*width <= 1024, "padding width is too large");
                let value = string(value.evaluate(view)?)?;
                json!(format!("{}{}", "0".repeat(width.saturating_sub(value.encode_utf16().count())), value))
            }
            Self::Equal { args } => {
                let left = args[0].evaluate(view)?;
                let right = args[1].evaluate(view)?;
                json!(if left.is_number() && right.is_number() { left.as_f64() == right.as_f64() } else { left == right })
            }
            Self::Add { args } | Self::Subtract { args } | Self::Multiply { args } | Self::Divide { args }
                | Self::Remainder { args } | Self::Less { args } => {
                let left = args[0].evaluate(view)?.as_f64().context("arithmetic requires numbers")?;
                let right = args[1].evaluate(view)?.as_f64().context("arithmetic requires numbers")?;
                let result = match self {
                    Self::Add { .. } => left + right,
                    Self::Subtract { .. } => left - right,
                    Self::Multiply { .. } => left * right,
                    Self::Divide { .. } => left / right,
                    Self::Remainder { .. } => left % right,
                    Self::Less { .. } => return Ok(json!(left < right)),
                    _ => unreachable!(),
                };
                ensure!(result.is_finite(), "expression result must be finite");
                json!(result)
            }
        })
    }
}
