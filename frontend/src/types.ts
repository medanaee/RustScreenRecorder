export interface RecordConfig {
  fps: number;
  quality_bitrate: number;
  resolution: string;
  encoder: string;
  record_mic: boolean;
  record_system_audio: boolean;
  mic_source: string;
  output_folder: string;
  show_cursor: boolean;
}