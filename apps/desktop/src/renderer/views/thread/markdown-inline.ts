/** Markdown inline tokens; escapes stay intact until a text node is emitted. */
export const inlinePattern = /(?<!\\)\[((?:\\.|[^\]\\])+)\]\(([^)\s]+)\)|(?<!\\)\*\*((?:\\.|[^*\\])+)\*\*|(?<!\\)__((?:\\.|[^_\\])+)__|(?<!\\)~~((?:\\.|[^~\\])+)~~|(?<!\\)\*((?:\\.|[^*\\\n])+)\*|(?<![\w\\])_((?:\\.|[^_\\\n])+?)_(?![\w])|(?<!\\)`([^`\n]+)`/

export function splitCodeSpans(text: string): string[] {
  return text.split(/((?<!\\)`[^`\n]+`)/)
}

/** Unescape only punctuation recognized by CommonMark, never arbitrary backslashes. */
export function unescapeInline(text: string): string {
  return text.replace(/\\([!"#$%&'()*+,\-./:;<=>?@[\\\]^_`{|}~])/g, '$1')
}
