import { useEffect, useRef, useState } from 'react';

export const download = 'https://github.com/nathandalpatan/dio/releases/latest/download/DIO-setup.exe';
export const releases = 'https://github.com/nathandalpatan/dio/releases';
const shot = (name: string) => `../../site/assets/shots/${name}.jpg`;

export function SiteNav({ concept, quietMenu = false }: { concept: string; quietMenu?: boolean }) {
  const [open, setOpen] = useState(false);
  const header = useRef<HTMLElement>(null);
  const menu = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => { if (!header.current?.contains(event.target as Node)) setOpen(false); };
    const onKeyDown = (event: KeyboardEvent) => { if (event.key === 'Escape') { setOpen(false); menu.current?.focus(); } };
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
    return () => { document.removeEventListener('pointerdown', onPointerDown); document.removeEventListener('keydown', onKeyDown); };
  }, [open]);
  return <header className={`site-nav ${quietMenu ? 'site-nav--quiet-menu' : ''}`} ref={header}>
    <a className="site-logo" href="#top">DIO</a><span className="site-concept">{concept}</span>
    <button className="site-menu" ref={menu} aria-controls="site-main-nav" aria-expanded={open} onClick={() => setOpen(!open)}><span>{open ? 'Close' : 'Menu'}</span><span className="site-menu__icon" aria-hidden="true"/></button>
    <nav id="site-main-nav" className={open ? 'is-open' : ''} aria-label="Main navigation" onClick={() => setOpen(false)}>
      <a href="#how">How it works</a><a href="#apps">Apps</a><a href="#speech">Speech</a><a href="#install">Install</a><a href="#questions">Questions</a>
    </nav>
    <a className="site-download" href={download}>Download for Windows</a>
  </header>;
}

export function PopupPreview({ expanded = false }: { expanded?: boolean }) {
  const [tab, setTab] = useState('Definition');
  const [saved, setSaved] = useState(false);
  const [speech, setSpeech] = useState('');
  const [isExpanded, setIsExpanded] = useState(expanded);
  useEffect(() => setIsExpanded(expanded), [expanded]);
  return <div className={`site-popup ${isExpanded ? 'is-expanded' : ''}`} aria-label="Interactive DIO popup preview">
    <div className="site-popup__bar"><span aria-hidden="true">←</span><strong>DIO</strong><span aria-hidden="true">×</span></div>
    <div className="site-popup__body"><div className="site-popup__word"><h3>collisions</h3><button aria-label={saved ? 'Remove saved word' : 'Save word'} aria-pressed={saved} onClick={() => setSaved(!saved)}>{saved ? '★' : '☆'}</button></div>
      <div className="site-popup__ipa"><em>noun</em><span>US</span><span>/kəˈlɪʒənz/</span><button aria-label="Play pronunciation" onClick={() => { setSpeech('Playing with browser speech in this prototype.'); if ('speechSynthesis' in window) { speechSynthesis.cancel(); speechSynthesis.speak(new SpeechSynthesisUtterance('collisions')); } }}>◖))</button></div>
      <span className="sr-only" role="status">{speech}</span>
      {!isExpanded && <div className="site-popup__tabs" role="tablist" aria-label="Entry sections" onKeyDown={event => { if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return; event.preventDefault(); const names = ['Definition', 'Examples', 'Related']; const next = names[(names.indexOf(tab) + (event.key === 'ArrowRight' ? 1 : 2)) % 3]; setTab(next); (event.currentTarget.querySelectorAll('button')[names.indexOf(next)] as HTMLButtonElement).focus(); }}>{['Definition', 'Examples', 'Related'].map(name => <button key={name} role="tab" aria-selected={tab === name} tabIndex={tab === name ? 0 : -1} onClick={() => setTab(name)}>{name}</button>)}</div>}
      <div className="site-popup__content">{isExpanded ? <><b>Noun</b><p>1. An instance of colliding.</p><b>Noun</b><p>1. An error caused by two source code elements having the same name.</p></> : tab === 'Definition' ? <><b>Noun</b><p>An instance of colliding.</p><small>Example</small><p>The collisions were difficult to predict.</p></> : tab === 'Examples' ? <p>The collisions were difficult to predict.</p> : <p>Related words and sources appear when available.</p>}</div>
    </div><div className="site-popup__footer"><span>Kokoro Heart · US English</span><button onClick={() => setIsExpanded(!isExpanded)}>{isExpanded ? 'Collapse' : 'Expand'} ↗</button></div>
  </div>;
}

export function Walkthrough({ compact = false }: { compact?: boolean }) {
  const [step, setStep] = useState(0);
  const steps = [
    { title: 'Select a word', text: 'Highlight a word in the app you are reading.' },
    { title: 'Open DIO beside it', text: 'Press Ctrl + Alt + D for a definition and pronunciation.' },
    { title: 'Read further when needed', text: 'Open examples, related words, and more meanings.' },
  ];
  return <div className={`walkthrough ${compact ? 'walkthrough--compact' : ''}`}>
    <div className="walkthrough__visual">{step === 0 ? <p>The <mark>collisions</mark> were difficult to predict.</p> : <PopupPreview expanded={step === 2}/>}</div>
    <div className="walkthrough__controls"><span>{`0${step + 1} / 03`}</span><div><button aria-label="Previous step" disabled={step === 0} onClick={() => setStep(step - 1)}>←</button><button aria-label="Next step" disabled={step === 2} onClick={() => setStep(step + 1)}>→</button></div></div>
    <div className="walkthrough__copy"><h3>{steps[step].title}</h3><p>{steps[step].text}</p></div>
  </div>;
}

const apps = [
  { name: 'Chrome', image: 'chrome', detail: 'Read a definition beside an article.' },
  { name: 'Acrobat', image: 'acrobat', detail: 'Look up words in a PDF.' },
  { name: 'Terminal', image: 'terminal', detail: 'Use the shortcut in Windows Terminal.' },
  { name: 'Steam', image: 'steam', detail: 'Check a word without leaving a store page.' },
];
export function AppGallery() {
  const [app, setApp] = useState(0);
  return <div className="app-gallery"><div className="app-gallery__tabs" role="tablist" aria-label="Supported app examples" onKeyDown={event => { if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return; event.preventDefault(); const next = (app + (event.key === 'ArrowRight' ? 1 : apps.length - 1)) % apps.length; setApp(next); (event.currentTarget.querySelectorAll('button')[next] as HTMLButtonElement).focus(); }}>{apps.map((item, index) => <button key={item.name} role="tab" aria-selected={app === index} tabIndex={app === index ? 0 : -1} onClick={() => setApp(index)}>{item.name}</button>)}</div>
    <p>{apps[app].detail}</p><div className="app-gallery__image"><img src={shot(apps[app].image)} alt={`DIO popup in ${apps[app].name}`} /></div><a href={shot(apps[app].image)} target="_blank" rel="noopener">View full screenshot ↗</a></div>;
}

export function Speech() { return <section className="site-section speech-section" id="speech"><div className="site-wrap"><div className="section-intro"><h2>Speech on your computer</h2><p>Install the Kokoro voice once, about 95 MB. Pronunciation and read-aloud then run locally.</p></div><div className="speech-grid"><div><h3>Hear a word</h3><p>Play an available recording or use the installed Kokoro voice.</p></div><div><h3>Read selected text</h3><p>Select up to 2,000 characters and press Ctrl + Alt + R. Press it again to stop.</p></div><div><h3>Keep text local</h3><p>Text read aloud stays on your computer.</p></div></div></div></section>; }
export function Install() { return <section className="site-section install-section" id="install"><div className="site-wrap"><div className="section-intro"><h2>Install DIO in 3 steps</h2><p>Download the Windows installer, open it, then look up your first word.</p></div><ol className="install-steps"><li><span>01</span><h3>Download the installer</h3><p>Get DIO-setup.exe for Windows.</p></li><li><span>02</span><h3>Run the installer</h3><p>Open the downloaded file and follow the prompts.</p></li><li><span>03</span><h3>Look up a word</h3><p>Select a word and press Ctrl + Alt + D.</p></li></ol><div className="install-actions"><a className="site-button" href={download}>Download for Windows</a><a href={releases}>See all releases ↗</a></div><p className="install-note">DIO is not code-signed yet. If Windows SmartScreen warns you, choose More info, then Run anyway.</p></div></section>; }
export function Questions() { return <section className="site-section questions-section" id="questions"><div className="site-wrap"><div className="section-intro"><h2>Questions</h2></div><div className="questions-list"><div><h3>Does DIO work offline?</h3><p>Yes. It includes more than 152,000 English words and an acronym list.</p></div><div><h3>What does DIO send online?</h3><p>Only the word you look up goes to the two online dictionary sources. Text read aloud stays local.</p></div><div><h3>Where might lookup be blocked?</h3><p>Administrator apps, games, remote desktop windows, and the classic console can block text capture.</p></div></div></div></section>; }
export function SiteFooter() { return <footer className="site-footer"><div className="site-wrap"><div><h2>Download DIO for Windows</h2><p>Look up words beside what you are reading.</p><a className="site-button" href={download}>Download for Windows</a></div><div className="site-footer__links"><strong>DIO</strong><a href="https://github.com/nathandalpatan/dio">Source code</a><a href={releases}>Releases</a><a href="https://github.com/nathandalpatan/dio#readme">Read the README</a></div></div></footer>; }
