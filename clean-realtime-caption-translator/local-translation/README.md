# Local Translation

Configure a local translation model or local HTTP translation service here.

Required behavior:

- input: `requestId`, `sessionKind`, `sourceText`, `sourceLang`, `targetLang`, `contextBefore`
- output: `requestId`, `translatedText`, `status`, `error`
- no cloud LLM translation
- no cloud translation fallback
- no LLM corrector

If no model path or endpoint is configured, the app displays `本地翻译模型未配置`.
