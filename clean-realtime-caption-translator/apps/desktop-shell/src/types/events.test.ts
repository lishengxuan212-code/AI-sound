import { describe, expect, it } from 'vitest';
import { assertSessionEvent, SessionKind } from './events';

describe('assertSessionEvent', () => {
  it('rejects events without sessionKind', () => {
    expect(() => assertSessionEvent({ type: 'bad' })).toThrow(/sessionKind/);
  });

  it('accepts explicit SystemSubtitle and MicInterpretation events', () => {
    expect(assertSessionEvent({ type: 'ok', sessionKind: SessionKind.SystemSubtitle })).toBe(true);
    expect(assertSessionEvent({ type: 'ok', sessionKind: SessionKind.MicInterpretation })).toBe(true);
  });
});
