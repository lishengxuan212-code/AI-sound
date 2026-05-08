import { describe, expect, it } from 'vitest';
import { redactSecrets } from './secretRedactor';

describe('redactSecrets', () => {
  it('redacts authorization, bearer tokens, and key-like fields', () => {
    const input = {
      Authorization: 'Bearer abc123',
      DASHSCOPE_API_KEY: 'dashscope-secret',
      nested: {
        game_tts_api_key: 'tts-secret',
        password: 'pw',
        ok: 'visible',
      },
      text: 'Authorization: Bearer leak',
    };

    expect(JSON.stringify(redactSecrets(input))).not.toContain('abc123');
    expect(JSON.stringify(redactSecrets(input))).not.toContain('dashscope-secret');
    expect(JSON.stringify(redactSecrets(input))).not.toContain('tts-secret');
    expect(JSON.stringify(redactSecrets(input))).not.toContain('Bearer leak');
    expect(redactSecrets(input).nested.ok).toBe('visible');
  });
});
