package com.vandam.ink

import org.json.JSONObject

internal object DevelopmentErrors {
    fun map(message: String, readMap: () -> String): String = runCatching {
        val map = JSONObject(readMap())
        val sources = map.getJSONArray("sources")
        val lines = map.getString("mappings").split(';')
        Regex("(?:app\\.js|eval_script|<eval>):([0-9]+)(?::([0-9]+))?").replace(message) { match ->
            val line = match.groupValues[1].toInt() - 1
            val column = match.groupValues[2].toIntOrNull()?.minus(1) ?: 0
            var source = 0
            var originalLine = 0
            var originalColumn = 0
            var location: String? = null
            for (index in 0..minOf(line, lines.lastIndex)) {
                var generatedColumn = 0
                for (segment in lines[index].split(',')) {
                    if (segment.isEmpty()) continue
                    val values = decode(segment)
                    generatedColumn += values[0]
                    if (values.size < 4) continue
                    source += values[1]
                    originalLine += values[2]
                    originalColumn += values[3]
                    if (index == line && generatedColumn <= column) {
                        location = "${sources.getString(source)}:${originalLine + 1}:${originalColumn + 1}"
                    }
                }
            }
            location ?: match.value
        }
    }.getOrDefault(message)

    private fun decode(segment: String): List<Int> {
        val alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        val values = mutableListOf<Int>()
        var value = 0
        var shift = 0
        for (character in segment) {
            val digit = alphabet.indexOf(character)
            require(digit >= 0)
            value = value or ((digit and 31) shl shift)
            if (digit and 32 == 0) {
                values.add(if (value and 1 != 0) -(value ushr 1) else value ushr 1)
                value = 0
                shift = 0
            } else shift += 5
        }
        require(shift == 0)
        return values
    }
}
