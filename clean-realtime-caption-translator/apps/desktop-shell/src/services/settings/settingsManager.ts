import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, AudioDevicesResult, ConnectionTestResult, LocalServicesStatus, SecretStatus, TtsVoiceOption } from '../../types/settings';

export async function loadSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('load_settings');
}

export async function getAppSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('get_app_settings');
}

export async function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>('save_app_settings', { settings });
}

export async function resetAppSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('reset_app_settings');
}

export async function testLocalAsrConnection(): Promise<ConnectionTestResult> {
  return invoke<ConnectionTestResult>('test_local_asr_connection');
}

export async function testLocalTranslationConnection(): Promise<ConnectionTestResult> {
  return invoke<ConnectionTestResult>('test_local_translation_connection');
}

export async function probeLocalServiceStatus(): Promise<LocalServicesStatus> {
  return invoke<LocalServicesStatus>('probe_local_service_status');
}

export async function ensureLocalServicesReady(): Promise<LocalServicesStatus> {
  return invoke<LocalServicesStatus>('ensure_local_services_ready');
}

export async function testTtsConnection(): Promise<ConnectionTestResult> {
  return invoke<ConnectionTestResult>('test_tts_connection');
}

export async function getSecretStatus(): Promise<SecretStatus> {
  return invoke<SecretStatus>('get_secret_status');
}

export async function listAudioDevices(): Promise<AudioDevicesResult> {
  return invoke<AudioDevicesResult>('list_audio_devices');
}

export async function listTtsVoices(): Promise<TtsVoiceOption[]> {
  return invoke<TtsVoiceOption[]>('list_tts_voices');
}
