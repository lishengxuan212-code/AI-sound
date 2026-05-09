import { reactive } from 'vue';
import type { LocalServicesStatus } from '../../types/settings';

export type LocalServicesPhase = 'idle' | 'starting' | 'ready' | 'failed';

export const localServicesStore = reactive<{
  phase: LocalServicesPhase;
  status?: LocalServicesStatus;
  message: string;
}>({
  phase: 'idle',
  message: '本地服务尚未检查。',
});

export function setLocalServicesStarting(message = '正在启动本地 ASR 与翻译服务...'): void {
  localServicesStore.phase = 'starting';
  localServicesStore.message = message;
}

export function setLocalServicesStatus(status: LocalServicesStatus): void {
  localServicesStore.status = status;
  localServicesStore.phase = status.asrOk && status.translationOk ? 'ready' : 'failed';
  localServicesStore.message = status.message;
}

export function setLocalServicesFailed(message: string): void {
  localServicesStore.phase = 'failed';
  localServicesStore.message = message;
}

export function areLocalServicesReady(): boolean {
  return localServicesStore.phase === 'ready' && Boolean(localServicesStore.status?.asrOk && localServicesStore.status.translationOk);
}
