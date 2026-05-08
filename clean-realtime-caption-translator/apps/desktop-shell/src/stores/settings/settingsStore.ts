import { reactive } from 'vue';
import type { AppSettings } from '../../types/settings';

export const settingsStore = reactive<{ settings?: AppSettings; error?: string }>({});
