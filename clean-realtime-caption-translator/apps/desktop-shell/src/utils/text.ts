export function compactText(text: string): string {
  return text.replace(/\s+/g, ' ').trim();
}

export function countWords(text: string): number {
  const latinWords = text.match(/[A-Za-z0-9]+(?:'[A-Za-z0-9]+)?/g)?.length ?? 0;
  const cjkChars = text.match(/[\u4e00-\u9fff]/g)?.length ?? 0;
  return latinWords + cjkChars;
}

export function hasStrongSentenceEnd(text: string): boolean {
  return /[.!?;。！？；]\s*$/.test(text.trim());
}
