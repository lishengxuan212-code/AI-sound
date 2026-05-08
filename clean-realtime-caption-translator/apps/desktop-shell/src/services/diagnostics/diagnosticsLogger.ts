import { redactSecrets } from './secretRedactor';

export type LogPrefix =
  | '[SYS][AUDIO]'
  | '[SYS][ASR_PARTIAL]'
  | '[SYS][ASR_FINAL]'
  | '[SYS][TRANSLATION]'
  | '[SYS][SEGMENT]'
  | '[MIC][AUDIO]'
  | '[MIC][ASR_PARTIAL]'
  | '[MIC][ASR_FINAL]'
  | '[MIC][TRANSLATION]'
  | '[MIC][TTS]'
  | '[CONFIG]'
  | '[SECRET_REDACTED]';

export function logDiagnostic(prefix: LogPrefix, details: Record<string, unknown>): void {
  const safe = redactSecrets(details);
  console.info(prefix, safe);
}
