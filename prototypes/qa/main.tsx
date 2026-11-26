import { createRoot } from 'react-dom/client';
import { AppView } from '../../src/main';
import './style.css';

const params = new URLSearchParams(location.search);
const view = params.get('view') === 'settings' ? 'settings' : 'popup';
const data = params.get('data') || 'demo';
const size = params.get('size') || 'standard';
const sizes: Record<string, [number, number]> = {
  small: [328, 293], standard: [378, 332], large: [429, 372], expanded: [514, 378], narrow: [300, 260], wide: [700, 560], minimum: [300, 220], huge: [900, 640],
  settings: [880, 600], 'settings-min': [720, 480],
};
const [width, height] = sizes[view === 'settings' ? (size === 'settings-min' ? size : 'settings') : size] || sizes.standard;
const text = params.get('text') || 'compact';
const href = (next: Record<string, string>) => `?${new URLSearchParams({ view, data, size, text, ...next })}`;
const modes = ['demo', 'worst', 'empty', 'large', 'slow', 'error', 'reading'];
const popupSizes = ['minimum', 'narrow', 'small', 'standard', 'large', 'expanded', 'wide', 'huge'];

createRoot(document.getElementById('qa')!).render(<>
  <header className="qa-toolbar">
    <strong>DIO QA</strong>
    <nav aria-label="View">{['popup', 'settings'].map(item => <a key={item} aria-current={view === item ? 'page' : undefined} href={href({ view: item, size: item === 'settings' ? 'settings' : 'standard' })}>{item}</a>)}</nav>
    <nav aria-label="Data">{modes.map(item => <a key={item} aria-current={data === item ? 'page' : undefined} href={href({ data: item })}>{item}</a>)}</nav>
    <nav aria-label="Text size">{['compact', 'standard', 'large'].map(item => <a key={item} aria-current={text === item ? 'page' : undefined} href={href({ text: item })}>{item}</a>)}</nav>
    <nav aria-label="Size">{(view === 'settings' ? ['settings-min', 'settings'] : popupSizes).map(item => <a key={item} aria-current={size === item ? 'page' : undefined} href={href({ size: item })}>{item}</a>)}</nav>
  </header>
  <p className="qa-note">Development fixture. This renders the production UI with mocked Tauri responses; it does not exercise native window placement or external services.</p>
  <div className="qa-frame" style={{ width, height }}><AppView /></div>
</>);
