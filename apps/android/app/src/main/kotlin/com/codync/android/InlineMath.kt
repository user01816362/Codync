package com.codync.android

/** Plain prose plus formulas only; retain native Markdown for styled, code and link paragraphs. */
internal object InlineMath {
    private val expression = Regex("""\\\(([^\n]{1,1024}?)\\\)|(?<!\\)\$([^$\n]{1,1024}?)\$""")
    fun eligible(text: String): Boolean {
        if (text.length > 4000 || text.contains("$$")) return false
        var found = false
        val rest = expression.replace(text.replace(Regex("""`[^`\n]*`"""), "")) { match ->
            val explicit = match.groups[1]?.value
            val math = explicit ?: match.groups[2]?.value.orEmpty()
            val valid = explicit != null || (math.trim() == math &&
                (Regex("""[\\^_=+*/{}]""").containsMatchIn(math) || Regex("""^[A-Za-z]$""").matches(math)))
            if (valid) { found = true; "" } else match.value
        }
        return found && !Regex("""[\[\]<>\\]""").containsMatchIn(rest)
    }
}
