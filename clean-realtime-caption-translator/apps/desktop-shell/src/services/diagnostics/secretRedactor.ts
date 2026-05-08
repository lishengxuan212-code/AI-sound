const SECRET_KEY_PATTERN = /(authorization|bearer|api[_-]?key|dashscope_api_key|game_tts_api_key|token|secret|password)/i;
const BEARER_PATTERN = /Bearer\s+[A-Za-z0-9._~+/=-]+/gi;
const AUTH_LINE_PATTERN = /(Authorization\s*[:=]\s*)([^\s,;}]+)/gi;

export type Redacted<T> = T;

export function redactSecrets<T>(value: T): Redacted<T> {
  if (typeof value === 'string') {
    return value.replace(BEARER_PATTERN, 'Bearer [REDACTED]').replace(AUTH_LINE_PATTERN, '$1[REDACTED]') as Redacted<T>;
  }
  if (Array.isArray(value)) {
    return value.map((item) => redactSecrets(item)) as Redacted<T>;
  }
  if (value && typeof value === 'object') {
    const result: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value)) {
      result[key] = SECRET_KEY_PATTERN.test(key) ? '[REDACTED]' : redactSecrets(item);
    }
    return result as Redacted<T>;
  }
  return value;
}
