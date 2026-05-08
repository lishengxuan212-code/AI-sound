export interface LocalAsrSettings {
  provider: 'sherpa_onnx' | 'vosk';
  wsUrl: string;
  modelDir: string;
  sourceLang: string;
}

export interface LocalTranslationSettings {
  provider: string;
  endpoint: string;
  modelPath: string;
  modelName: string;
  sourceLang: string;
  targetLang: string;
}

export interface QwenTtsSettings {
  endpoint: string;
  model: 'qwen-qwen-tts-latest';
  voice: string;
  format: string;
  sampleRate: number;
  speed: number;
  volume: number;
  hasApiKey: boolean;
}

export interface AppSettings {
  localAsr: LocalAsrSettings;
  localTranslation: LocalTranslationSettings;
  qwenTts: QwenTtsSettings;
  logLevel: string;
  diagnosticsEnabled: boolean;
}
