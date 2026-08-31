use crate::ir::{StateLiteral, StateShape};

pub fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut value = 0xcbf29ce484222325_u64;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x100000001b3);
    }
    value
}

pub fn state_shape_name(shape: &StateShape) -> String {
    match shape {
        StateShape::Null => "null".to_owned(),
        StateShape::Number => "number".to_owned(),
        StateShape::Bool => "bool".to_owned(),
        StateShape::String => "string".to_owned(),
        StateShape::Literal(StateLiteral::Number(value)) => format!("literal-number:{value}"),
        StateShape::Literal(StateLiteral::Bool(value)) => format!("literal-bool:{value}"),
        StateShape::Literal(StateLiteral::String(value)) => format!("literal-string:{value}"),
        StateShape::Optional(shape) => format!("optional<{}>", state_shape_name(shape)),
        StateShape::Union(shapes) => format!(
            "union<{}>",
            shapes
                .iter()
                .map(state_shape_name)
                .collect::<Vec<_>>()
                .join("|")
        ),
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
