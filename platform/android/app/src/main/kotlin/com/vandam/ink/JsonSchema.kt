package com.vandam.ink

import org.json.JSONArray
import org.json.JSONObject

internal fun validInkJson(schema: Any, value: Any): Boolean = when (schema) {
    "number" -> value is Number && value.toDouble().isFinite()
    "boolean" -> value is Boolean
    "string" -> value is String
    is JSONObject -> when {
        schema.has("null") -> value === JSONObject.NULL
        schema.has("literal") -> literalEquals(schema.get("literal"), value)
        schema.has("optional") -> {
            value === JSONObject.NULL || validInkJson(schema.get("optional"), value)
        }
        schema.has("oneOf") -> {
            val variants = schema.getJSONArray("oneOf")
            (0 until variants.length()).any { validInkJson(variants.get(it), value) }
        }
        schema.has("array") -> {
            value is JSONArray && (0 until value.length()).all {
                validInkJson(schema.get("array"), value.get(it))
            }
        }
        else -> value is JSONObject && schema.keys().asSequence().all { name ->
            val field = schema.get(name)
            if (field is JSONObject && field.has("optional")) {
                !value.has(name) || validInkJson(field, value.get(name))
            } else {
                value.has(name) && validInkJson(field, value.get(name))
            }
        }
    }
    else -> false
}

private fun literalEquals(expected: Any, value: Any): Boolean = when {
    expected is Number && value is Number -> expected.toDouble() == value.toDouble()
    else -> expected == value
}
