import { probeLocalServiceStatus } from './settingsManager';
import { areLocalServicesReady, localServicesStore, setLocalServicesStatus } from '../../stores/services/localServicesStore';

let monitorHandle: number | undefined;

export function startLocalServicesMonitor(intervalMs = 3000): void {
  if (monitorHandle !== undefined) return;
  monitorHandle = window.setInterval(() => {
    if (areLocalServicesReady()) {
      stopLocalServicesMonitor();
      return;
    }
    void refreshLocalServicesStatus();
  }, intervalMs);
}

export function stopLocalServicesMonitor(): void {
  if (monitorHandle === undefined) return;
  window.clearInterval(monitorHandle);
  monitorHandle = undefined;
}

export async function refreshLocalServicesStatus(): Promise<void> {
  try {
    const status = await probeLocalServiceStatus();
    setLocalServicesStatus(status);
    if (status.asrOk && status.translationOk) stopLocalServicesMonitor();
  } catch (error) {
    localServicesStore.phase = 'failed';
    localServicesStore.message = error instanceof Error ? error.message : String(error);
  }
}
