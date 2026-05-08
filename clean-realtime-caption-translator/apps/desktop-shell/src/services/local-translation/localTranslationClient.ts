import type { LocalTranslationRequest, LocalTranslationResponse } from '../../types/translation';

export async function translateLocal(request: LocalTranslationRequest, endpoint: string, timeoutMs = 15000): Promise<LocalTranslationResponse> {
  if (!endpoint) {
    return {
      requestId: request.requestId,
      translatedText: '',
      status: 'failed',
      error: '本地翻译模型未配置',
    };
  }

  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`Local translation failed: HTTP ${response.status}`);
    return (await response.json()) as LocalTranslationResponse;
  } catch (error) {
    return {
      requestId: request.requestId,
      translatedText: '',
      status: 'failed',
      error: error instanceof Error ? error.message : 'Local translation failed',
    };
  } finally {
    window.clearTimeout(timeout);
  }
}
