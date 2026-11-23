import { useState } from 'react';

export type Section = 'general' | 'pronunciation' | 'shortcuts' | 'appearance' | 'saved' | 'data';
export const sections: { id: Section; label: string; task: string; description: string }[] = [
  { id: 'general', label: 'General', task: 'Reading', description: 'Startup and popup behaviour' },
  { id: 'pronunciation', label: 'Pronunciation', task: 'Speech', description: 'Accent, voice and playback' },
  { id: 'shortcuts', label: 'Shortcuts', task: 'Shortcuts', description: 'Activation and read aloud' },
  { id: 'appearance', label: 'Appearance', task: 'Appearance', description: 'Theme and motion' },
  { id: 'saved', label: 'Saved words', task: 'Saved words', description: 'Your vocabulary' },
  { id: 'data', label: 'Data', task: 'Data', description: 'Local storage and privacy' },
];

type Settings = { startup: boolean; expanded: boolean; reducedMotion: boolean; size: string; accent: string; autoplay: boolean; ttsEnabled: boolean; voice: string; speed: number; volume: number; keyboardEnabled: boolean; shortcut: string; middleClick: boolean; readerEnabled: boolean; readerShortcut: string; readerMaxChars: number; theme: string };
const requestedTheme = new URLSearchParams(location.search).get('theme');
const initial: Settings = { startup: false, expanded: false, reducedMotion: false, size: 'standard', accent: 'us', autoplay: false, ttsEnabled: true, voice: 'Heart', speed: 1, volume: 1, keyboardEnabled: true, shortcut: 'Ctrl+Alt+D', middleClick: false, readerEnabled: true, readerShortcut: 'Ctrl+Alt+R', readerMaxChars: 2000, theme: requestedTheme === 'dark' || requestedTheme === 'light' ? requestedTheme : 'system' };

export function useSettingsModel() {
  const start = new URLSearchParams(location.search).get('view') as Section;
  const [section, setSection] = useState<Section>(sections.some(item => item.id === start) ? start : 'general');
  const [settings, setSettings] = useState(initial);
  const [installed, setInstalled] = useState(false);
  const [notice, setNotice] = useState('');
  const [words, setWords] = useState(['serendipity', 'interdisciplinary', 'collisions']);
  const [selectedWord, setSelectedWord] = useState('');
  const change = (patch: Partial<Settings>) => { setSettings(current => ({ ...current, ...patch })); setNotice('Saved locally in this prototype.'); };
  return { section, setSection, settings, change, installed, setInstalled, notice, setNotice, words, setWords, selectedWord, setSelectedWord };
}
export type Model = ReturnType<typeof useSettingsModel>;

export function SettingsNav({ model, taskLabels = false }: { model: Model; taskLabels?: boolean }) {
  return <nav className="settings-nav" role="tablist" aria-label="Settings sections" aria-orientation="vertical" onKeyDown={event => {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    const index = sections.findIndex(item => item.id === model.section);
    const next = sections[(index + (event.key === 'ArrowDown' ? 1 : sections.length - 1)) % sections.length];
    model.setSection(next.id);
    requestAnimationFrame(() => document.getElementById(`settings-tab-${next.id}`)?.focus());
  }}>
    {sections.map(item => <button key={item.id} id={`settings-tab-${item.id}`} role="tab" aria-controls="settings-panel" aria-selected={model.section === item.id} tabIndex={model.section === item.id ? 0 : -1} onClick={() => { model.setSection(item.id); model.setNotice(''); }}><span>{taskLabels ? item.task : item.label}</span>{taskLabels && <small>{item.description}</small>}</button>)}
  </nav>;
}

function Row({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) { return <label className="setting-row"><span><strong>{label}</strong>{hint && <small>{hint}</small>}</span>{children}</label>; }
function Switch({ label, hint, checked, onChange }: { label: string; hint?: string; checked: boolean; onChange: (checked: boolean) => void }) { return <Row label={label} hint={hint}><input className="setting-switch" type="checkbox" role="switch" checked={checked} onChange={e => onChange(e.target.checked)}/></Row>; }
function Select({ label, value, options, onChange, hint }: { label: string; value: string; options: [string, string][]; onChange: (value: string) => void; hint?: string }) { return <Row label={label} hint={hint}><select value={value} onChange={e => onChange(e.target.value)}>{options.map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select></Row>; }
function Shortcut({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  const [capture, setCapture] = useState(false);
  return <Row label={label}><input className="shortcut-input" readOnly aria-label={label} value={capture ? 'Press key combination…' : value} onFocus={() => setCapture(true)} onBlur={() => setCapture(false)} onKeyDown={event => {
    event.preventDefault();
    if (event.key === 'Escape') { event.currentTarget.blur(); return; }
    if ((!event.ctrlKey && !event.altKey && !event.metaKey) || ['Control', 'Alt', 'Meta', 'Shift'].includes(event.key)) return;
    const key = event.code.replace(/^(Key|Digit)/, '');
    onChange(`${event.ctrlKey ? 'Ctrl+' : ''}${event.altKey ? 'Alt+' : ''}${event.shiftKey ? 'Shift+' : ''}${event.metaKey ? 'Super+' : ''}${key}`);
    event.currentTarget.blur();
  }}/></Row>;
}

export function SettingsPanel({ model, taskLabels = false }: { model: Model; taskLabels?: boolean }) {
  const s = model.settings;
  const update = model.change;
  const current = sections.find(item => item.id === model.section)!;
  return <section className="settings-panel" id="settings-panel" role="tabpanel" aria-labelledby={`settings-tab-${model.section}`}>
    <div className="settings-heading"><span className="settings-eyebrow">DIO Settings</span><h1>{taskLabels ? current.task : current.label}</h1><p>{current.description}.</p></div>
    {model.notice && <p className="settings-notice" role="status">{model.notice}</p>}
    <div className="settings-controls">{model.section === 'general' ? <>
      <Switch label="Start DIO when I sign in" checked={s.startup} onChange={startup => update({ startup })}/>
      <Switch label="Show expanded definitions by default" checked={s.expanded} onChange={expanded => update({ expanded })}/>
      <Switch label="Reduce motion" checked={s.reducedMotion} onChange={reducedMotion => update({ reducedMotion })}/>
      <Select label="Popup size" value={s.size} options={[["small","Small"],["standard","Standard"],["large","Large"]]} onChange={size => update({ size })}/>
    </> : model.section === 'pronunciation' ? <>
      <Select label="Accent" hint="Shows available IPA in this accent first." value={s.accent} options={[["us","US English"],["uk","UK English"]]} onChange={accent => update({ accent })}/>
      <Switch label="Auto-play pronunciation" checked={s.autoplay} onChange={autoplay => update({ autoplay })}/>
      <Switch label="Use the Kokoro voice" hint="For pronunciation and reading selected text." checked={s.ttsEnabled} onChange={ttsEnabled => update({ ttsEnabled })}/>
      <div className="voice-status"><div><strong>{model.installed ? 'Voice ready' : 'Voice model not installed'}</strong><small>{model.installed ? 'Speech runs on this computer.' : 'Download once, about 95 MB.'}</small></div><button onClick={() => { model.setInstalled(!model.installed); model.setNotice(model.installed ? 'Voice removed from this prototype.' : 'Voice installed in this prototype.'); }}>{model.installed ? 'Remove voice' : 'Install voice'}</button></div>
      <Select label="Voice" value={s.voice} options={[["Heart","Heart (US)"],["Emma","Emma (UK)"],["Michael","Michael (US)"]]} onChange={voice => update({ voice })}/>
      <Row label="Speed"><span className="settings-range"><input aria-label="Speed" type="range" min="0.5" max="2" step="0.05" value={s.speed} onChange={e => update({ speed: Number(e.target.value) })}/><output>{s.speed.toFixed(2)}×</output></span></Row>
      <Row label="Volume"><span className="settings-range"><input aria-label="Volume" type="range" min="0" max="1" step="0.05" value={s.volume} onChange={e => update({ volume: Number(e.target.value) })}/><output>{Math.round(s.volume * 100)}%</output></span></Row>
      <div className="voice-test"><p>Installing downloads the model once. Text read aloud stays on this computer.</p><button disabled={!model.installed || !s.ttsEnabled} onClick={() => { model.setNotice('Playing a browser voice preview. The desktop app uses Kokoro.'); if ('speechSynthesis' in window) { speechSynthesis.cancel(); speechSynthesis.speak(new SpeechSynthesisUtterance('This is how DIO reads selected text aloud.')); } }}>Test voice</button></div>
    </> : model.section === 'shortcuts' ? <>
      <Switch label="Keyboard shortcut" checked={s.keyboardEnabled} onChange={keyboardEnabled => update({ keyboardEnabled })}/>
      <Shortcut label="Lookup shortcut" value={s.shortcut} onChange={shortcut => update({ shortcut })}/>
      <Switch label="Middle click activation" checked={s.middleClick} onChange={middleClick => update({ middleClick })}/>
      <Switch label="Enable read-aloud shortcut" checked={s.readerEnabled} onChange={readerEnabled => update({ readerEnabled })}/>
      <Shortcut label="Read-aloud shortcut" value={s.readerShortcut} onChange={readerShortcut => update({ readerShortcut })}/>
      <Row label="Longest selection" hint="Longer selections are refused rather than cut off."><input className="number-input" type="number" min="100" max="10000" step="100" value={s.readerMaxChars} onChange={e => update({ readerMaxChars: Number(e.target.value) })}/></Row>
    </> : model.section === 'appearance' ? <>
      <Select label="Theme" value={s.theme} options={[["system","System"],["light","Light"],["dark","Dark"]]} onChange={theme => update({ theme })}/>
      <Switch label="Reduce motion" checked={s.reducedMotion} onChange={reducedMotion => update({ reducedMotion })}/>
    </> : model.section === 'saved' ? <div className="saved-words">{model.words.length ? model.words.map(word => <button key={word} onClick={() => model.setSelectedWord(word)}>{word} ↗</button>) : <p>No saved words yet.</p>}{model.selectedWord && <article className="word-preview"><button aria-label="Close word preview" onClick={() => model.setSelectedWord('')}>×</button><h2>{model.selectedWord}</h2><p>{({ serendipity: 'The occurrence of a pleasant discovery by chance.', interdisciplinary: 'Involving more than one area of study.', collisions: 'Instances of colliding.' } as Record<string,string>)[model.selectedWord]}</p></article>}</div> : <>
      <div className="voice-status"><div><strong>Dictionary cache</strong><small>Stored on this computer.</small></div><button onClick={() => model.setNotice('Dictionary cache cleared in this prototype. Saved words remain.')}>Clear dictionary cache</button></div>
      <p className="data-note">DIO checks its local cache, then requests definitions from Simple English Wiktionary and the Free Dictionary API. Its bundled dictionary is available offline. Saved words and settings stay on this computer.</p>
    </>}</div>
  </section>;
}
