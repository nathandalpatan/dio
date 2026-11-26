import type { ReactNode } from 'react';

export type Entry = {
  word: string;
  part: string;
  ipa: string;
  primary: string;
  examples: string[];
  meanings: string[];
  related: string[];
  synonyms?: string[];
  origin?: string;
  source: string;
};

export const entries: Record<string, Entry> = {
  truth: {
    word: 'truth', part: 'noun', ipa: '/tɹuːθ/',
    primary: 'The truth is an idea which is correct or accepted as correct.',
    examples: ["Don’t lie to me. Tell me the truth."],
    meanings: ['True facts, genuine depiction or statements of reality.', 'Conformity to fact or reality; correctness, accuracy.'],
    related: ['fact', 'reality'], synonyms: ['fact'], source: 'Simple English Wiktionary · Free Dictionary API',
  },
  serendipity: {
    word: 'serendipity', part: 'noun', ipa: '/ˌsɛɹənˈdɪpɪti/',
    primary: 'The ability to make fortunate discoveries by chance.',
    examples: ['Finding that little bookshop was pure serendipity.'],
    meanings: ['An unexpected and fortunate discovery.'], related: ['luck', 'chance'],
    origin: 'Coined by Horace Walpole in 1754.', source: 'Free Dictionary API',
  },
  collision: {
    word: 'collision', part: 'noun', ipa: '/kəˈlɪʒən/',
    primary: 'An instance of colliding.',
    examples: ['The collision closed the road for two hours.'],
    meanings: ['A conflict between opposing ideas or interests.'], related: ['impact', 'contact'],
    source: 'Free Dictionary API',
  },
  interdisciplinary: {
    word: 'interdisciplinary', part: 'adjective', ipa: '/ˌɪntəˌdɪsɪˈplɪnəɹi/',
    primary: 'Involving two or more areas of knowledge or study.',
    examples: ['The team took an interdisciplinary approach to the problem.'],
    meanings: ['Drawing on methods from more than one academic discipline.'],
    related: ['collaborative', 'integrated'], source: 'Free Dictionary API',
  },
  fact: { word: 'fact', part: 'noun', ipa: '/fækt/', primary: 'Something that is known to be true.', examples: ['Check the facts before publishing.'], meanings: [], related: ['truth'], source: 'Free Dictionary API' },
  reality: { word: 'reality', part: 'noun', ipa: '/ɹiˈælɪti/', primary: 'The state of things as they actually exist.', examples: ['The reality was more complicated.'], meanings: [], related: ['truth'], source: 'Free Dictionary API' },
  luck: { word: 'luck', part: 'noun', ipa: '/lʌk/', primary: 'Success or failure brought about by chance.', examples: ['A little luck helped us find it.'], meanings: [], related: ['serendipity'], source: 'Free Dictionary API' },
  chance: { word: 'chance', part: 'noun', ipa: '/tʃæns/', primary: 'The possibility of something happening.', examples: ['They met by chance.'], meanings: [], related: ['serendipity'], source: 'Free Dictionary API' },
  impact: { word: 'impact', part: 'noun', ipa: '/ˈɪmpækt/', primary: 'The act of one object hitting another.', examples: ['The impact left a mark.'], meanings: [], related: ['collision'], source: 'Free Dictionary API' },
  contact: { word: 'contact', part: 'noun', ipa: '/ˈkɒntækt/', primary: 'The state of touching or meeting.', examples: ['The surfaces came into contact.'], meanings: [], related: ['collision'], source: 'Free Dictionary API' },
  collaborative: { word: 'collaborative', part: 'adjective', ipa: '/kəˈlæbəɹətɪv/', primary: 'Done by people working together.', examples: ['It was a collaborative effort.'], meanings: [], related: ['interdisciplinary'], source: 'Free Dictionary API' },
  integrated: { word: 'integrated', part: 'adjective', ipa: '/ˈɪntɪɡɹeɪtɪd/', primary: 'Combined into a unified whole.', examples: ['The project used an integrated approach.'], meanings: [], related: ['interdisciplinary'], source: 'Free Dictionary API' },
};

export type PreviewStatus = 'ready' | 'loading' | 'not-found' | 'error' | 'reading';
export type PreviewModel = {
  entry?: Entry;
  status: PreviewStatus;
  expanded: boolean;
  saved: boolean;
  canBack: boolean;
  canForward: boolean;
  settingsOpen: boolean;
  speaking: boolean;
  speechMessage: string;
  accent: 'en-US' | 'en-GB';
  voice: 'default' | 'female' | 'male';
};
export type PreviewActions = {
  back: () => void;
  forward: () => void;
  close: () => void;
  save: () => void;
  pronounce: () => void;
  read: () => void;
  stop: () => void;
  expand: () => void;
  lookup: (word: string) => void;
  settings: () => void;
  setAccent: (accent: 'en-US' | 'en-GB') => void;
  setVoice: (voice: 'default' | 'female' | 'male') => void;
};
export type VariantProps = { model: PreviewModel; actions: PreviewActions };

const paths: Record<string, string> = {
  back: 'M15 18l-6-6 6-6M9 12h12', forward: 'M9 18l6-6-6-6M15 12H3', close: 'M18 6 6 18M6 6l12 12',
  star: 'm12 2 3.1 6.3 6.9 1-5 4.9 1.2 6.9L12 17.5l-6.2 3.3L7 13.9 2 9l6.9-1L12 2Z',
  speaker: 'M4 9h3l4-3v12l-4-3H4zM15 9a4 4 0 0 1 0 6M17.5 6.5a7.5 7.5 0 0 1 0 11',
  play: 'm8 5 11 7-11 7z', stop: 'M6 6h12v12H6z',
  expand: 'M8 3H3v5m0-5 6 6m7 12h5v-5m0 5-6-6',
  collapse: 'm3 9 6-6m0 0v5M3 3h5m13 12-6 6m0 0v-5m0 5h5',
  settings: 'M4 7h16M4 17h16M8 4v6m8 4v6',
  chevron: 'm6 9 6 6 6-6',
};

export function Icon({ name, size = 18, filled = false }: { name: string; size?: number; filled?: boolean }) {
  return <svg aria-hidden="true" viewBox="0 0 24 24" width={size} height={size} fill={filled ? 'currentColor' : 'none'} stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d={paths[name]}/></svg>;
}

export function Status({ model }: { model: PreviewModel }) {
  const content: Record<Exclude<PreviewStatus, 'ready'>, ReactNode> = {
    loading: <>Looking up definition<span className="status-dots" aria-hidden="true">…</span></>,
    'not-found': 'No definition found. Try another selection.',
    error: 'Could not reach a dictionary. Try again or use an offline word.',
    reading: <>Reading selected text aloud<br /><small>Press Stop to end playback.</small></>,
  };
  return <div className="variant-status" role="status"><span className={model.status === 'loading' ? 'variant-spinner' : 'status-symbol'} aria-hidden="true">{model.status === 'loading' ? '' : '⌕'}</span><p>{content[model.status === 'ready' ? 'not-found' : model.status]}</p></div>;
}

export function SettingsPanel({ model, actions }: VariantProps) {
  if (!model.settingsOpen) return null;
  return <div className="preview-settings" role="dialog" aria-modal="true" aria-label="Speech settings">
    <div className="preview-settings__head"><strong>Speech settings</strong><button className="icon-target" onClick={actions.settings} aria-label="Close speech settings"><Icon name="close" /></button></div>
    <label>Accent<select value={model.accent} onChange={event => actions.setAccent(event.target.value as PreviewModel['accent'])}><option value="en-US">US English</option><option value="en-GB">UK English</option></select></label>
    <label>Browser preview voice<select value={model.voice} onChange={event => actions.setVoice(event.target.value as PreviewModel['voice'])}><option value="default">System default</option><option value="female">First available voice</option><option value="male">Second available voice</option></select></label>
    <p>This prototype uses your browser’s speech engine. DIO uses Kokoro in the desktop app.</p>
  </div>;
}

export function RelatedLinks({ entry, lookup }: { entry: Entry; lookup: (word: string) => void }) {
  return <div className="related-links">{entry.related.map(word => <button key={word} className="text-link" onClick={() => lookup(word)}>{word}</button>)}</div>;
}
