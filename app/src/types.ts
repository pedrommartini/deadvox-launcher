export type Language = 'en' | 'pt-BR';
export type HeroState = 'complete' | 'partial' | 'in-progress' | 'not-started';

export interface Hero {
  id: number;
  name: string;
  asset_id: string;
  portrait: string;
  total: number;
  available: number;
  content_available: number;
  state: HeroState;
  coverage: number | null;
}

export interface Snapshot {
  language: Language;
  ui_language: 'pt-BR';
  game_running: boolean;
  game_root: string | null;
  game_error: string | null;
  pack_available: boolean;
  pack_version: string;
  pack_download_available: boolean;
  voices_pending_application: boolean;
  update_repository: string | null;
  update_error: string | null;
  client_update: UpdateSummary | null;
  voice_update: UpdateSummary | null;
  heroes: Hero[];
  updated_at: string;
  version: string;
  portraits_source: string;
}

export interface UpdateSummary { version: string; bytes: number }
export interface OperationProgress { phase: string; percent: number | null }
export type Action = 'status' | 'enable' | 'disable' | 'play' | 'set_game_folder' | 'auto_game_folder' | 'check_updates' | 'download_voices' | 'update_client';
