package com.vandam.ink

import android.os.Handler
import android.os.Looper
import android.view.textservice.SentenceSuggestionsInfo
import android.view.textservice.SpellCheckerSession
import android.view.textservice.SuggestionsInfo
import android.view.textservice.TextInfo
import android.view.textservice.TextServicesManager
import com.vandam.ink.keyboard.InkKeyboardView
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale

internal fun createSpelling(activity: MainActivity, onEdit: TextEditHandler, keyboard: InkKeyboardView): Spelling =
    DeviceSpelling(activity, onEdit, keyboard)

private data class InputContext(val id: Int, val text: String, val cursor: Int, val correct: Boolean, val check: Boolean)
private data class Word(val start: Int, val end: Int, val text: String)
private data class CheckedWord(val word: Word, val suggestions: List<String>, val recommended: Boolean)
private data class Correction(val context: InputContext, val word: Word)
private data class Undo(val context: InputContext, val start: Int, val end: Int, val original: String)

private class DeviceSpelling(
    private val activity: MainActivity,
    private val onEdit: TextEditHandler,
    private val keyboard: InkKeyboardView,
) : Spelling, SpellCheckerSession.SpellCheckerSessionListener {
    private val handler = Handler(Looper.getMainLooper())
    private var session: SpellCheckerSession? = null
    private var context: InputContext? = null
    private var requested: InputContext? = null
    private var words = emptyList<Word>()
    private var checked = emptyList<CheckedWord>()
    private var generation = 0
    private var pending: Correction? = null
    private var undo: Undo? = null
    private var selected: CheckedWord? = null
    private val rejected = mutableSetOf<String>()
    private val check = Runnable { request() }
    private val wordPattern = Regex("[\\p{L}][\\p{L}\\p{M}'’]*")

    init {
        keyboard.onDismissSuggestions = { dismiss() }
        keyboard.onSuggestion = { suggestion ->
            val current = context
            val word = selected?.word
            if (current != null && word != null) replace(current, word.start, word.end, suggestion)
            dismiss()
        }
    }

    override fun sync(context: String) {
        if (context.isEmpty()) {
            close()
            return
        }
        val data = JSONObject(context)
        val next = InputContext(data.getInt("id"), data.getString("text"), data.getInt("cursor"),
            data.getBoolean("autoCorrect"), data.getBoolean("spellCheck"))
        val previous = this.context
        this.context = next
        if (!next.correct && !next.check) {
            close()
            this.context = next
            return
        }
        if (previous != next) {
            selected = null
            keyboard.suggestions = emptyList()
            if (undo?.context != next) undo = null
            if (pending?.context != next) pending = null
            if (previous?.id != next.id || previous.text != next.text || previous.check != next.check || previous.correct != next.correct) {
                handler.removeCallbacks(check)
                if (previous?.id != next.id) rejected.clear()
                checked = emptyList()
                requested = null
                generation++
                handler.postDelayed(check, 200)
            }
        }
        if (!data.isNull("selected")) {
            val start = data.getInt("selected")
            selected = checked.find { it.word.start == start }
            keyboard.suggestions = selected?.suggestions.orEmpty()
        } else {
            selected = null
            keyboard.suggestions = emptyList()
        }
    }

    override fun insert(text: String) {
        val before = context
        undo = null
        dismiss()
        onEdit(TextEdit.Insert(text))
        val after = context ?: return
        if (before == null || !before.correct || text.length != 1 || !(text[0].isWhitespace() || text[0] in ".,!?;:")) return
        if (after.text != before.text.substring(0, before.cursor) + text + before.text.substring(before.cursor)) return
        val match = wordPattern.findAll(before.text).lastOrNull { it.range.last + 1 == before.cursor } ?: return
        val token = before.text.substring(0, before.cursor).takeLastWhile { !it.isWhitespace() }
        if (token.any { it in "@/.:" || it.isDigit() }) return
        pending = Correction(after, Word(match.range.first, before.cursor, match.value))
        handler.removeCallbacks(check)
        request()
    }

    override fun backspace() {
        dismiss()
        val last = undo
        undo = null
        if (last != null && context == last.context) {
            rejected += last.original
            replace(last.context, last.start, last.end, last.original)
        } else {
            onEdit(TextEdit.Backspace)
        }
    }

    override fun dismiss(): Boolean {
        val shown = keyboard.suggestions.isNotEmpty()
        selected = null
        keyboard.suggestions = emptyList()
        if (shown) context?.let { onEdit(TextEdit.Assistance(payload(it).put("dismiss", true).toString())) }
        return shown
    }

    override fun close() {
        handler.removeCallbacks(check)
        session?.close()
        session = null
        context = null
        requested = null
        pending = null
        undo = null
        checked = emptyList()
        words = emptyList()
        rejected.clear()
        generation++
        dismiss()
    }

    private fun request() {
        val current = context ?: return
        if (!current.correct && !current.check) return
        if (requested?.id == current.id && requested?.text == current.text) return
        if (session == null) {
            session = runCatching {
                activity.getSystemService(TextServicesManager::class.java)
                    .newSpellCheckerSession(null, Locale.getDefault(), this, true)
            }.getOrNull()
        }
        val active = session ?: return
        words = wordPattern.findAll(current.text).map { Word(it.range.first, it.range.last + 1, it.value) }.toList()
        requested = current
        generation++
        if (words.isEmpty()) {
            publish(current, emptyList())
            return
        }
        // Check words individually: the device service can treat trailing punctuation as part of a word.
        runCatching {
            active.getSentenceSuggestions(words.mapIndexed { index, word -> TextInfo(word.text, generation, index) }.toTypedArray(), 3)
        }.onFailure { close() }
    }

    override fun onGetSuggestions(results: Array<out SuggestionsInfo>) = Unit

    override fun onGetSentenceSuggestions(results: Array<out SentenceSuggestionsInfo>) {
        if (results.isEmpty()) return
        val request = requested ?: return
        val current = context ?: return
        if (request.id != current.id || request.text != current.text) return
        val found = mutableListOf<CheckedWord>()
        for (sentence in results) {
            for (index in 0 until sentence.suggestionsCount) {
                val info = sentence.getSuggestionsInfoAt(index)
                if (info.cookie != generation) return
                val word = words.getOrNull(info.sequence) ?: continue
                if (sentence.getOffsetAt(index) != 0 || sentence.getLengthAt(index) != word.text.length) continue
                if (info.suggestionsAttributes and SuggestionsInfo.RESULT_ATTR_LOOKS_LIKE_TYPO == 0) continue
                val options = (0 until info.suggestionsCount).map { info.getSuggestionAt(it) }
                    .filter { it.isNotBlank() && it.none(Char::isISOControl) && it != word.text }.distinct().take(3)
                if (options.isNotEmpty()) found += CheckedWord(word, options,
                    info.suggestionsAttributes and SuggestionsInfo.RESULT_ATTR_HAS_RECOMMENDED_SUGGESTIONS != 0)
            }
        }
        checked = found
        val correction = pending
        pending = null
        val candidate = found.find { it.word == correction?.word }
        if (correction?.context == current && candidate != null && safeCorrection(candidate)) {
            val replacement = candidate.suggestions.first()
            replace(current, candidate.word.start, candidate.word.end, replacement)
            val updated = context
            if (updated != null && updated.text == current.text.replaceRange(candidate.word.start, candidate.word.end, replacement)) {
                undo = Undo(updated, candidate.word.start, candidate.word.start + replacement.length + 1, candidate.word.text)
            }
        } else {
            publish(current, found)
        }
    }

    private fun publish(current: InputContext, found: List<CheckedWord>) {
        val ranges = JSONArray()
        if (current.check) found.forEach { ranges.put(JSONArray(listOf(it.word.start, it.word.end))) }
        onEdit(TextEdit.Assistance(payload(current).put("ranges", ranges).toString()))
    }

    private fun replace(current: InputContext, start: Int, end: Int, replacement: String) {
        onEdit(TextEdit.Assistance(payload(current).put("cursor", current.cursor)
            .put("start", start).put("end", end).put("replacement", replacement).toString()))
    }

    private fun payload(current: InputContext) = JSONObject().put("id", current.id).put("text", current.text)

    private fun safeCorrection(candidate: CheckedWord): Boolean {
        if (candidate.word.text in rejected || !candidate.recommended || candidate.word.text.length !in 3..48) return false
        val original = candidate.word.text
        if (original.any(Char::isUpperCase)) return false
        val costs = candidate.suggestions.map { distance(original, it) }
        return costs.first() <= 1.0 && costs.drop(1).all { it > costs.first() }
    }

    // Transposed adjacent letters are a stronger typo signal than an equally close alternative word.
    private fun distance(a: String, b: String): Double {
        if (b.length > 50) return Double.POSITIVE_INFINITY
        val d = Array(a.length + 1) { i ->
            DoubleArray(b.length + 1) { j ->
                when {
                    i == 0 -> j.toDouble()
                    j == 0 -> i.toDouble()
                    else -> 0.0
                }
            }
        }
        for (i in 1..a.length) for (j in 1..b.length) {
            d[i][j] = minOf(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + if (a[i - 1] == b[j - 1]) 0.0 else 1.0)
            if (i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1]) {
                d[i][j] = minOf(d[i][j], d[i - 2][j - 2] + 0.5)
            }
        }
        return d[a.length][b.length]
    }
}
