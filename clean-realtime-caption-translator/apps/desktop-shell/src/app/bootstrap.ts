import { loadSettings } from '../services/settings/settingsManager';
import { settingsStore } from '../stores/settings/settingsStore';
import { bindSystemSubtitleEvents } from '../hooks/system-subtitle/useSystemSubtitleEvents';
import { bindMicInterpretationEvents } from '../hooks/mic-interpretation/useMicInterpretationEvents';
import { logDiagnostic } from '../services/diagnostics/diagnosticsLogger';

export async function bootstrapApp(): Promise<void> {
  try {
    settingsStore.settings = await loadSettings();
    logDiagnostic('[CONFIG]', {
      localAsrProvider: settingsStore.settings.localAsr.provider,
      localAsrWsUrl: settingsStore.settings.localAsr.wsUrl,
      localTranslationConfigured: Boolean(settingsStore.settings.localTranslation.endpoint || settingsStore.settings.localTranslation.modelPath),
      qwenTtsModel: settingsStore.settings.qwenTts.model,
      qwenTtsHasApiKey: settingsStore.settings.qwenTts.hasApiKey,
    });
  } catch (error) {
    settingsStore.error = error instanceof Error ? error.message : 'Failed to load settings';
  }

  await bindSystemSubtitleEvents();
  await bindMicInterpretationEvents();
}
