import { useCallback, useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { ArrowLeft, ArrowUpRight, Check, ChevronRight, CircleHelp, Clock3, Download, FolderOpen, Heart, LoaderCircle, Maximize2, Minus, Moon, Pencil, Play, RefreshCw, Search, Settings2, Sun, X } from 'lucide-react';
import { action, desktop, setThemeIcon } from './api';
import { Flag } from './Flags';
import Roadmap from './Roadmap';
import { detectUiLocale } from './locale';
import type { Hero, HeroState, Language, Snapshot, Action, OperationProgress } from './types';

const names: Record<HeroState, string> = {
  complete: 'Disponível', partial: 'Disponível · cobertura parcial', 'in-progress': 'Em produção', 'not-started': 'Em breve',
};
const contentState = (hero: Hero): HeroState => hero.content_available > 0
  ? hero.content_available === hero.total ? 'complete' : 'partial'
  : hero.state;
const number = new Intl.NumberFormat('pt-BR');
const languageLabel: Record<Language, string> = {
  en: 'Original',
  'pt-BR': 'PT-BR',
};
const languageName: Record<Language, string> = {
  en: 'Inglês',
  'pt-BR': 'Português (Brasil)',
};
type Theme = 'dark' | 'light';
const phases: Record<string, string> = {
  'Locating Deadlock': 'Localizando Deadlock', 'Starting Deadlock': 'Iniciando Deadlock',
  'Reading voice settings': 'Lendo configurações de voz', 'Checking voice pack': 'Verificando pacote de vozes',
  'Backing up game settings': 'Salvando configurações do jogo', 'Verifying installed voices': 'Verificando vozes instaladas',
  'Saving voice language': 'Salvando idioma das vozes', 'Checking original voices': 'Verificando vozes originais',
  'Restoring original voices': 'Restaurando vozes originais', 'Checking launcher updates': 'Verificando atualizações do launcher',
  'Checking voice updates': 'Verificando atualizações de vozes', 'Downloading': 'Baixando', 'Verifying download': 'Verificando download',
  'Saving voice catalog': 'Salvando catálogo de vozes', 'Preparing restart': 'Preparando reinicialização',
  'Applying character voices': 'Aplicando vozes dos personagens', 'Restoring voice priority': 'Restaurando prioridade das vozes',
  'Applying PT-BR': 'Aplicando vozes PT-BR',
};

function ThirdPartyNotices() {
  const [text, setText] = useState('Carregando avisos de terceiros…');
  useEffect(() => {
    let active = true;
    void import('../../docs/THIRD-PARTY-NOTICES.txt?raw').then(module => {
      if (active) setText(module.default);
    }).catch(() => {
      if (active) setText('Não foi possível carregar os avisos. Consulte THIRD-PARTY-NOTICES.txt na pasta do aplicativo.');
    });
    return () => { active = false; };
  }, []);
  return <pre className="third-party-notices">{text}</pre>;
}

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
      <div className="modal-heading"><h2>{title}</h2><button ref={close} className="icon-button" aria-label="Fechar diálogo" onClick={onClose}><X size={19} /></button></div>
      {children}
    </section>
  </div>;
}

function LanguageChangedDialog({ language, onClose, onSupport }: { language: Language; onClose: () => void; onSupport: () => void }) {
  const close = useRef<HTMLButtonElement>(null);
  const label = language === 'pt-BR' ? 'Português brasileiro' : 'Inglês original';
  useEffect(() => {
    close.current?.focus();
    const keyboard = (event: KeyboardEvent) => { if (event.key === 'Escape') onClose(); };
    document.addEventListener('keydown', keyboard);
    return () => document.removeEventListener('keydown', keyboard);
  }, [onClose]);
  return <div className="support-backdrop" onClick={onClose}>
    <section className="support-modal" role="dialog" aria-modal="true" aria-label="Idioma alterado" onClick={event => event.stopPropagation()}>
      <button ref={close} className="support-close" aria-label="Fechar confirmação" onClick={onClose}><X size={27} /></button>
      <div className="support-success"><span className="success-emblem"><Check size={34} strokeWidth={3} /></span><h2><span>Idioma alterado para</span><strong>{label} com sucesso!</strong></h2></div>
      <div className="support-divider" />
      <div className="support-story"><div className="support-art"><img src="/assets/support/support-characters-v2.png" alt="Uma heroína de DeadVox oferecendo uma coxinha ao desenvolvedor" /></div><div className="support-copy"><span>MANTENHA O DEADVOX VIVO</span><h3>Pague uma coxinha para o desenvolvedor.</h3><p>Cada contribuição ajuda a viabilizar o próximo pacote de vozes.</p></div></div>
      <div className="support-divider lower" />
      <div className="support-actions"><button className="support-later" onClick={onClose}><Clock3 size={21} /> Talvez depois</button><div className="support-primary"><button className="support-button" onClick={onSupport}><Heart size={24} /><span>Contribuir com o projeto</span><ChevronRight size={25} /></button><small>Contribua via Pix</small></div></div>
    </section>
  </div>;
}

function Wordmark() {
  return <span className="wordmark"><img className="wordmark-ink" src="/assets/brand/deadvox-wordmark.png" alt="DeadVox" width="4344" height="1448" /><img className="wordmark-accent" src="/assets/brand/deadvox-wordmark.png" alt="" aria-hidden="true" width="4344" height="1448" /></span>;
}

export default function App() {
  const [data, setData] = useState<Snapshot | null>(null);
  const [page, setPage] = useState<Language | 'roadmap' | null>(null);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState('all');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState('');
  const [dialog, setDialog] = useState<'about' | 'settings' | 'updates' | 'notices' | 'support' | null>(null);
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
  useEffect(() => {
    document.documentElement.lang = data?.ui_language ?? detectUiLocale();
  }, [data?.ui_language]);

  const refresh = async () => {
    if (operation.current) return;
    operation.current = true; setRefreshing(true); setProgress({ phase: 'Lendo configurações de voz', percent: null });
    try { setData(await action('status')); setError(null); }
    catch (e) { setError(String(e)); }
    finally { operation.current = false; setRefreshing(false); setProgress(null); }
  };
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    const cleanup: Array<() => void> = [];
    void listen<OperationProgress>('operation-progress', event => {
      if (operation.current) setProgress({ ...event.payload, phase: phases[event.payload.phase] ?? event.payload.phase });
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
    operation.current = true; setBusy(true); setError(null); setNote(''); setProgress({ phase: 'Preparando', percent: null });
    if (['enable', 'disable', 'download_voices', 'update_client'].includes(name)) setDialog(null);
    try {
      const result = await action(name, options);
      setData(result);
      if (name === 'set_game_folder' || name === 'auto_game_folder') setGamePath(result.game_root ?? '');
      const messages: Partial<Record<Action, string>> = {
        play: 'Iniciando Deadlock pelo Steam…', enable: 'PT-BR selecionado.', disable: 'Vozes originais selecionadas.',
        set_game_folder: 'Pasta do jogo verificada.', auto_game_folder: 'Bibliotecas do Steam verificadas.',
        download_voices: result.voices_pending_application ? 'Vozes baixadas. Feche Deadlock e aplique a atualização.' : 'Vozes PT-BR baixadas e verificadas.',
        delete_voices: 'Vozes baixadas removidas. Vozes originais selecionadas.',
        update_client: 'Atualização baixada. Reiniciando o launcher…',
        check_updates: result.update_error ?? (result.client_update || result.voice_update ? 'Há atualizações disponíveis.' : result.update_repository ? 'Nenhuma versão mais recente encontrada.' : 'Atualizações online ainda não estão disponíveis nesta versão.'),
      };
      if (name === 'enable' || name === 'disable' || name === 'download_voices') { setNote(''); setLanguageConfirmation(result.language); }
      else setNote(messages[name] ?? 'Concluído.');
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
  const showSupportAvailability = useCallback(() => { setLanguageConfirmation(null); setDialog('support'); }, []);
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
    : data?.voices_pending_application ? 'Atualização pendente'
    : data?.voice_update ? 'Atualização disponível'
    : data?.language === 'pt-BR' && hero.available > 0 ? 'Instalado · ativo'
    : data?.pack_available ? 'Instalado · inativo' : 'Não instalado';
  const visible = heroes.filter(h => h.name.toLowerCase().includes(query.toLowerCase()) && (
    filter === 'all' || (filter === 'available' && h.content_available > 0) || (filter === 'in-progress' && contentState(h) === 'in-progress') || (filter === 'not-started' && contentState(h) === 'not-started')
  ));

  return <div className={`app-shell theme-${theme}`}>
    <div className="window-bar">
      <div className="drag-area" data-tauri-drag-region onDoubleClick={() => { if (desktop) void getCurrentWindow().toggleMaximize(); }}>
        <span className="tiny-brand" data-tauri-drag-region><span className="brand-diamond" /> DEADVOX</span>
      </div>
      {desktop && <div className="window-controls">
        <button aria-label="Minimizar janela" onClick={() => void getCurrentWindow().minimize()}><Minus size={14} /></button>
        <button aria-label="Maximizar janela" onClick={() => void getCurrentWindow().toggleMaximize()}><Maximize2 size={12} /></button>
        <button className="close-window" aria-label="Fechar janela" disabled={loading} title={loading ? 'Aguarde a operação atual' : 'Fechar'} onClick={() => void getCurrentWindow().close()}><X size={15} /></button>
      </div>}
    </div>
    <header className="app-header">
      <button className="brand" onClick={() => setPage(null)} aria-label="Início do DeadVox"><Wordmark /></button>
      <nav><button className="text-button" onClick={() => setPage('roadmap')}>Roadmap</button><button className="icon-button" title="Atualizações" aria-label="Atualizações" onClick={() => setDialog('updates')}><Download size={18} />{(data?.client_update || data?.voice_update) && <span className="update-dot" />}</button><button className="icon-button" title="Configurações" aria-label="Configurações" onClick={openSettings}><Settings2 size={19} /></button><span className="nav-divider" /><button className="text-button" onClick={() => setDialog('about')}>Sobre <ArrowUpRight size={14} /></button></nav>
    </header>

    {page === 'roadmap' ? <Roadmap heroes={heroes} onBack={() => setPage(null)} /> : !page ? <main className="home">
      <div className="home-intro"><span className="eyebrow">SEU JOGO. SUA VOZ.</span><h1>Escolha o idioma das vozes.</h1><p>Uma voz diferente. O mesmo Deadlock.</p></div>
      <div className="language-grid">
        {(['en', 'pt-BR'] as Language[]).map(language => {
          const selected = data?.language === language;
          return <article key={language} className={`language-card ${selected ? 'selected' : ''} ${busy ? 'changing' : ''}`}>
            <button className="language-hit-area" aria-label={`Selecionar ${languageName[language]}`} aria-pressed={selected} disabled={!canChange || (language === 'pt-BR' && !data?.pack_available && !selected)} onClick={() => selectLanguage(language)} />
            <div className="card-topline"><span className="card-eyebrow">{language === 'en' ? 'ÁUDIO ORIGINAL' : 'PACOTE DE VOZES PT-BR'}</span>{selected ? <span className="selected-check"><Check size={14} strokeWidth={2.2} /></span> : <span className="unselected-check" />}</div>
            <Flag country={language === 'en' ? 'us' : 'br'} />
            <div className="language-title"><h2>{languageLabel[language]}</h2><button className="edit-button" aria-label={`Ver ${languageName[language]}`} title="Ver personagens" onClick={() => openRoster(language)}><Pencil size={17} strokeWidth={1.6} /></button></div>
            <p>{languageName[language]}</p>
            {language === 'pt-BR' && data && (!data.pack_available || data.voice_update || data.voices_pending_application) && <button className="pack-download" disabled={loading || (data.voices_pending_application ? !canChange : !data.pack_download_available)} onClick={() => void perform(data.voices_pending_application ? 'enable' : 'download_voices')}><Download size={14} />{data.voices_pending_application ? 'Aplicar atualização' : data.voice_update ? 'Atualizar vozes' : 'Baixar vozes'}</button>}
            {language === 'pt-BR' && data?.pack_available && <button className="pack-download" disabled={!canChange} onClick={() => { if (window.confirm('Apagar as vozes PT-BR baixadas e voltar às vozes originais?')) void perform('delete_voices'); }}>Apagar vozes baixadas</button>}
          </article>;
        })}
      </div>
      {data?.game_running && <p className="inline-hint">Feche Deadlock para trocar o idioma das vozes.</p>}
      {data?.game_error && <button className="text-button" onClick={openSettings}><FolderOpen size={16} /> Localizar Deadlock</button>}
      {!data && !error && <div className="inline-hint"><LoaderCircle className="spin" size={16} /> Lendo suas configurações de voz…</div>}
    </main> : <main className="roster">
      <div className="page-heading"><div className="page-heading-left"><button className="back-button" aria-label="Voltar aos idiomas" onClick={() => setPage(null)}><ArrowLeft size={20} /></button><div><div className="eyebrow">BIBLIOTECA DE VOZES</div><h1>{isOriginal ? 'Vozes originais' : 'Pacote de vozes PT-BR'}</h1><p>{isOriginal ? 'O elenco original em inglês, incluído com Deadlock.' : 'Explore os personagens. Acompanhe o progresso do pacote de vozes.'}</p></div></div><button className="refresh-button" aria-label="Atualizar progresso" onClick={() => void refresh()} disabled={refreshing}><RefreshCw size={16} className={refreshing ? 'spin' : ''} /><span>Atualizar</span></button></div>
      <div className="roster-tools"><div className="filter-tabs" role="group" aria-label="Filtrar status dos personagens">
        <button className={filter === 'all' ? 'active' : ''} onClick={() => setFilter('all')}>Todos os personagens <span>{heroes.length}</span></button>
        {!isOriginal && <>
          <button className={filter === 'available' ? 'active' : ''} onClick={() => setFilter('available')}>Disponíveis <span>{available}</span></button>
          <button className={filter === 'in-progress' ? 'active' : ''} onClick={() => setFilter('in-progress')}>Em produção <span>{production}</span></button>
          <button className={filter === 'not-started' ? 'active' : ''} onClick={() => setFilter('not-started')}>Em breve <span>{notStarted}</span></button>
        </>}
      </div><label className="search-box"><Search size={16} /><input value={query} onChange={e => setQuery(e.target.value)} placeholder="Buscar personagens" aria-label="Buscar personagens" />{query && <button aria-label="Limpar busca" onClick={() => setQuery('')}><X size={14} /></button>}</label></div>
      <div className="list-header"><span>PERSONAGEM</span><span>{isOriginal ? 'IDIOMA DAS VOZES' : 'STATUS DO PACOTE'}</span><span>{isOriginal ? 'DISPONIBILIDADE' : 'COBERTURA'}</span><span /></div>
      <div className="hero-list">
        {visible.map(hero => <button className={`hero-row ${!isOriginal && contentState(hero) === 'not-started' ? 'coming-later' : ''}`} key={hero.id} onClick={() => setSelectedHero(hero)} aria-label={`Ver detalhes de ${hero.name}`}>
          <div className="hero-identity"><img src={hero.portrait} alt={hero.name} loading="lazy" /><span>{hero.name}</span></div>
          <div className="hero-state-block"><div className={`hero-state ${isOriginal ? 'complete' : contentState(hero)}`}><span className="status-dot" />{isOriginal ? 'Inglês · original' : names[contentState(hero)]}</div>{!isOriginal && installationLabel(hero) && <span className="installation-state">{installationLabel(hero)}</span>}</div>
          <div className="coverage-cell">{isOriginal ? <span className="original-available">Incluído no jogo</span> : <><div className="coverage-label"><span>{hero.content_available ? `${number.format(hero.content_available)} / ${number.format(hero.total)} falas PT-BR` : 'Pacote de vozes em breve'}</span><span>{hero.content_available && hero.total ? `${Math.round(1000 * hero.content_available / hero.total) / 10}%` : '—'}</span></div><div className={`progress-track ${contentState(hero)}`}><span style={{ width: hero.total ? `${100 * hero.content_available / hero.total}%` : '0%' }} /></div></>}</div>
          <ChevronRight size={16} className="row-arrow" />
        </button>)}
        {visible.length === 0 && <div className="empty-state"><Search size={24} /><h3>Nenhum personagem encontrado</h3><p>Tente outro nome ou status.</p><button className="text-button" onClick={() => { setQuery(''); setFilter('all'); }}>Redefinir filtros</button></div>}
      </div>
      <div className="roster-caption"><span>Retratos: Valve · Deadlock API</span><span>{isOriginal ? 'O áudio original é gerenciado pelo jogo.' : 'O conteúdo lançado e a instalação são mostrados separadamente.'}</span></div>
    </main>}

    {(error || data?.game_error) && <div className="notice error" role="alert"><CircleHelp size={17} /><span>{error || data?.game_error}</span><button className="text-button" onClick={() => void refresh()}>Tentar novamente</button></div>}
    {note && !error && <div className="notice" role="status"><Check size={16} /><span>{note}</span><button className="icon-button" aria-label="Fechar mensagem" onClick={() => setNote('')}><X size={14} /></button></div>}
    <footer className="app-footer"><div className={`footer-language ${data && !data.game_error ? 'ready' : ''}`}><span className="status-dot" /><span>{data ? data.language === 'en' ? 'Vozes originais selecionadas' : 'PT-BR selecionado' : 'Conectando…'}</span></div><div className="footer-actions">{page && <button className="secondary-button" onClick={() => setPage(null)}><ArrowLeft size={16} /> Voltar</button>}<button className={`play-button ${loading ? 'operation-button' : ''}`} disabled={loading || !data || !!data.game_error || data.game_running} onClick={() => void perform('play')} aria-label={loading ? `${progress?.phase ?? 'Carregando'} ${progressPercent === null ? '' : `${Math.round(progressPercent)}%`}` : 'Jogar Deadlock'}>{loading && <span className={`operation-fill ${progressPercent === null ? 'indeterminate' : ''}`} style={progressPercent === null ? undefined : { width: `${progressPercent}%` }} />}<span className="play-content">{loading ? <LoaderCircle size={17} className="spin" /> : <Play size={16} fill="currentColor" />}<span>{loading ? <><strong>{progressPercent === null ? 'Carregando…' : `${Math.round(progressPercent)}%`}</strong><small>{progress?.phase ?? 'Aguarde'}</small></> : data?.game_running ? 'Deadlock está em execução' : 'Jogar Deadlock'}</span>{!loading && <ArrowUpRight size={17} />}</span></button></div></footer>

    {languageConfirmation && <LanguageChangedDialog language={languageConfirmation} onClose={closeLanguageConfirmation} onSupport={showSupportAvailability} />}
    {dialog === 'support' && <Modal title="Apoie o DeadVox" onClose={closeDialog}>
      <div className="pix-support"><p>Leia este QR Code Pix com o aplicativo do seu banco para contribuir com o projeto.</p>
        <svg className="pix-filter" aria-hidden="true"><filter id="pix-deadvox-accent" colorInterpolationFilters="sRGB"><feComponentTransfer><feFuncR type="table" tableValues="0.663 1" /><feFuncG type="table" tableValues="0.082 1" /><feFuncB type="table" tableValues="0.11 1" /></feComponentTransfer></filter></svg>
        <div className="pix-qr-frame"><img src="/assets/support/contribution-qr.png" alt="QR Code Pix para apoiar DeadVox" width="320" height="323" /></div>
        <p className="small-copy">Obrigado por apoiar o DeadVox.</p>
      </div>
    </Modal>}
    {dialog === 'about' && <Modal title="Sobre o DeadVox" onClose={closeDialog}><div className="about-mark"><Wordmark /></div><div className="about-descriptor">Plataforma comunitária de dublagem</div><p>Dublagem comunitária para Deadlock, começando em português brasileiro. Um personagem por vez. Um idioma por vez.</p><p className="brand-signature">IA assistida • Direção humana</p><p>A tecnologia cuida da escala. As pessoas protegem a interpretação.</p><div className="detail-lines"><div><span>Versão</span><strong>{data?.version ?? '0.1.0-beta.2'}</strong></div><div><span>Retratos dos personagens</span><strong>Valve, via Deadlock API</strong></div><div><span>Pacote de vozes</span><strong>PT-BR · Português (Brasil)</strong></div><div><span>Domínio da marca</span><strong>deadvox.com</strong></div></div><p className="small-copy">DeadVox é um projeto comunitário independente, não afiliado, endossado nem patrocinado pela Valve Corporation. Deadlock, seus personagens, nomes e ativos relacionados pertencem aos respectivos proprietários.</p><button className="secondary-button" onClick={() => setDialog('notices')}>Avisos de terceiros</button></Modal>}
    {dialog === 'notices' && <Modal title="Avisos de terceiros" onClose={closeDialog}><ThirdPartyNotices /></Modal>}
    {dialog === 'settings' && <Modal title="Configurações" onClose={closeDialog}><section className="settings-section"><span className="settings-label">APARÊNCIA</span><h3>Tema de cores</h3><div className="theme-picker" role="group" aria-label="Tema de cores"><button className={`theme-option ${theme === 'dark' ? 'active' : ''}`} aria-pressed={theme === 'dark'} onClick={() => setTheme('dark')}><Moon size={17} /><span><strong>Escuro</strong><small>Preto profundo e carmesim</small></span></button><button className={`theme-option ${theme === 'light' ? 'active' : ''}`} aria-pressed={theme === 'light'} onClick={() => setTheme('light')}><Sun size={17} /><span><strong>Claro</strong><small>Marfim quente e tinta</small></span></button></div></section><section className="settings-section game-settings"><span className="settings-label">JOGO</span><h3>Pasta do Deadlock</h3><p>Encontre Deadlock nas suas bibliotecas Steam ou escolha a pasta de instalação.</p><label className="path-label" htmlFor="game-folder">Pasta do jogo</label><input id="game-folder" className="settings-input" value={gamePath} onChange={event => setGamePath(event.target.value)} placeholder="Selecione a pasta do Deadlock" disabled={loading} /><div className="settings-actions"><button className="secondary-button" disabled={loading || !desktop} onClick={() => void perform('set_game_folder')}><FolderOpen size={15} /> Procurar</button><button className="secondary-button" disabled={loading || !gamePath.trim() || !desktop} onClick={() => void perform('set_game_folder', { gamePath })}>Usar pasta</button><button className="text-button" disabled={loading || !desktop} onClick={() => void perform('auto_game_folder')}>Detectar pelo Steam</button></div><p className="small-copy">{data?.game_root ? `Detectado: ${data.game_root}` : 'Nenhuma pasta de jogo selecionada.'}</p></section><section className="settings-section update-settings"><span className="settings-label">ATUALIZAÇÕES</span><p className="small-copy">As atualizações vêm do canal oficial de lançamentos DeadVox. A fonte de download não pode ser alterada no launcher.</p><p className="small-copy">O launcher e os pacotes de vozes são atualizados separadamente. Feche o jogo antes de aplicar mudanças nas vozes.</p></section></Modal>}
    {dialog === 'updates' && <Modal title="Atualizações" onClose={closeDialog}><p>{data?.client_update || data?.voice_update ? 'Há novas versões disponíveis. Escolha o que atualizar.' : data?.update_repository ? 'O launcher e as vozes são verificados separadamente pelo canal oficial de lançamentos.' : 'Atualizações online ainda não estão disponíveis nesta versão.'}</p><div className="update-card"><div><h3>Launcher</h3><p>{data?.client_update ? `Versão ${data.client_update.version} · ${Math.ceil(data.client_update.bytes / 1048576)} MB` : `Instalado: ${data?.version ?? '—'}`}</p></div><button className="secondary-button" disabled={loading || !data?.client_update || !desktop} onClick={() => void perform('update_client')}><Download size={15} /> Atualizar e reiniciar</button></div><div className="update-card"><div><h3>Vozes PT-BR</h3><p>{data?.voice_update ? `Versão ${data.voice_update.version} · ${Math.ceil(data.voice_update.bytes / 1048576)} MB` : `Instalado: ${data?.pack_version ?? '—'}`}</p></div><button className="secondary-button" disabled={loading || (!data?.voice_update && !(!data?.pack_available && data?.pack_download_available)) || !desktop} onClick={() => void perform('download_voices')}><Download size={15} /> {data?.voice_update ? 'Atualizar vozes' : 'Baixar'}</button></div>{data?.update_error && <p className="small-copy" role="status">{data.update_error}</p>}<div className="settings-actions"><button className="secondary-button" disabled={loading || !data?.update_repository || !desktop} onClick={() => void perform('check_updates')}><RefreshCw size={15} /> Verificar atualizações</button></div></Modal>}
    {selectedHero && <Modal title={selectedHero.name} onClose={closeHero}><div className="hero-detail-intro"><img src={selectedHero.portrait} alt={selectedHero.name} /><div><div className="eyebrow">{isOriginal ? 'VOZES ORIGINAIS' : 'PACOTE DE VOZES PT-BR'}</div><h3>{isOriginal ? 'Incluído com Deadlock' : names[contentState(selectedHero)]}</h3></div></div>{isOriginal ? <p>Este personagem usa os arquivos de voz originais em inglês fornecidos pelo jogo.</p> : <><div className="detail-lines"><div><span>Falas PT-BR no pacote lançado</span><strong>{number.format(selectedHero.content_available)} falas</strong></div><div><span>Falas elegíveis para PT-BR</span><strong>{selectedHero.total ? number.format(selectedHero.total) : 'Ainda não anunciado'}</strong></div>{installationLabel(selectedHero) && <div><span>Instalação</span><strong>{installationLabel(selectedHero)}</strong></div>}</div><p className="small-copy">{contentState(selectedHero) === 'partial' ? 'O pacote inclui parte dos diálogos deste personagem. As falas restantes tocam em inglês.' : contentState(selectedHero) === 'complete' ? 'Todas as falas elegíveis desta versão estão cobertas. Sons originais e performances criativas são preservados.' : contentState(selectedHero) === 'in-progress' ? 'Este personagem está em produção. A data de lançamento ainda não foi anunciada.' : 'O pacote de vozes deste personagem estará disponível em breve. A data de lançamento ainda não foi anunciada.'}</p></>}</Modal>}
  </div>;
}
