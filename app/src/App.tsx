import { useCallback, useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { ArrowLeft, ArrowUpRight, Check, ChevronRight, CircleHelp, Clock3, Download, FolderOpen, Heart, LoaderCircle, Maximize2, Minus, Moon, Pencil, Play, RefreshCw, Search, Settings2, Sun, X } from 'lucide-react';
import { action, desktop, setThemeIcon } from './api';
import { Flag } from './Flags';
import type { Hero, HeroState, Language, Snapshot, Action, OperationProgress } from './types';

const names: Record<HeroState, string> = {
  complete: 'Available', partial: 'Available · partial coverage', 'in-progress': 'In Production', 'not-started': 'Coming Later',
};
const contentState = (hero: Hero): HeroState => hero.content_available > 0
  ? hero.content_available === hero.total ? 'complete' : 'partial'
  : hero.state;
const number = new Intl.NumberFormat('en-US');
const languageLabel: Record<Language, string> = {
  en: 'Original',
  'pt-BR': 'PT-BR',
};
const languageName: Record<Language, string> = {
  en: 'English',
  'pt-BR': 'Portuguese (Brazil)',
};
type Theme = 'dark' | 'light';

function Modal({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    close.current?.focus();
    const keyboard = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
      if (event.key === 'Tab') {
        const dialog = close.current?.closest('[role="dialog"]');
        const elements = dialog?.querySelectorAll<HTMLElement>('button:not(:disabled), a[href], input:not(:disabled)');
        if (!elements?.length) return;
        const first = elements[0]; const last = elements[elements.length - 1];
        if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
        if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
      }
    };
    document.addEventListener('keydown', keyboard);
    return () => { document.removeEventListener('keydown', keyboard); previous?.focus(); };
  }, [onClose]);
  return <div className="modal-backdrop" onClick={onClose}>
    <section className="modal" role="dialog" aria-modal="true" aria-label={title} onClick={e => e.stopPropagation()}>
      <div className="modal-heading"><h2>{title}</h2><button ref={close} className="icon-button" aria-label="Close dialog" onClick={onClose}><X size={19} /></button></div>
      {children}
    </section>
  </div>;
}

function LanguageChangedDialog({ language, onClose, onSupport }: { language: Language; onClose: () => void; onSupport: () => void }) {
  const close = useRef<HTMLButtonElement>(null);
  const label = language === 'pt-BR' ? 'Brazilian Portuguese' : 'Original English';
  useEffect(() => {
    close.current?.focus();
    const keyboard = (event: KeyboardEvent) => { if (event.key === 'Escape') onClose(); };
    document.addEventListener('keydown', keyboard);
    return () => document.removeEventListener('keydown', keyboard);
  }, [onClose]);
  return <div className="support-backdrop" onClick={onClose}>
    <section className="support-modal" role="dialog" aria-modal="true" aria-label="Language changed" onClick={event => event.stopPropagation()}>
      <button ref={close} className="support-close" aria-label="Close confirmation" onClick={onClose}><X size={27} /></button>
      <div className="support-success"><span className="success-emblem"><Check size={34} strokeWidth={3} /></span><h2><span>Language changed to</span><strong>{label} successfully!</strong></h2></div>
      <div className="support-divider" />
      <div className="support-story"><div className="support-art"><img src="/assets/support/support-characters-v2.png" alt="A DeadVox hero offering a coxinha to the developer" /></div><div className="support-copy"><span>KEEP DEADVOX ALIVE</span><h3>Buy a coxinha for the developer.</h3><p>Every contribution helps make the next voice pack possible.</p></div></div>
      <div className="support-divider lower" />
      <div className="support-actions"><button className="support-later" onClick={onClose}><Clock3 size={21} /> Maybe later</button><div className="support-primary"><button className="support-button" onClick={onSupport}><Heart size={24} /><span>Support project</span><ChevronRight size={25} /></button><small>Official support channel coming soon</small></div></div>
    </section>
  </div>;
}

function Wordmark() {
  return <span className="wordmark"><img className="wordmark-ink" src="/assets/brand/deadvox-wordmark.png" alt="DeadVox" width="4344" height="1448" /><img className="wordmark-accent" src="/assets/brand/deadvox-wordmark.png" alt="" aria-hidden="true" width="4344" height="1448" /></span>;
}

export default function App() {
  const [data, setData] = useState<Snapshot | null>(null);
  const [page, setPage] = useState<Language | null>(null);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState('all');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState('');
  const [dialog, setDialog] = useState<'about' | 'settings' | 'updates' | null>(null);
  const [progress, setProgress] = useState<OperationProgress | null>(null);
  const [gamePath, setGamePath] = useState('');
  const [theme, setTheme] = useState<Theme>(() => localStorage.getItem('deadvox-theme') === 'light' ? 'light' : 'dark');
  const [languageConfirmation, setLanguageConfirmation] = useState<Language | null>(null);
  const seenUpdates = useRef('');
  const [selectedHero, setSelectedHero] = useState<Hero | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const operation = useRef(false);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem('deadvox-theme', theme);
    void setThemeIcon(theme).catch(() => undefined);
  }, [theme]);

  const refresh = async () => {
    if (operation.current) return;
    operation.current = true; setRefreshing(true); setProgress({ phase: 'Reading voice settings', percent: null });
    try { setData(await action('status')); setError(null); }
    catch (e) { setError(String(e)); }
    finally { operation.current = false; setRefreshing(false); setProgress(null); }
  };
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    const cleanup: Array<() => void> = [];
    void listen<OperationProgress>('operation-progress', event => {
      if (operation.current) setProgress(event.payload);
    }).then(unlisten => { if (disposed) unlisten(); else cleanup.push(unlisten); });
    void listen<string>('close-blocked', event => setNote(event.payload)).then(unlisten => { if (disposed) unlisten(); else cleanup.push(unlisten); });
    return () => { disposed = true; cleanup.forEach(unlisten => unlisten()); };
  }, []);
  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => { if (!document.hidden) void refresh(); }, 20000);
    return () => window.clearInterval(id);
  }, []);

  const perform = async (name: Action, options?: { gamePath?: string }) => {
    if (operation.current) return;
    operation.current = true; setBusy(true); setError(null); setNote(''); setProgress({ phase: 'Preparing', percent: null });
    if (['enable', 'disable', 'download_voices', 'update_client'].includes(name)) setDialog(null);
    try {
      const result = await action(name, options);
      setData(result);
      if (name === 'set_game_folder' || name === 'auto_game_folder') setGamePath(result.game_root ?? '');
      const messages: Partial<Record<Action, string>> = {
        play: 'Starting Deadlock through Steam…', enable: 'PT-BR selected.', disable: 'Original voices selected.',
        set_game_folder: 'Game folder checked.', auto_game_folder: 'Steam libraries checked.',
        download_voices: result.voices_pending_application ? 'Voices downloaded. Close Deadlock and apply the update.' : 'PT-BR voices downloaded and verified.',
        update_client: 'Update downloaded. Restarting the launcher…',
        check_updates: result.update_error ?? (result.client_update || result.voice_update ? 'Updates are available.' : result.update_repository ? 'No newer releases found.' : 'Online updates are not available in this build yet.'),
      };
      if (name === 'enable' || name === 'disable') { setNote(''); setLanguageConfirmation(result.language); }
      else setNote(messages[name] ?? 'Done.');
      if (name === 'update_client' && desktop) await getCurrentWindow().close();
    } catch (e) { setError(String(e)); }
    finally { operation.current = false; setBusy(false); setProgress(null); }
  };
  useEffect(() => {
    if (!desktop || !data?.update_repository) return;
    const timer = window.setTimeout(() => { if (!operation.current) void perform('check_updates'); }, 1500);
    const interval = window.setInterval(() => { if (!operation.current && !document.hidden) void perform('check_updates'); }, 6 * 60 * 60 * 1000);
    return () => { window.clearTimeout(timer); window.clearInterval(interval); };
  }, [data?.update_repository]);
  useEffect(() => {
    const key = `${data?.client_update?.version ?? ''}|${data?.voice_update?.version ?? ''}`;
    if (key !== '|' && key !== seenUpdates.current) { seenUpdates.current = key; setDialog('updates'); }
  }, [data?.client_update?.version, data?.voice_update?.version]);
  const openRoster = (language: Language) => { setPage(language); setQuery(''); setFilter('all'); };
  const selectLanguage = (language: Language) => {
    if (data?.language !== language || (language === 'pt-BR' && data.voices_pending_application)) void perform(language === 'en' ? 'disable' : 'enable');
  };
  const closeDialog = useCallback(() => setDialog(null), []);
  const closeHero = useCallback(() => setSelectedHero(null), []);
  const closeLanguageConfirmation = useCallback(() => setLanguageConfirmation(null), []);
  const showSupportAvailability = useCallback(() => { setLanguageConfirmation(null); setNote('Official DeadVox support options will be available soon.'); }, []);
  const heroes = data?.heroes ?? [];
  const loading = busy || refreshing;
  const canChange = !!data && !data.game_error && !data.game_running && !loading;
  const openSettings = () => { setGamePath(data?.game_root ?? ''); setDialog('settings'); };
  const progressPercent = progress?.percent == null ? null : Math.max(0, Math.min(100, progress.percent));
  const isOriginal = page === 'en';
  const available = heroes.filter(h => h.content_available > 0).length;
  const production = heroes.filter(h => contentState(h) === 'in-progress').length;
  const notStarted = heroes.filter(h => contentState(h) === 'not-started').length;
  const installationLabel = (hero: Hero) => !hero.content_available ? null
    : data?.voices_pending_application ? 'Update pending'
    : data?.voice_update ? 'Update Available'
    : data?.language === 'pt-BR' && hero.available > 0 ? 'Installed · active'
    : data?.pack_available ? 'Installed · inactive' : 'Not Installed';
  const visible = heroes.filter(h => h.name.toLowerCase().includes(query.toLowerCase()) && (
    filter === 'all' || (filter === 'available' && h.content_available > 0) || (filter === 'in-progress' && contentState(h) === 'in-progress') || (filter === 'not-started' && contentState(h) === 'not-started')
  ));

  return <div className={`app-shell theme-${theme}`}>
    <div className="window-bar">
      <div className="drag-area" data-tauri-drag-region onDoubleClick={() => { if (desktop) void getCurrentWindow().toggleMaximize(); }}>
        <span className="tiny-brand" data-tauri-drag-region><span className="brand-diamond" /> DEADVOX</span>
      </div>
      {desktop && <div className="window-controls">
        <button aria-label="Minimize window" onClick={() => void getCurrentWindow().minimize()}><Minus size={14} /></button>
        <button aria-label="Maximize window" onClick={() => void getCurrentWindow().toggleMaximize()}><Maximize2 size={12} /></button>
        <button className="close-window" aria-label="Close window" disabled={loading} title={loading ? 'Please wait for the current operation' : 'Close'} onClick={() => void getCurrentWindow().close()}><X size={15} /></button>
      </div>}
    </div>
    <header className="app-header">
      <button className="brand" onClick={() => setPage(null)} aria-label="DeadVox home"><Wordmark /></button>
      <nav><button className="icon-button" title="Updates" aria-label="Updates" onClick={() => setDialog('updates')}><Download size={18} />{(data?.client_update || data?.voice_update) && <span className="update-dot" />}</button><button className="icon-button" title="Settings" aria-label="Settings" onClick={openSettings}><Settings2 size={19} /></button><span className="nav-divider" /><button className="text-button" onClick={() => setDialog('about')}>About <ArrowUpRight size={14} /></button></nav>
    </header>

    {!page ? <main className="home">
      <div className="home-intro"><span className="eyebrow">YOUR GAME. YOUR VOICE.</span><h1>Choose your voice language.</h1><p>A different voice. The same Deadlock.</p></div>
      <div className="language-grid">
        {(['en', 'pt-BR'] as Language[]).map(language => {
          const selected = data?.language === language;
          return <article key={language} className={`language-card ${selected ? 'selected' : ''} ${busy ? 'changing' : ''}`}>
            <button className="language-hit-area" aria-label={`Select ${languageName[language]}`} aria-pressed={selected} disabled={!canChange || (language === 'pt-BR' && !data?.pack_available && !selected)} onClick={() => selectLanguage(language)} />
            <div className="card-topline"><span className="card-eyebrow">{language === 'en' ? 'ORIGINAL AUDIO' : 'PT-BR VOICE PACK'}</span>{selected ? <span className="selected-check"><Check size={14} strokeWidth={2.2} /></span> : <span className="unselected-check" />}</div>
            <Flag country={language === 'en' ? 'us' : 'br'} />
            <div className="language-title"><h2>{languageLabel[language]}</h2><button className="edit-button" aria-label={`Edit ${languageName[language]}`} title="View heroes" onClick={() => openRoster(language)}><Pencil size={17} strokeWidth={1.6} /></button></div>
            <p>{languageName[language]}</p>
            {language === 'pt-BR' && data && (!data.pack_available || data.voice_update || data.voices_pending_application) && <button className="pack-download" disabled={loading || (data.voices_pending_application ? !canChange : !data.pack_download_available)} onClick={() => void perform(data.voices_pending_application ? 'enable' : 'download_voices')}><Download size={14} />{data.voices_pending_application ? 'Apply update' : data.voice_update ? 'Update voices' : 'Download voices'}</button>}
          </article>;
        })}
      </div>
      {data?.game_running && <p className="inline-hint">Close Deadlock to switch voice languages.</p>}
      {data?.game_error && <button className="text-button" onClick={openSettings}><FolderOpen size={16} /> Locate Deadlock</button>}
      {!data && !error && <div className="inline-hint"><LoaderCircle className="spin" size={16} /> Reading your voice settings…</div>}
    </main> : <main className="roster">
      <div className="page-heading"><div className="page-heading-left"><button className="back-button" aria-label="Back to languages" onClick={() => setPage(null)}><ArrowLeft size={20} /></button><div><div className="eyebrow">VOICE LIBRARY</div><h1>{isOriginal ? 'Original voices' : 'PT-BR voice pack'}</h1><p>{isOriginal ? 'The original English cast, included with Deadlock.' : 'Explore the heroes. Follow the voice-pack progress.'}</p></div></div><button className="refresh-button" aria-label="Refresh progress" onClick={() => void refresh()} disabled={refreshing}><RefreshCw size={16} className={refreshing ? 'spin' : ''} /><span>Refresh</span></button></div>
      <div className="roster-tools"><div className="filter-tabs" role="group" aria-label="Filter hero status">
        <button className={filter === 'all' ? 'active' : ''} onClick={() => setFilter('all')}>All heroes <span>{heroes.length}</span></button>
        {!isOriginal && <>
          <button className={filter === 'available' ? 'active' : ''} onClick={() => setFilter('available')}>Available <span>{available}</span></button>
          <button className={filter === 'in-progress' ? 'active' : ''} onClick={() => setFilter('in-progress')}>In Production <span>{production}</span></button>
          <button className={filter === 'not-started' ? 'active' : ''} onClick={() => setFilter('not-started')}>Coming Later <span>{notStarted}</span></button>
        </>}
      </div><label className="search-box"><Search size={16} /><input value={query} onChange={e => setQuery(e.target.value)} placeholder="Search heroes" aria-label="Search heroes" />{query && <button aria-label="Clear search" onClick={() => setQuery('')}><X size={14} /></button>}</label></div>
      <div className="list-header"><span>HERO</span><span>{isOriginal ? 'VOICE LANGUAGE' : 'VOICE-PACK STATUS'}</span><span>{isOriginal ? 'AVAILABILITY' : 'COVERAGE'}</span><span /></div>
      <div className="hero-list">
        {visible.map(hero => <button className={`hero-row ${!isOriginal && contentState(hero) === 'not-started' ? 'coming-later' : ''}`} key={hero.id} onClick={() => setSelectedHero(hero)} aria-label={`View ${hero.name} details`}>
          <div className="hero-identity"><img src={hero.portrait} alt={hero.name} loading="lazy" /><span>{hero.name}</span></div>
          <div className="hero-state-block"><div className={`hero-state ${isOriginal ? 'complete' : contentState(hero)}`}><span className="status-dot" />{isOriginal ? 'English · Original' : names[contentState(hero)]}</div>{!isOriginal && installationLabel(hero) && <span className="installation-state">{installationLabel(hero)}</span>}</div>
          <div className="coverage-cell">{isOriginal ? <span className="original-available">Included with the game</span> : <><div className="coverage-label"><span>{hero.content_available ? `${number.format(hero.content_available)} / ${number.format(hero.total)} PT-BR lines` : 'Voice pack coming later'}</span><span>{hero.content_available && hero.total ? `${Math.round(1000 * hero.content_available / hero.total) / 10}%` : '—'}</span></div><div className={`progress-track ${contentState(hero)}`}><span style={{ width: hero.total ? `${100 * hero.content_available / hero.total}%` : '0%' }} /></div></>}</div>
          <ChevronRight size={16} className="row-arrow" />
        </button>)}
        {visible.length === 0 && <div className="empty-state"><Search size={24} /><h3>No heroes found</h3><p>Try a different name or status.</p><button className="text-button" onClick={() => { setQuery(''); setFilter('all'); }}>Reset filters</button></div>}
      </div>
      <div className="roster-caption"><span>Portraits: Valve · Deadlock API</span><span>{isOriginal ? 'Original audio is managed by the game.' : 'Released content and installation are shown separately.'}</span></div>
    </main>}

    {(error || data?.game_error) && <div className="notice error" role="alert"><CircleHelp size={17} /><span>{error || data?.game_error}</span><button className="text-button" onClick={() => void refresh()}>Retry</button></div>}
    {note && !error && <div className="notice" role="status"><Check size={16} /><span>{note}</span><button className="icon-button" aria-label="Dismiss message" onClick={() => setNote('')}><X size={14} /></button></div>}
    <footer className="app-footer"><div className={`footer-language ${data && !data.game_error ? 'ready' : ''}`}><span className="status-dot" /><span>{data ? data.language === 'en' ? 'Original voices selected' : 'PT-BR selected' : 'Connecting…'}</span></div><div className="footer-actions">{page && <button className="secondary-button" onClick={() => setPage(null)}><ArrowLeft size={16} /> Back</button>}<button className={`play-button ${loading ? 'operation-button' : ''}`} disabled={loading || !data || !!data.game_error || data.game_running} onClick={() => void perform('play')} aria-label={loading ? `${progress?.phase ?? 'Loading'} ${progressPercent === null ? '' : `${Math.round(progressPercent)}%`}` : 'Play Deadlock'}>{loading && <span className={`operation-fill ${progressPercent === null ? 'indeterminate' : ''}`} style={progressPercent === null ? undefined : { width: `${progressPercent}%` }} />}<span className="play-content">{loading ? <LoaderCircle size={17} className="spin" /> : <Play size={16} fill="currentColor" />}<span>{loading ? <><strong>{progressPercent === null ? 'Loading…' : `${Math.round(progressPercent)}%`}</strong><small>{progress?.phase ?? 'Please wait'}</small></> : data?.game_running ? 'Deadlock is running' : 'Play Deadlock'}</span>{!loading && <ArrowUpRight size={17} />}</span></button></div></footer>

    {languageConfirmation && <LanguageChangedDialog language={languageConfirmation} onClose={closeLanguageConfirmation} onSupport={showSupportAvailability} />}
    {dialog === 'about' && <Modal title="About DeadVox" onClose={closeDialog}><div className="about-mark"><Wordmark /></div><div className="about-descriptor">Community Dubbing Platform</div><p>Community-made dubbing for Deadlock, starting in Brazilian Portuguese. One hero at a time. One language at a time.</p><p className="brand-signature">AI Assisted • Human Directed</p><p>Technology handles the scale. Humans protect the performance.</p><div className="detail-lines"><div><span>Version</span><strong>{data?.version ?? '0.1.0-alpha.5'}</strong></div><div><span>Hero portraits</span><strong>Valve, via Deadlock API</strong></div><div><span>Voice pack</span><strong>PT-BR · Portuguese (Brazil)</strong></div><div><span>Brand domain</span><strong>deadvox.com</strong></div></div><p className="small-copy">DeadVox is an independent community project and is not affiliated with, endorsed by or sponsored by Valve Corporation. Deadlock, its characters, names and related assets are property of their respective owners.</p></Modal>}
    {dialog === 'settings' && <Modal title="Settings" onClose={closeDialog}><section className="settings-section"><span className="settings-label">APPEARANCE</span><h3>Color theme</h3><div className="theme-picker" role="group" aria-label="Color theme"><button className={`theme-option ${theme === 'dark' ? 'active' : ''}`} aria-pressed={theme === 'dark'} onClick={() => setTheme('dark')}><Moon size={17} /><span><strong>Dark</strong><small>Deep black and crimson</small></span></button><button className={`theme-option ${theme === 'light' ? 'active' : ''}`} aria-pressed={theme === 'light'} onClick={() => setTheme('light')}><Sun size={17} /><span><strong>Light</strong><small>Warm ivory and ink</small></span></button></div></section><section className="settings-section game-settings"><span className="settings-label">GAME</span><h3>Deadlock folder</h3><p>Find Deadlock in your Steam libraries or choose its installation folder.</p><label className="path-label" htmlFor="game-folder">Game folder</label><input id="game-folder" className="settings-input" value={gamePath} onChange={event => setGamePath(event.target.value)} placeholder="Select your Deadlock folder" disabled={loading} /><div className="settings-actions"><button className="secondary-button" disabled={loading || !desktop} onClick={() => void perform('set_game_folder')}><FolderOpen size={15} /> Browse</button><button className="secondary-button" disabled={loading || !gamePath.trim() || !desktop} onClick={() => void perform('set_game_folder', { gamePath })}>Use folder</button><button className="text-button" disabled={loading || !desktop} onClick={() => void perform('auto_game_folder')}>Detect with Steam</button></div><p className="small-copy">{data?.game_root ? `Detected: ${data.game_root}` : 'No game folder selected yet.'}</p></section><section className="settings-section update-settings"><span className="settings-label">UPDATES</span><p className="small-copy">Updates come from the DeadVox release channel. The download source cannot be changed in the launcher.</p><p className="small-copy">Launcher and voice packs update separately. Close the game before applying voice changes.</p></section></Modal>}
    {dialog === 'updates' && <Modal title="Updates" onClose={closeDialog}><p>{data?.client_update || data?.voice_update ? 'New releases are available. Choose what to update.' : data?.update_repository ? 'Your launcher and voices are checked separately through the official release channel.' : 'Online updates are not available in this build yet.'}</p><div className="update-card"><div><h3>Launcher</h3><p>{data?.client_update ? `Version ${data.client_update.version} · ${Math.ceil(data.client_update.bytes / 1048576)} MB` : `Installed: ${data?.version ?? '—'}`}</p></div><button className="secondary-button" disabled={loading || !data?.client_update || !desktop} onClick={() => void perform('update_client')}><Download size={15} /> Update & restart</button></div><div className="update-card"><div><h3>PT-BR voices</h3><p>{data?.voice_update ? `Version ${data.voice_update.version} · ${Math.ceil(data.voice_update.bytes / 1048576)} MB` : `Installed: ${data?.pack_version ?? '—'}`}</p></div><button className="secondary-button" disabled={loading || (!data?.voice_update && !(!data?.pack_available && data?.pack_download_available)) || !desktop} onClick={() => void perform('download_voices')}><Download size={15} /> {data?.voice_update ? 'Update voices' : 'Download'}</button></div>{data?.update_error && <p className="small-copy" role="status">{data.update_error}</p>}<div className="settings-actions"><button className="secondary-button" disabled={loading || !data?.update_repository || !desktop} onClick={() => void perform('check_updates')}><RefreshCw size={15} /> Check for updates</button></div></Modal>}
    {selectedHero && <Modal title={selectedHero.name} onClose={closeHero}><div className="hero-detail-intro"><img src={selectedHero.portrait} alt={selectedHero.name} /><div><div className="eyebrow">{isOriginal ? 'ORIGINAL VOICES' : 'PT-BR VOICE PACK'}</div><h3>{isOriginal ? 'Included with Deadlock' : names[contentState(selectedHero)]}</h3></div></div>{isOriginal ? <p>This hero uses the original English voice files supplied with the game.</p> : <><div className="detail-lines"><div><span>PT-BR lines in the released pack</span><strong>{number.format(selectedHero.content_available)} lines</strong></div><div><span>Speech lines eligible for PT-BR</span><strong>{selectedHero.total ? number.format(selectedHero.total) : 'Not announced'}</strong></div>{installationLabel(selectedHero) && <div><span>Installation</span><strong>{installationLabel(selectedHero)}</strong></div>}</div><p className="small-copy">{contentState(selectedHero) === 'partial' ? 'The pack includes part of this hero’s dialogue. Remaining lines play in English.' : contentState(selectedHero) === 'complete' ? 'All eligible speech in this release is covered. Original sounds and creative performances are preserved.' : contentState(selectedHero) === 'in-progress' ? 'This character is in production. A release date has not been announced.' : 'This character’s voice pack is coming later. A release date has not been announced.'}</p></>}</Modal>}
  </div>;
}
