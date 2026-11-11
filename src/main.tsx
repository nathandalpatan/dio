import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './style.css';

type License = { name: string; url: string };
type Phonetic = { text: string; audio: string; sourceUrl: string; license?: License };
type Definition = { definition: string; example: string; synonyms: string[]; antonyms: string[] };
type Meaning = { partOfSpeech: string; definitions: Definition[]; synonyms: string[]; antonyms: string[] };
type Entry = { word: string; phonetic: string; phonetics: Phonetic[]; meanings: Meaning[]; origin: string; sourceUrls: string[]; license?: License };
type PopupState = { requestId: number; status: string; query: string; entries: Entry[]; message: string; stale: boolean; expanded: boolean; direction: string; canBack: boolean; saved: boolean };
type Settings = { shortcut: string; keyboardEnabled: boolean; middleClick: boolean; theme: string; size: string; expanded: boolean; autoplay: boolean; pronunciation: string; startup: boolean; reducedMotion: boolean };

const empty: PopupState = { requestId: 0, status: 'idle', query: '', entries: [], message: '', stale: false, expanded: false, direction: 'se', canBack: false, saved: false };
const messageFor: Record<string, string> = {
  idle: 'Highlight text and press Ctrl+Alt+D.',
  loading: 'Looking up definition…',
  unsupported: 'This app does not expose its selection to Windows accessibility.',
  notFound: 'No definition found.',
  timeout: 'The selection or dictionary request timed out.',
  network: 'The dictionary is unavailable.',
  storage: 'Could not access local dictionary data.',
};
const arrows: Record<string, string> = { se: '↘', sw: '↙', ne: '↗', nw: '↖' };
const opposite: Record<string, string> = { se: '↖', sw: '↗', ne: '↙', nw: '↘' };
const call = <T,>(command: string, args?: Record<string, unknown>) => invoke<T>(command, args);
const asError = (error: unknown) => typeof error === 'string' ? error : String(error);
const safeUrl = (address: string) => { try { const url = new URL(address); return url.protocol === 'https:' ? url.href : null; } catch { return null; } };

export function PopupView() {
  const [state, setState] = useState<PopupState>(empty);
  const [error, setError] = useState('');
  const [settings, setSettings] = useState<Settings | null>(null);
  useEffect(() => {
    let eventSeen = false;
    const subscription = listen<PopupState>('popup-state', event => { eventSeen = true; setState(current => event.payload.requestId >= current.requestId ? event.payload : current); setError(''); });
    call<PopupState>('popup_state').then(value => { if (!eventSeen) setState(value); }).catch(error => setError(asError(error)));
    call<Settings>('get_settings').then(setSettings).catch(error => setError(asError(error)));
    const preferences = listen<Settings>('settings-changed', event => setSettings(event.payload));
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); void call('close_popup'); }
      if (event.key === 'Backspace' && !['INPUT', 'TEXTAREA'].includes((event.target as HTMLElement).tagName)) { event.preventDefault(); void call('back'); }
    };
    window.addEventListener('keydown', key);
    return () => { window.removeEventListener('keydown', key); void subscription.then(unlisten => unlisten()); void preferences.then(unlisten => unlisten()); };
  }, []);
  const entry = state.entries[0];
  const primary = entry?.meanings.find(meaning => meaning.definitions.length > 0);
  const phonetic = entry?.phonetics.find(item => item.text)?.text || entry?.phonetic;
  const availableAudio = entry?.phonetics.filter(item => item.audio) || [];
  const preferredLocale = settings?.pronunciation === 'us' ? /(?:^|[_/.-])us(?:[_/.-]|$)/i : /(?:^|[_/.-])(?:uk|gb)(?:[_/.-]|$)/i;
  const audio = (settings?.pronunciation === 'first' ? availableAudio[0] : availableAudio.find(item => preferredLocale.test(item.audio))) || availableAudio[0];
  const more = async (word: string) => { setError(''); try { await call('lookup', { word }); } catch (e) { setError(asError(e)); } };
  const play = async () => {
    if (!audio) return;
    let source: string | undefined;
    try {
      const bytes = await call<ArrayBuffer>('pronunciation', { url: audio.audio });
      source = URL.createObjectURL(new Blob([bytes]));
      const player = new Audio(source);
      player.onended = () => URL.revokeObjectURL(source!);
      player.onerror = () => { URL.revokeObjectURL(source!); setError('Could not play pronunciation.'); };
      await player.play();
    } catch (e) { if (source) URL.revokeObjectURL(source); setError(asError(e)); }
  };
  useEffect(() => { if (settings?.autoplay && state.status === 'ready' && audio) void play(); }, [state.requestId, state.status, settings?.autoplay]);
  const toggle = async () => { try { await call('expand', { expanded: !state.expanded, reducedMotion: window.matchMedia('(prefers-reduced-motion: reduce)').matches }); } catch (e) { setError(asError(e)); } };
  const save = async () => { try { await call('save_word', { save: !state.saved }); } catch (e) { setError(asError(e)); } };
  return <main className={`popup ${state.expanded ? 'expanded' : ''} theme-${settings?.theme || 'system'} ${settings?.reducedMotion ? 'reduced-motion' : ''}`}>
    <header className="popup-header">
      <button className="icon-button" aria-label="Back" disabled={!state.canBack} onClick={() => void call('back').catch(e => setError(asError(e)))}>←</button>
      <span className="brand">DIO</span>
      <button className="icon-button" aria-label="Close" onClick={() => void call('close_popup')}>×</button>
    </header>
    <div className="content">
      {state.status === 'ready' && entry ? <>
        <div className="word-row"><h1>{entry.word || state.query}</h1><button className="save-button" onClick={() => void save()} aria-label={state.saved ? 'Remove saved word' : 'Save word'}>{state.saved ? '★' : '☆'}</button></div>
        <div className="phonetic-row">{phonetic && <span>{phonetic}</span>}{audio && <button className="audio-button" onClick={() => void play()} aria-label="Play pronunciation">▶ Listen</button>}</div>
        {primary && <section className="primary"><span className="part">{primary.partOfSpeech}</span><p>{primary.definitions[0]?.definition}</p></section>}
        {state.stale && <p className="notice">{state.message}</p>}
        {state.expanded && <div className="details">
          {entry.meanings.map((meaning, index) => <section key={`${meaning.partOfSpeech}-${index}`}><h2>{meaning.partOfSpeech}</h2><ol>{meaning.definitions.map((definition, i) => <li key={i}>{definition.definition}{definition.example && <p className="example">“{definition.example}”</p>}</li>)}</ol>
            {([['Synonyms', meaning.synonyms.concat(meaning.definitions.flatMap(definition => definition.synonyms))], ['Antonyms', meaning.antonyms.concat(meaning.definitions.flatMap(definition => definition.antonyms))]] as [string, string[]][]).map(([label, values]) => values.length > 0 && <div className="related" key={label}><strong>{label}</strong><div>{[...new Set(values)].map(word => <button key={word} onClick={() => void more(word)}>{word}</button>)}</div></div>)}
          </section>)}
          {entry.origin && <section><h2>Origin</h2><p>{entry.origin}</p></section>}
          {(entry.sourceUrls.length > 0 || entry.license || audio?.license || audio?.sourceUrl) && <footer className="source">Source: {entry.sourceUrls.map(url => safeUrl(url) && <a key={url} href={safeUrl(url)!} target="_blank" rel="noreferrer">Entry source</a>)}{entry.license && <> · {safeUrl(entry.license.url) ? <a href={safeUrl(entry.license.url)!} target="_blank" rel="noreferrer">{entry.license.name}</a> : entry.license.name}</>}{audio?.sourceUrl && safeUrl(audio.sourceUrl) && <> · <a href={safeUrl(audio.sourceUrl)!} target="_blank" rel="noreferrer">Audio source</a></>}{audio?.license && <> · Audio: {safeUrl(audio.license.url) ? <a href={safeUrl(audio.license.url)!} target="_blank" rel="noreferrer">{audio.license.name}</a> : audio.license.name}</>}</footer>}
        </div>}
      </> : <div className="status"><span>{state.status === 'loading' ? '◌' : '⌕'}</span><p>{state.message || messageFor[state.status] || 'Choose a word to look up.'}</p></div>}
      {error && <p className="notice error" role="alert">{error}</p>}
    </div>
    <footer className="popup-footer"><span>{state.status === 'ready' && !state.expanded ? primary?.partOfSpeech : ''}</span><button className="expand-button" onClick={() => void toggle()}>{state.expanded ? opposite[state.direction] : arrows[state.direction]} {state.expanded ? 'Less' : 'More'}</button></footer>
  </main>;
}

function SettingsView() {
  const [draft, setDraft] = useState<Settings | null>(null);
  const [saved, setSaved] = useState<string[]>([]);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [capturing, setCapturing] = useState(false);
  useEffect(() => {
    void call<Settings>('get_settings').then(setDraft).catch(e => setError(asError(e)));
    void call<string[]>('saved_words').then(setSaved).catch(e => setError(asError(e)));
    void call<string | null>('shortcut_error').then(value => { if (value) setError(value); });
    void call<string | null>('hook_error').then(value => { if (value) setError(value); });
    const subscription = listen('saved-changed', () => { void call<string[]>('saved_words').then(setSaved).catch(e => setError(asError(e))); });
    return () => { void subscription.then(unlisten => unlisten()); };
  }, []);
  const change = (patch: Partial<Settings>) => setDraft(current => current && { ...current, ...patch });
  const capture = (event: React.KeyboardEvent<HTMLInputElement>) => {
    event.preventDefault();
    if (event.key === 'Escape') { setCapturing(false); return; }
    if (!event.ctrlKey && !event.altKey && !event.metaKey) return;
    if (['Control', 'Alt', 'Meta', 'Shift'].includes(event.key)) return;
    const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
    change({ shortcut: `${event.ctrlKey ? 'Ctrl+' : ''}${event.altKey ? 'Alt+' : ''}${event.shiftKey ? 'Shift+' : ''}${event.metaKey ? 'Super+' : ''}${key}` });
    setCapturing(false);
  };
  const save = async () => { if (!draft) return; setError(''); try { await call('update_settings', { next: draft }); setNotice('Settings saved.'); } catch (e) { setError(asError(e)); } };
  const clear = async () => { setError(''); try { await call('clear_cache'); setNotice('Dictionary cache cleared. Saved words remain.'); } catch (e) { setError(asError(e)); } };
  if (!draft) return <main className="settings"><p>Loading settings…</p>{error && <p role="alert">{error}</p>}</main>;
  return <main className="settings"><header><span className="brand">DIO</span><h1>Settings</h1><p>Look up highlighted text from any app that exposes its selection to Windows.</p></header>
    <section><h2>Activation</h2><label className="check"><input type="checkbox" checked={draft.keyboardEnabled} onChange={e => change({ keyboardEnabled: e.target.checked })}/> Keyboard shortcut</label>
      <label>Shortcut<input value={capturing ? 'Press a key combination…' : draft.shortcut} readOnly onFocus={() => setCapturing(true)} onBlur={() => setCapturing(false)} onKeyDown={capture}/></label>
      <label className="check"><input type="checkbox" checked={draft.middleClick} onChange={e => change({ middleClick: e.target.checked })}/> Middle click activation</label><p className="hint">The source app may also handle the middle click normally.</p></section>
    <section><h2>Appearance</h2><label>Theme<select value={draft.theme} onChange={e => change({ theme: e.target.value })}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label>
      <label>Popup size<select value={draft.size} onChange={e => change({ size: e.target.value })}><option value="small">Small</option><option value="standard">Standard</option><option value="large">Large</option></select></label>
      <label className="check"><input type="checkbox" checked={draft.expanded} onChange={e => change({ expanded: e.target.checked })}/> Show expanded definitions by default</label>
      <label className="check"><input type="checkbox" checked={draft.reducedMotion} onChange={e => change({ reducedMotion: e.target.checked })}/> Reduce motion</label></section>
    <section><h2>Pronunciation</h2><label>Preferred audio<select value={draft.pronunciation} onChange={e => change({ pronunciation: e.target.value })}><option value="first">First available</option><option value="us">US English</option><option value="uk">UK English</option></select></label><label className="check"><input type="checkbox" checked={draft.autoplay} onChange={e => change({ autoplay: e.target.checked })}/> Play automatically</label></section>
    <section><h2>System</h2><label className="check"><input type="checkbox" checked={draft.startup} onChange={e => change({ startup: e.target.checked })}/> Start DIO when I sign in</label></section>
    <section><h2>Saved vocabulary</h2>{saved.length ? <div className="saved-list">{saved.map(word => <button key={word} onClick={() => void call('open_saved', { word }).catch(e => setError(asError(e)))}>{word}</button>)}</div> : <p className="hint">No saved words yet.</p>}</section>
    <section><h2>Local data</h2><button className="secondary" onClick={() => void clear()}>Clear dictionary cache</button><p className="hint">Selections are sent only to the Free Dictionary API for lookup. Saved words and settings stay on this computer.</p></section>
    {error && <p className="notice error" role="alert">{error}</p>}{notice && <p className="notice">{notice}</p>}
    <div className="settings-actions"><button onClick={() => void save()}>Save settings</button></div>
  </main>;
}

const root = document.getElementById('root');
if (root) createRoot(root).render(window.location.search.includes('settings') ? <SettingsView/> : <PopupView/>);
