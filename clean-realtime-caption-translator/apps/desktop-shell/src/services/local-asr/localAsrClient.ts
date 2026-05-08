import type { LocalAsrEvent } from '../../types/asr';
import { SessionKind } from '../../types/events';

export interface LocalAsrClientOptions {
  url: string;
  sessionKind: SessionKind;
  sessionId: string;
  onEvent: (event: LocalAsrEvent) => void;
  onError: (error: Error) => void;
}

export class LocalAsrClient {
  private socket?: WebSocket;

  constructor(private readonly options: LocalAsrClientOptions) {}

  connect(): void {
    this.socket = new WebSocket(this.options.url);
    this.socket.binaryType = 'arraybuffer';
    this.socket.onmessage = (message) => {
      const payload = JSON.parse(String(message.data)) as LocalAsrEvent;
      this.options.onEvent({ ...payload, sessionKind: this.options.sessionKind });
    };
    this.socket.onerror = () => this.options.onError(new Error('Local ASR WebSocket error'));
    this.socket.onopen = () => {
      this.socket?.send(
        JSON.stringify({
          type: 'start',
          sessionKind: this.options.sessionKind,
          sessionId: this.options.sessionId,
        }),
      );
    };
  }

  sendPcm(chunk: ArrayBuffer): void {
    if (this.socket?.readyState === WebSocket.OPEN) this.socket.send(chunk);
  }

  close(): void {
    this.socket?.close();
  }
}
