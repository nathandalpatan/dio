import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource/inter/400.css';
import '@fontsource/inter/500.css';
import '@fontsource/inter/600.css';
import '@fontsource/source-serif-4/400.css';
import '@fontsource/source-serif-4/400-italic.css';
import '@fontsource/source-serif-4/600.css';
import '@fontsource/source-serif-4/700.css';
import Quiet from './Quiet';
import WordStage from './WordStage';
import FastLookup from './FastLookup';
import { entries, type PreviewActions, type PreviewModel, type PreviewStatus } from './shared';
import './style.css';

const variants = [
  { name: 'Quiet Reference', component: Quiet, axis: 'Minimal reading surface' },
  { name: 'Word Stage', component: WordStage, axis: 'Expressive hierarchy' },
  { name: 'Fast Lookup', component: FastLookup, axis: 'Information efficiency' },
] as const;
type Scenario = 'truth' | 'serendipity' | 'interdisciplinary' | 'loading' | 'not-found' | 'error' | 'reading';
type History = { words: string[]; index: number };

export function App() {
  const initialVariant = Number(new URLSearchParams(location.search).get('v'));
  const [variant, setVariant] = useState(initialVariant >= 1 && initialVariant <= variants.length ? initialVariant - 1 : 0);
  const [mountKey, setMountKey] = useState(0);
  const [scenario, setScenario] = useState<Scenario>('truth');
  const [history, setHistory] = useState<History>({ words: ['truth'], index: 0 });
  const [visible, setVisible] = useState(true);
  const [expanded, setExpanded] = useState(new URLSearchParams(location.search).get('expanded') === '1');
  const [saved, setSaved] = useState<Set<string>>(() => new Set());
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [speaking, setSpeaking] = useState(false);
  const [speechMessage, setSpeechMessage] = useState('');
  const [accent, setAccent] = useState<'en-US' | 'en-GB'>('en-US');
  const [voice, setVoice] = useState<'default' | 'female' | 'male'>('default');
  const [themeChoice, setThemeChoice] = useState<'default' | 'light' | 'dark'>('default');
  const picker = useRef<HTMLElement>(null);
  const highlight = useRef<HTMLSpanElement>(null);

  const selectedWord = history.words[history.index];
  const entry = scenario === 'truth' || scenario === 'serendipity' || scenario === 'interdisciplinary' ? entries[selectedWord] : undefined;
  const status: PreviewStatus = entry ? 'ready' : scenario === 'not-found' ? 'not-found' : scenario === 'error' ? 'error' : scenario === 'reading' ? 'reading' : 'loading';
  const theme = themeChoice === 'default' ? variant === 0 ? 'light' : 'dark' : themeChoice;
  const Variant = variants[variant].component;

  function chooseVariant(next: number) {
    if (next < 0 || next >= variants.length) return;
    setVariant(next);
    setMountKey(current => current + 1);
    const url = new URL(location.href);
    url.searchParams.set('v', String(next + 1));
    window.history.replaceState(null, '', url);
  }

  useLayoutEffect(() => {
    const update = () => {
      const item = picker.current?.querySelectorAll<HTMLButtonElement>('.proto-picker-item')[variant];
      if (!item || !highlight.current) return;
      highlight.current.style.width = `${item.offsetWidth}px`;
      highlight.current.style.transform = `translateX(${item.offsetLeft}px)`;
    };
    update();
    window.addEventListener('resize', update);
    return () => window.removeEventListener('resize', update);
  }, [variant]);

  useEffect(() => {
    const frame = requestAnimationFrame(() => requestAnimationFrame(() => picker.current?.setAttribute('data-ready', '')));
    return () => cancelAnimationFrame(frame);
  }, []);

  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (event.defaultPrevented || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName) || target.isContentEditable || event.metaKey || event.ctrlKey || event.altKey) return;
      const number = Number(event.key);
      if (number >= 1 && number <= variants.length) chooseVariant(number - 1);
      else if (event.key === 'ArrowRight') chooseVariant((variant + 1) % variants.length);
      else if (event.key === 'ArrowLeft') chooseVariant((variant - 1 + variants.length) % variants.length);
      else if (event.key === 'Escape' && visible) {
        if (settingsOpen) setSettingsOpen(false);
        else setVisible(false);
      }
    };
    document.addEventListener('keydown', key);
    return () => document.removeEventListener('keydown', key);
  }, [variant, visible, settingsOpen]);

  function stopSpeech() {
    window.speechSynthesis?.cancel();
    setSpeaking(false);
    setSpeechMessage('Speech stopped.');
  }

  function speak(text: string) {
    if (!('speechSynthesis' in window)) {
      setSpeechMessage('Speech preview is unavailable in this browser.');
      return;
    }
    window.speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = accent;
    const available = window.speechSynthesis.getVoices().filter(item => item.lang.toLowerCase().startsWith(accent.toLowerCase().slice(0, 2)));
    if (voice !== 'default' && available.length) utterance.voice = available[voice === 'female' ? 0 : Math.min(1, available.length - 1)];
    utterance.onstart = () => { setSpeaking(true); setSpeechMessage('Speaking with browser voice · prototype preview'); };
    utterance.onend = () => { setSpeaking(false); setSpeechMessage('Speech finished.'); };
    utterance.onerror = () => { setSpeaking(false); setSpeechMessage('Browser speech could not play.'); };
    window.speechSynthesis.speak(utterance);
    setSpeechMessage('Starting browser speech preview…');
  }

  function chooseScenario(next: Scenario) {
    stopSpeech();
    setScenario(next);
    if (next in entries) setHistory({ words: [next], index: 0 });
    else setHistory({ words: ['truth'], index: 0 });
    setVisible(true);
    setExpanded(false);
    setSettingsOpen(false);
    setMountKey(current => current + 1);
  }

  function closeLookup() {
    stopSpeech();
    setSettingsOpen(false);
    setVisible(false);
  }

  function moveHistory(step: number) {
    if (history.index + step < 0 || history.index + step >= history.words.length) return;
    stopSpeech();
    setSettingsOpen(false);
    setHistory(current => ({ ...current, index: current.index + step }));
  }

  const actions: PreviewActions = {
    back: () => {
      if (history.index > 0) moveHistory(-1);
      else closeLookup();
    },
    forward: () => moveHistory(1),
    close: closeLookup,
    save: () => {
      if (!entry) return;
      setSaved(current => { const next = new Set(current); if (next.has(entry.word)) next.delete(entry.word); else next.add(entry.word); return next; });
    },
    pronounce: () => entry && speak(entry.word),
    read: () => entry && speak(`${entry.word}. ${entry.primary}`),
    stop: stopSpeech,
    expand: () => { setSettingsOpen(false); setExpanded(current => !current); },
    lookup: word => {
      if (!entries[word]) return;
      stopSpeech();
      setScenario('truth');
      setHistory(current => ({ words: [...current.words.slice(0, current.index + 1), word], index: current.index + 1 }));
      setExpanded(false);
      setSettingsOpen(false);
      setMountKey(current => current + 1);
    },
    settings: () => setSettingsOpen(current => !current),
    setAccent,
    setVoice,
  };
  const model: PreviewModel = { entry, status, expanded, saved: !!entry && saved.has(entry.word), canBack: history.index > 0, canForward: history.index < history.words.length - 1, settingsOpen, speaking, speechMessage, accent, voice };

  return <main className={`prototype-page theme-${theme}`}>
    <header className="prototype-toolbar"><div><strong>DIO / Lookup concepts</strong><span>{variants[variant].axis}</span></div><div className="prototype-toolbar__controls"><label>Scenario<select value={scenario} onChange={event => chooseScenario(event.target.value as Scenario)}><option value="truth">Truth · full entry</option><option value="serendipity">Serendipity · origin</option><option value="interdisciplinary">Long word</option><option value="loading">Loading</option><option value="not-found">Not found</option><option value="error">Lookup error</option><option value="reading">Reading aloud</option></select></label><label>Appearance<select value={themeChoice} onChange={event => setThemeChoice(event.target.value as typeof themeChoice)}><option value="default">Concept default</option><option value="light">Light</option><option value="dark">Dark</option></select></label></div></header>
    <div className="prototype-context"><div className="context-window"><div className="context-window__bar"><span className="context-window__dots" aria-hidden="true">● ● ●</span><span>Reading notes.txt</span></div><div className="context-window__body"><span className="context-kicker">A selected word in another app</span><p>It is a <button className="context-selected" onClick={() => chooseScenario('truth')}>truth</button> universally acknowledged, that a single man in possession of a good fortune, must be in want of a wife.</p><p className="context-hint">Select a scenario above or click the highlighted word to reopen the lookup.</p></div></div>
      {visible ? <div className={`preview-anchor ${expanded ? 'is-expanded' : ''}`}><Variant key={`${variant}-${mountKey}`} model={model} actions={actions} /></div> : <div className="preview-closed"><strong>Lookup closed</strong><p>Click the selected word or choose a scenario to open it again.</p><button onClick={() => setVisible(true)}>Open lookup</button></div>}
    </div>
    <p className="prototype-note" role="status" aria-live="polite">{speechMessage || 'Prototype only. Dictionary content is local fixture data; speech uses your browser.'}</p>
    <nav className="proto-picker" aria-label="Prototype variants" ref={picker}>
      <span className="proto-picker-highlight" aria-hidden="true" ref={highlight}></span>
      {variants.map((item, index) => <button key={item.name} className="proto-picker-item" data-active={variant === index ? '' : undefined} aria-current={variant === index ? 'true' : undefined} onClick={() => chooseVariant(index)}>{item.name}</button>)}
    </nav>
  </main>;
}

const root = document.getElementById('prototype-root');
if (root) createRoot(root).render(<App />);
