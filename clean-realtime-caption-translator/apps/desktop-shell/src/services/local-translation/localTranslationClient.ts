import type { LocalTranslationRequest, LocalTranslationResponse } from '../../types/translation';

function buildTranslateUrl(endpoint: string): string {
  const trimmed = endpoint.trim().replace(/\/+$/, '');
  if (!trimmed) return '';
  return trimmed.endsWith('/translate') ? trimmed : `${trimmed}/translate`;
}

export async function translateLocal(request: LocalTranslationRequest, endpoint: string, timeoutMs = 15000): Promise<LocalTranslationResponse> {
  const translateUrl = buildTranslateUrl(endpoint);
  if (!translateUrl) {
    return {
      requestId: request.requestId,
      translatedText: '',
      status: 'failed',
      error: '本地翻译 endpoint 未配置',
    };
  }

  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(translateUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
      signal: controller.signal,
    });
    if (!response.ok) {
      const detail = await response.text().catch(() => '');
      throw new Error(detail ? `本地翻译服务连接失败: HTTP ${response.status}: ${detail}` : `本地翻译服务连接失败: HTTP ${response.status}`);
    }
    return (await response.json()) as LocalTranslationResponse;
  } catch (error) {
    return {
      requestId: request.requestId,
      translatedText: '',
      status: 'failed',
      error: error instanceof Error ? error.message : '本地翻译服务连接失败',
    };
  } finally {
    window.clearTimeout(timeout);
  }
}
