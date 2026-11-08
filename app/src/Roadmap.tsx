import { useEffect, useRef, useState } from 'react';
import { ArrowLeft, Check } from 'lucide-react';
import { desktop, roadmapAudio } from './api';
import type { Hero } from './types';

const items = [
  { id: 'ghost', name: 'Lady Geist', description: 'Uma ocultista formidável que extrai poder do mundo espiritual.' },
  { id: 'announcer', name: 'Archmother', description: 'Uma patrona que guia sua equipe durante a batalha.' },
  { id: 'hidden_king', name: 'Hidden King', description: 'Um patrono cuja voz guia seus aliados durante a partida.' },
  { id: 'spirit_jar', name: 'The Urn', description: 'O espírito falante carregado pela cidade.' },
  { id: 'shopkeeper', name: 'The Shopkeeper', description: 'O dono da Loja de Curiosidades que abastece os heróis.' },
  { id: 'priest', name: 'Venator', description: 'Um caçador que entra na batalha por Nova York.' },
  { id: 'drifter', name: 'Drifter', description: 'Um lutador habilidoso que percorre as ruas de Nova York.' },
];

export default function Roadmap({ heroes, onBack }: { heroes: Hero[]; onBack: () => void }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [message, setMessage] = useState('');
  const audio = useRef<HTMLAudioElement | null>(null);
  const objectUrl = useRef<string | null>(null);
  const request = useRef(0);
  const stop = () => { request.current++; audio.current?.pause(); audio.current = null; if (objectUrl.current) URL.revokeObjectURL(objectUrl.current); objectUrl.current = null; };
  useEffect(() => () => stop(), []);
  const select = async (id: string) => {
    stop(); setSelected(id); setMessage('');
    if (!desktop) { setMessage('A prévia de voz está disponível no launcher para desktop.'); return; }
    if ((heroes.find(hero => hero.asset_id === id)?.content_available ?? 0) === 0) { setMessage('Ainda não há uma prévia de voz escolhida para este personagem.'); return; }
    const current = request.current;
    try {
      const bytes = await roadmapAudio(id, true);
      if (request.current !== current) return;
      const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: 'audio/mpeg' }));
      objectUrl.current = url; audio.current = new Audio(url);
      await audio.current.play();
    } catch { if (request.current === current) setMessage('A prévia de voz escolhida e verificada não está disponível neste pacote.'); }
  };
  const item = items.find(entry => entry.id === selected);
  const complete = (id: string) => { const h = heroes.find(hero => hero.asset_id === id); return !!h && h.total > 0 && h.content_available === h.total; };
  return <main className="roadmap-page">
    <button className="back-button roadmap-back" onClick={onBack} aria-label="Voltar ao launcher"><ArrowLeft size={18} /> Voltar</button>
    <div className="roadmap-heading"><h1>Roadmap</h1><p>O caminho à frente para Deadlock.</p></div>
    <div className="roadmap-cards" role="group" aria-label="Personagens do roadmap">
      {items.map((entry, index) => <button key={entry.id} className={`roadmap-card ${complete(entry.id) ? 'is-complete' : ''} ${selected === entry.id ? 'is-selected' : ''}`} onClick={() => void select(entry.id)} aria-pressed={selected === entry.id} aria-label={`${entry.name}${complete(entry.id) ? ', concluído' : ''}: selecionar e reproduzir voz`}>
        <img src={`/assets/roadmap/portrait-${index}.webp`} alt="" /><span className="roadmap-card-label"><span className="roadmap-complete">{complete(entry.id) && <><Check size={15} /> Concluído</>}</span><strong>{entry.name}</strong></span>
      </button>)}
    </div>
    {item && <section className={`roadmap-detail ${complete(item.id) ? 'is-complete' : ''}`} aria-live="polite"><h2>{item.name}</h2><span className="roadmap-detail-divider" /><p>{item.description}</p></section>}
    {message && <p className="roadmap-message" role="status">{message}</p>}
  </main>;
}
