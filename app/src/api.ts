import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Snapshot, Action, Hero } from './types';
import roster from '../public/assets/heroes.json';
import catalog from '../src-tauri/resources/voice-packs.json';

export const desktop = isTauri();

export async function setThemeIcon(theme: 'dark' | 'light'): Promise<void> {
  if (desktop) await invoke('set_theme_icon', { theme });
}

export async function roadmapAudio(character: string, dubbed: boolean): Promise<number[]> {
  return invoke<number[]>('roadmap_audio', { character, dubbed });
}

export async function action(name: Action, options?: { gamePath?: string }): Promise<Snapshot> {
  if (desktop) return invoke<Snapshot>('launcher_action', { action: name, ...options });
  if (name !== 'status') throw new Error('Abra o launcher para desktop para alterar o idioma das vozes ou iniciar Deadlock.');
  return {
    language: 'en', ui_language: 'pt-BR', game_running: false, game_root: null, game_error: null,
    pack_available: false, version: '0.1.0-beta.3', updated_at: new Date().toISOString(),
    pack_version: catalog.packs[0].version, pack_download_available: false,
    voices_pending_application: false, update_repository: null, update_error: null,
    client_update: null, voice_update: null,
    portraits_source: roster.source,
    heroes: roster.heroes.map((hero): Hero => {
      const pack = catalog.packs.flatMap(p => p.heroes).find(item => item.asset_id === hero.asset_id);
      const total = pack?.total ?? 0;
      const available = pack?.available ?? 0;
      return { ...hero, total, available, content_available: available,
        state: available && available === total ? 'complete' : available ? 'partial' : 'not-started',
        coverage: total ? 100 * available / total : null,
      };
    }).sort((a, b) => Number(a.content_available === 0) - Number(b.content_available === 0) || a.name.localeCompare(b.name, 'pt-BR')),
  };
}
