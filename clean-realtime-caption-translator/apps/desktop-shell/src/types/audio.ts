export type AudioInputKind = 'system' | 'microphone';

export interface AudioLevel {
  inputKind: AudioInputKind;
  rms: number;
  peak: number;
  updatedAt: number;
}
