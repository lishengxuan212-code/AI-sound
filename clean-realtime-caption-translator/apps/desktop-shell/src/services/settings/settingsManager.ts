import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from '../../types/settings';

export async function loadSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('load_settings');
}
