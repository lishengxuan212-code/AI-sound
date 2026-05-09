export interface LocalAsrSettings {
  provider: 'sherpa_onnx' | 'vosk';
  wsUrl: string;
  modelDir: string;
  sampleRate: number;
  languageMode: string;
  enableEndpointDetection: boolean;
}

export interface LocalTranslationSettings {
  provider: string;
  endpoint: string;
  apiKey: string;
  proxyUrl: string;
  prompt: string;
  modelPath: string;
  modelName: string;
  sourceLang: string;
  targetLang: string;
  timeoutMs: number;
}

export interface QwenTtsSettings {
  endpoint: string;
  apiKey: string;
  model: string;
  voice: string;
  format: string;
  sampleRate: number;
  speed: number;
  volume: number;
  hasApiKey: boolean;
  gameTtsApiKeyConfigured: boolean;
  dashscopeApiKeyConfigured: boolean;
}

export interface AudioDeviceSettings {
  systemOutputDeviceName: string;
  micInputDeviceName: string;
}

export interface AudioDeviceSummary {
  name: string;
  isDefault: boolean;
}

export interface AudioDevicesResult {
  inputDevices: AudioDeviceSummary[];
  outputDevices: AudioDeviceSummary[];
}

export interface TtsVoiceOption {
  value: string;
  labelZh: string;
  languageHint: string;
}

export interface LanguageSettings {
  systemSourceLang: string;
  systemTargetLang: string;
  micSourceLang: string;
  micTargetLang: string;
}

export interface SystemSegmenterSettings {
  minWordsBeforeFinalize: number;
  preferredMaxChars: number;
  hardMaxChars: number;
  silenceForVisualFinalizeMs: number;
  allowCommaFinalize: boolean;
  allowShortPauseFinalize: boolean;
}

export interface MicSegmenterSettings {
  minWordsBeforeTranslate: number;
  preferredMaxChars: number;
  hardMaxChars: number;
  silenceForTranslateMs: number;
  allowCommaTranslate: boolean;
}

export interface DiagnosticsSettings {
  logLevel: 'debug' | 'info' | 'warn' | 'error';
  diagnosticsEnabled: boolean;
  showSystemLogs: boolean;
  showMicLogs: boolean;
  showAsrLogs: boolean;
  showTranslationLogs: boolean;
  showTtsLogs: boolean;
  showErrorLogs: boolean;
}

export interface AppSettings {
  localAsr: LocalAsrSettings;
  localTranslation: LocalTranslationSettings;
  qwenTts: QwenTtsSettings;
  audioDevices: AudioDeviceSettings;
  language: LanguageSettings;
  systemSegmenter: SystemSegmenterSettings;
  micSegmenter: MicSegmenterSettings;
  diagnostics: DiagnosticsSettings;
  logLevel: DiagnosticsSettings['logLevel'];
  diagnosticsEnabled: boolean;
  e2eAutostartSystemSubtitle: boolean;
}

export interface SecretStatus {
  gameTtsApiKeyConfigured: boolean;
  dashscopeApiKeyConfigured: boolean;
}

export interface ConnectionTestResult {
  ok: boolean;
  message: string;
}

export interface LocalServicesStatus {
  asrOk: boolean;
  translationOk: boolean;
  startedAsr: boolean;
  startedTranslation: boolean;
  message: string;
}
