type Listener<T> = (event: { payload: T }) => void;
const listeners = new Map<string, Set<Listener<unknown>>>();
const params = new URLSearchParams(location.search);
const mode = params.get('data') || 'demo';
const expanded = ['expanded', 'wide', 'huge'].includes(params.get('size') || '');
const longWord = 'pneumonoultramicroscopicsilicovolcanoconiosis';
const definition = (text: string, example = '') => ({ definition: text, example, synonyms: [] as string[], antonyms: [] as string[] });
const regular = {
  word: 'truth', phonetic: '/truːθ/', phonetics: [], origin: '', sourceUrls: [] as string[],
  meanings: [{ partOfSpeech: 'noun', definitions: [definition('The truth is an idea which is correct or accepted as correct.', 'Don’t lie to me. Tell me the truth.')], synonyms: ['fact'], antonyms: [] as string[] }],
};
const worst = {
  word: longWord, phonetic: '/ˌnjuː.mə.noʊ.ʌl.trə.maɪ.krə.skɒ.pɪk.sɪ.lɪ.koʊ.vɒl.keɪ.noʊ.koʊ.niˈoʊ.sɪs/', phonetics: [],
  origin: 'A long compound word with an origin note that should remain readable when the lookup window is narrow and the text is enlarged.',
  sourceUrls: ['https://example.com/dictionary/very-long-entry-name-with-several-segments'],
  meanings: [{ partOfSpeech: 'noun', definitions: [
    definition('A deliberately long medical term used here to test how a real dictionary entry wraps in a compact lookup window.', 'The article mentioned pneumonoultramicroscopicsilicovolcanoconiosis while discussing unusually long English words.'),
    ...Array.from({ length: 7 }, (_, index) => definition(`Additional sense ${index + 2} with a full sentence of explanatory context that must remain readable at the smallest supported popup size.`)),
  ], synonyms: ['medical terminology', 'Benachrichtigungseinstellungen', 'نور الهدى عبد الرحمن', '王秀英', '👩🏽‍💻'], antonyms: [] as string[] }],
};
const large = {
  ...regular,
  meanings: [{ ...regular.meanings[0], definitions: Array.from({ length: 60 }, (_, index) => definition(`Meaning ${index + 1}: an explanation with enough detail to fill more than one line in a compact window.`)), synonyms: Array.from({ length: 80 }, (_, index) => `related expression ${index + 1}`) }],
};
const entry = mode === 'worst' ? worst : mode === 'large' ? large : regular;
let state = {
  requestId: 1, status: mode === 'empty' ? 'notFound' : mode === 'error' ? 'network' : mode === 'reading' ? 'reading' : mode === 'slow' ? 'loading' : 'ready',
  query: entry.word, entries: ['empty', 'error', 'reading', 'slow'].includes(mode) ? [] : [entry],
  message: mode === 'error' ? 'The dictionary request failed. Try the lookup again.' : '',
  reading: mode === 'reading' ? 'A long selection with punctuation, line breaks, and varied scripts. '.repeat(34) : '',
  stale: false, expanded, direction: mode === 'worst' ? 'nw' : 'se', canBack: false, canForward: false, saved: false,
};
let settings = {
  shortcut: 'Ctrl+Alt+D', keyboardEnabled: true, middleClick: false, theme: params.get('theme') || 'light', size: 'standard', textSize: params.get('text') || 'compact', expanded: false, autoplay: false,
  startup: false, reducedMotion: false, accent: 'us', ttsEnabled: true, voice: 'af_heart', speed: 1, volume: 1,
  readerEnabled: true, readerShortcut: 'Ctrl+Alt+R', readerMaxChars: 2000,
};
let speech = { status: 'ready', progress: 0, message: '', owner: '' };
const updateFixture = params.get('update') || 'upToDate';
const notes = ['Automatic updates, with a choice of how DIO installs them.', 'Faster offline dictionary lookups.', 'Clearer pronunciation playback.', 'Fixed popup alignment near the screen edge.'];
const failures: Record<string, [string, string]> = {
  failedDownload: ['download', 'The connection was interrupted.'],
  failedSignature: ['download', 'The update could not be verified. Its signature does not match DIO’s signing key, so it was discarded.'],
  failedInstall: ['install', 'The installer did not finish.'],
  failedCheck: ['check', 'DIO could not reach GitHub. Check your internet connection.'],
};
let update = {
  status: updateFixture.startsWith('failed') ? 'failed' : updateFixture, current: updateFixture === 'updated' ? '0.2.1' : '0.2.0', version: '0.2.1',
  notes: updateFixture === 'updated' ? ['Faster offline lookups and clearer pronunciation playback.'] : notes, progress: updateFixture === 'downloading' ? 0.68 : 0,
  retry: failures[updateFixture]?.[0] || '', error: failures[updateFixture]?.[1] || '',
  lastChecked: updateFixture === 'idle' ? null : Math.floor(Date.now() / 1000) - 120,
};
const setUpdate = (patch: Partial<typeof update>) => { update = { ...update, ...patch }; emit('update-state', update); };
const parts = ['analysis', 'context', 'meaning', 'memory', 'language', 'pattern', 'reading', 'reference', 'speech', 'study', 'syntax', 'usage'];
const words = mode === 'large' ? Array.from({ length: 1284 }, (_, index) => `${parts[index % parts.length]} ${parts[Math.floor(index / parts.length) % parts.length]} ${Math.floor(index / 144) + 1}`) : mode === 'empty' ? [] : ['truth'];
const emit = <T,>(name: string, payload: T) => listeners.get(name)?.forEach(listener => listener({ payload }));

export const isTauri = () => true;
export const listen = async <T,>(name: string, listener: Listener<T>) => {
  if (!listeners.has(name)) listeners.set(name, new Set());
  const group = listeners.get(name)!;
  group.add(listener as Listener<unknown>);
  return () => { group.delete(listener as Listener<unknown>); };
};
let settingsAttempts = 0;
export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  switch (command) {
    case 'popup_state':
      if (mode === 'slow') setTimeout(() => { state = { ...state, status: 'ready', entries: [entry] }; emit('popup-state', state); }, 2500);
      return state as T;
    case 'get_settings':
      if (mode === 'error' && params.get('view') === 'settings' && ++settingsAttempts === 1) throw 'Settings could not be loaded.';
      if (mode === 'slow') await new Promise(resolve => setTimeout(resolve, 2500));
      return settings as T;
    case 'saved_words': return words.map((word, index) => ({ word, added: Math.floor(Date.now() / 1000) - index * 900 })) as T;
    case 'tts_state': return speech as T;
    case 'update_state': return update as T;
    case 'check_for_updates': setUpdate({ status: 'checking' }); setTimeout(() => setUpdate({ status: 'available', lastChecked: Math.floor(Date.now() / 1000) }), 1200); break;
    case 'download_update':
    case 'retry_update': {
      setUpdate({ status: 'downloading', progress: 0, error: '', retry: '' });
      const tick = setInterval(() => {
        if (update.status !== 'downloading') { clearInterval(tick); return; }
        if (update.progress >= 1) { clearInterval(tick); setUpdate({ status: 'verifying' }); setTimeout(() => setUpdate({ status: 'ready' }), 900); return; }
        setUpdate({ progress: Math.min(1, update.progress + 0.1) });
      }, 250);
      break;
    }
    case 'cancel_update': setUpdate({ status: 'available', progress: 0 }); break;
    case 'postpone_update': setUpdate({ status: 'restartRequired' }); break;
    case 'install_update': setUpdate({ status: 'installing' }); break;
    case 'remind_later':
    case 'dismiss_update': setUpdate({ status: 'idle', error: '', retry: '' }); break;
    case 'tts_voices': return [{ id: 'af_heart', name: 'Heart', accent: 'us' }, { id: 'bf_emma', name: 'Emma', accent: 'uk' }] as T;
    case 'expand': state = { ...state, expanded: Boolean(args?.expanded) }; emit('popup-state', state); break;
    case 'save_word': state = { ...state, saved: Boolean(args?.save) }; emit('popup-state', state); emit('saved-changed', null); break;
    case 'lookup':
    case 'open_saved': state = { ...state, requestId: state.requestId + 1, status: 'ready', query: String(args?.word), entries: [{ ...regular, word: String(args?.word) }], canBack: true }; emit('popup-state', state); break;
    case 'update_settings': settings = args?.next as typeof settings; emit('settings-changed', settings); break;
    case 'tts_speak': speech = { ...speech, status: 'playing', owner: String(args?.owner) }; emit('tts-state', speech); break;
    case 'tts_pause': speech = { ...speech, status: 'paused' }; emit('tts-state', speech); break;
    case 'tts_resume': speech = { ...speech, status: 'playing' }; emit('tts-state', speech); break;
    case 'tts_stop': speech = { ...speech, status: 'ready', owner: '' }; emit('tts-state', speech); break;
    case 'show_settings': location.href = `?view=settings&data=${mode}&size=settings`; break;
  }
  return undefined as T;
}
