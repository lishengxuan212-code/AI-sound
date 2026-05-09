import { useSystemSubtitle } from '../hooks/system-subtitle/useSystemSubtitle';
import { logDiagnostic } from '../services/diagnostics/diagnosticsLogger';
import { startCaptionSnapshotPoller } from '../services/session/captionSnapshotPoller';
import { startSessionEventPoller } from '../services/session/sessionEventPoller';
import { startLocalServicesMonitor } from '../services/settings/localServicesMonitor';
import { ensureLocalServicesReady, listAudioDevices, listTtsVoices, loadSettings } from '../services/settings/settingsManager';
import { areLocalServicesReady, setLocalServicesFailed, setLocalServicesStarting, setLocalServicesStatus } from '../stores/services/localServicesStore';
import { settingsStore } from '../stores/settings/settingsStore';

export async function bootstrapApp(): Promise<void> {
  startSessionEventPoller();
  startCaptionSnapshotPoller();

  try {
    settingsStore.settings = await loadSettings();
    const [devices, voices] = await Promise.all([listAudioDevices(), listTtsVoices()]);
    settingsStore.inputDevices = devices.inputDevices;
    settingsStore.outputDevices = devices.outputDevices;
    settingsStore.ttsVoices = voices;
    logDiagnostic('[SETTINGS][LOAD]', {
      localAsrProvider: settingsStore.settings.localAsr.provider,
      localAsrWsUrl: settingsStore.settings.localAsr.wsUrl,
      localTranslationConfigured: Boolean(settingsStore.settings.localTranslation.endpoint || settingsStore.settings.localTranslation.modelPath),
      qwenTtsModel: settingsStore.settings.qwenTts.model,
      qwenTtsHasApiKey: settingsStore.settings.qwenTts.hasApiKey,
    });

    setLocalServicesStarting();
    try {
      const status = await ensureLocalServicesReady();
      setLocalServicesStatus(status);
      if (!status.asrOk || !status.translationOk) startLocalServicesMonitor();
      logDiagnostic('[SETTINGS][LOAD]', {
        localAsrConnected: status.asrOk,
        localTranslationConnected: status.translationOk,
        startedAsr: status.startedAsr,
        startedTranslation: status.startedTranslation,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setLocalServicesFailed(message);
      startLocalServicesMonitor();
      settingsStore.error = message;
    }
  } catch (error) {
    settingsStore.error = error instanceof Error ? error.message : '设置加载失败。';
  }

  if (settingsStore.settings?.e2eAutostartSystemSubtitle && areLocalServicesReady()) {
    void useSystemSubtitle().startSystemSubtitle();
  }
}
