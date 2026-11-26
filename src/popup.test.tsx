import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AppView, PopupView, ago } from './main';

const api = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
  listeners: {} as Record<string, (event: { payload: unknown }) => void>,
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke, isTauri: api.isTauri }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async (name: string, listener: (event: { payload: unknown }) => void) => {
  api.listeners[name] = listener;
  return () => { delete api.listeners[name]; };
}) }));

const settings = { shortcut: 'Ctrl+Alt+D', keyboardEnabled: true, middleClick: false, theme: 'system', size: 'standard', expanded: false, autoplay: false, startup: false, reducedMotion: true };
const initial = { requestId: 0, status: 'idle', query: '', entries: [], message: '', stale: false, expanded: false, direction: 'se', canBack: false, canForward: false, saved: false };
const entry = { word: 'serendipity', phonetic: '', phonetics: [], meanings: [{ partOfSpeech: 'noun', definitions: [{ definition: 'A happy discovery.', example: '', synonyms: [], antonyms: [] }], synonyms: ['luck'], antonyms: [] }], origin: '', sourceUrls: [] };
const send = (payload: object) => act(() => api.listeners['popup-state']({ payload: { ...initial, ...payload } }));

beforeEach(() => {
  Object.defineProperty(window, 'matchMedia', { configurable: true, value: () => ({ matches: false }) });
  api.listeners = {};
  api.isTauri.mockReset().mockReturnValue(true);
  api.invoke.mockReset().mockImplementation((command: string) => Promise.resolve(command === 'popup_state' ? initial : command === 'get_settings' ? settings : undefined));
});
afterEach(cleanup);

describe('popup', () => {
  it('shows browser instructions without invoking the desktop backend', () => {
    api.isTauri.mockReturnValue(false);
    render(<AppView />);
    expect(screen.getByText(/DIO runs in its desktop window/)).toBeTruthy();
    expect(api.invoke).not.toHaveBeenCalled();
    expect(Object.keys(api.listeners)).toHaveLength(0);
  });

  it('mounts the popup in the desktop runtime', async () => {
    const { container } = render(<AppView />);
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('popup_state', undefined));
    expect(container.querySelector('.popup-header')?.hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(screen.getByText('DIO').hasAttribute('data-tauri-drag-region')).toBe(true);
    expect(screen.getByRole('button', { name: 'Close' }).hasAttribute('data-tauri-drag-region')).toBe(false);
  });

  it('shows loading, timeout, and missing definition states', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'loading' });
    expect(screen.getByText('Looking up definition…')).toBeTruthy();
    send({ requestId: 1, status: 'timeout', message: 'Selection retrieval timed out.' });
    expect(screen.getByText('Selection retrieval timed out.')).toBeTruthy();
    send({ requestId: 2, status: 'notFound' });
    expect(screen.getByText('No definition found.')).toBeTruthy();
    send({ requestId: 1, status: 'ready', entries: [entry] });
    expect(screen.queryByText('serendipity')).toBeNull();
  });

  it('caps long sense lists and hides numeronym synonyms', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    const definitions = Array.from({ length: 7 }, (_, i) => ({ definition: `Sense ${i + 1}.`, example: '', synonyms: [], antonyms: [] }));
    const long = { ...entry, meanings: [{ partOfSpeech: 'noun', definitions, synonyms: ['a11y', 'usability'], antonyms: [] }] };
    send({ requestId: 1, status: 'ready', query: 'serendipity', entries: [long], expanded: true });
    expect(screen.getByText('Sense 5.')).toBeTruthy();
    expect(screen.queryByText('Sense 6.')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Show all 7 meanings' }));
    expect(screen.getByText('Sense 7.')).toBeTruthy();
    fireEvent.click(screen.getByRole('tab', { name: 'Synonyms' }));
    expect(screen.queryByText(/a11y/)).toBeNull();
    expect(screen.getByText('usability')).toBeTruthy();
  });

  it('hides absent fields and follows related words', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', query: 'serendipity', entries: [entry], expanded: true });
    expect(screen.getByText('serendipity')).toBeTruthy();
    expect(screen.queryByText('Origin')).toBeNull();
    fireEvent.click(screen.getByRole('tab', { name: 'Synonyms' }));
    expect(screen.queryByText('Antonyms')).toBeNull();
    fireEvent.click(screen.getByText('luck'));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('lookup', { word: 'luck' }));
  });

  it('shows additional acronym senses without repeating the primary definition', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    const lol = { ...entry, word: 'LOL', meanings: [
      { ...entry.meanings[0], partOfSpeech: 'interjection', definitions: [
        { ...entry.meanings[0].definitions[0], definition: 'Laugh out loud: expresses amusement.' },
        { ...entry.meanings[0].definitions[0], definition: 'Lots of love: an affectionate sign-off.' },
      ], synonyms: [] },
      { ...entry.meanings[0], partOfSpeech: 'initialism', definitions: [{ ...entry.meanings[0].definitions[0], definition: 'League of Legends: a video game.' }], synonyms: [] },
    ] };
    send({ requestId: 1, status: 'ready', query: 'LoL', entries: [lol], expanded: true });
    expect(screen.getAllByText('Laugh out loud: expresses amusement.')).toHaveLength(1);
    expect(screen.getByText('Lots of love: an affectionate sign-off.')).toBeTruthy();
    expect(screen.getByText('League of Legends: a video game.')).toBeTruthy();
  });

  it('shows entry sections as tabs in the compact view', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', query: 'serendipity', entries: [entry] });
    expect(screen.getAllByRole('tab').map(tab => tab.textContent)).toEqual(['Definition', 'Synonyms']);
    expect(screen.getByText('A happy discovery.')).toBeTruthy();
    expect(screen.queryByText('Example')).toBeNull();
    fireEvent.click(screen.getByRole('tab', { name: 'Synonyms' }));
    expect(screen.getByText('Alternatives based on this meaning')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'luck' }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('lookup', { word: 'luck' }));
    const withExample = { ...entry, origin: 'Coined by Horace Walpole.', meanings: [{ ...entry.meanings[0], definitions: [{ ...entry.meanings[0].definitions[0], example: 'A fortunate find.' }] }] };
    send({ requestId: 2, status: 'ready', query: 'serendipity', entries: [withExample] });
    expect(screen.getByRole('tab', { name: 'Definition' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByText('Example')).toBeTruthy();
    expect(screen.getByText('A fortunate find.')).toBeTruthy();
    expect(screen.queryByRole('tab', { name: 'Related' })).toBeNull();
    fireEvent.keyDown(screen.getByRole('tablist', { name: 'Entry sections' }), { key: 'ArrowRight' });
    expect(screen.getByRole('tab', { name: 'Examples' }).getAttribute('aria-selected')).toBe('true');
    fireEvent.keyDown(screen.getByRole('tablist', { name: 'Entry sections' }), { key: 'ArrowLeft' });
    expect(screen.getByRole('tab', { name: 'Definition' }).getAttribute('aria-selected')).toBe('true');
    fireEvent.click(screen.getByRole('tab', { name: 'Examples' }));
    expect(screen.getByText('A fortunate find.')).toBeTruthy();
    fireEvent.click(screen.getByText('Expand ↘'));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('expand', { expanded: true, reducedMotion: false }));
    send({ requestId: 2, status: 'ready', query: 'serendipity', entries: [withExample], expanded: true });
    expect(screen.getByRole('tab', { name: 'Examples' }).getAttribute('aria-selected')).toBe('true');
    fireEvent.click(screen.getByRole('tab', { name: 'Definition' }));
    expect(screen.getByText('Origin')).toBeTruthy();
  });

  it('uses an example from the primary definition only', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    const meanings = [{ ...entry.meanings[0], definitions: [entry.meanings[0].definitions[0], { ...entry.meanings[0].definitions[0], definition: 'Another meaning.', example: 'A later example.' }] }];
    send({ requestId: 1, status: 'ready', entries: [{ ...entry, meanings }] });
    expect(screen.queryByText('Example')).toBeNull();
    expect(screen.getByRole('tab', { name: 'Examples' })).toBeTruthy();
  });

  it('offers forward navigation only when native history has a later word', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', entries: [entry] });
    expect(screen.queryByRole('button', { name: 'Forward to next word' })).toBeNull();
    send({ requestId: 2, status: 'ready', entries: [entry], canBack: true, canForward: true });
    fireEvent.click(screen.getByRole('button', { name: 'Forward to next word' }));
    expect(api.invoke).toHaveBeenCalledWith('forward', undefined);
    send({ requestId: 3, status: 'ready', entries: [entry], canBack: true });
    expect(screen.queryByRole('button', { name: 'Forward to next word' })).toBeNull();
  });

  it('uses the actual expansion direction and handles keyboard dismissal', async () => {
    const { container } = render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', entries: [entry], direction: 'nw', canBack: true });
    expect(container.querySelector('.reduced-motion')).toBeTruthy();
    fireEvent.click(screen.getByText('Expand ↖'));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('expand', { expanded: true, reducedMotion: false }));
    send({ requestId: 1, status: 'ready', entries: [entry], direction: 'nw', expanded: true, canBack: true });
    expect(screen.getByText('Collapse ↘')).toBeTruthy();
    fireEvent.keyDown(window, { key: 'Backspace' });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(api.invoke).toHaveBeenCalledWith('back', undefined);
    expect(api.invoke).toHaveBeenCalledWith('close_popup', undefined);
  });

  it('shows sourced IPA in the preferred accent with every variant under Expand', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['settings-changed']).toBeDefined());
    const pronunciations = [
      { ipa: '/ˌsɛɹ.ənˈdɪp.ɪ.ti/', dialect: 'uk', source: 'wiktionary' },
      { ipa: '/ˌsɛɹ.ənˈdɪp.ə.t̬i/', dialect: 'us', source: 'dictionary' },
    ];
    send({ requestId: 1, status: 'ready', entries: [entry], pronunciations });
    expect(screen.getByText('/ˌsɛɹ.ənˈdɪp.ə.t̬i/')).toBeTruthy();
    expect(screen.getByText('US')).toBeTruthy();
    act(() => api.listeners['settings-changed']({ payload: { ...settings, accent: 'uk' } }));
    expect(screen.getByText('/ˌsɛɹ.ənˈdɪp.ɪ.ti/').getAttribute('title')).toBe('Wiktionary');
    send({ requestId: 1, status: 'ready', entries: [entry], pronunciations, expanded: true });
    expect(screen.getByText('Pronunciation')).toBeTruthy();
    expect(screen.getByText('Free Dictionary API')).toBeTruthy();
  });

  it('marks generated IPA and hides the row when nothing is known', async () => {
    const { container } = render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', entries: [entry], pronunciations: [{ ipa: '/blɔɹp/', dialect: 'us', source: 'generated' }] });
    expect(screen.getByLabelText('Generated pronunciation')).toBeTruthy();
    expect(screen.getByText('/blɔɹp/').closest('[title]')?.getAttribute('title')).toMatch(/not a dictionary transcription/);
    send({ requestId: 2, status: 'ready', entries: [entry], pronunciations: [] });
    expect(container.querySelector('.ipa')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Play pronunciation' })).toBeNull();
  });

  it('pronounces with Kokoro, falling back to the recording when speech is unavailable', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['tts-state']).toBeDefined());
    act(() => api.listeners['tts-state']({ payload: { status: 'ready', progress: 0, message: '', owner: '' } }));
    const recorded = { ...entry, phonetics: [{ text: '', audio: 'https://api.dictionaryapi.dev/media/serendipity-us.mp3', sourceUrl: '' }] };
    send({ requestId: 1, status: 'ready', entries: [recorded] });
    fireEvent.click(screen.getByRole('button', { name: 'Play pronunciation' }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('tts_speak', { text: 'serendipity', owner: 'word' }));
    expect(api.invoke).not.toHaveBeenCalledWith('pronunciation', expect.anything());
    act(() => api.listeners['tts-state']({ payload: { status: 'notInstalled', progress: 0, message: '', owner: '' } }));
    fireEvent.click(screen.getByRole('button', { name: 'Play pronunciation' }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('pronunciation', { url: recorded.phonetics[0].audio }));
  });

  it('autoplays when speech becomes ready after the definition', async () => {
    api.invoke.mockImplementation((command: string) => Promise.resolve(({ popup_state: initial, get_settings: { ...settings, autoplay: true } } as Record<string, unknown>)[command]));
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['tts-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', query: 'serendipity', entries: [entry] });
    expect(api.invoke).not.toHaveBeenCalledWith('tts_speak', expect.anything());
    act(() => api.listeners['tts-state']({ payload: { status: 'ready', progress: 0, message: '', owner: '' } }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('tts_speak', { text: 'serendipity', owner: 'word' }));
    act(() => api.listeners['tts-state']({ payload: { status: 'playing', progress: 0, message: '', owner: 'word' } }));
    expect(api.invoke.mock.calls.filter(([command]) => command === 'tts_speak')).toHaveLength(1);
    send({ requestId: 2, status: 'ready', query: 'serendipity', entries: [entry] });
    await waitFor(() => expect(api.invoke.mock.calls.filter(([command]) => command === 'tts_speak')).toHaveLength(2));
  });

  it('shows simple definitions compactly and adds the full dictionary when expanded', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    const sense = (definition: string) => ({ definition, example: '', synonyms: [], antonyms: [] });
    const simple = { ...entry, word: 'run', meanings: [{ partOfSpeech: 'verb', definitions: [sense('If you run, you go quickly.')], synonyms: [], antonyms: [] }] };
    const full = { ...entry, word: 'run', meanings: [{ partOfSpeech: 'verb', definitions: [sense('To run.')], synonyms: [], antonyms: [] }] };
    send({ requestId: 1, status: 'ready', query: 'run', entries: [simple, full] });
    expect(screen.getByText('If you run, you go quickly.')).toBeTruthy();
    expect(screen.queryByText('To run.')).toBeNull();
    send({ requestId: 1, status: 'ready', query: 'run', entries: [simple, full], expanded: true });
    expect(screen.getByText('If you run, you go quickly.')).toBeTruthy();
    expect(screen.getByText('More meanings')).toBeTruthy();
    expect(screen.getByText('To run.')).toBeTruthy();
  });

  it('sizes text by the Text size setting, not by the popup', async () => {
    const { container } = render(<PopupView />);
    await waitFor(() => expect(container.querySelector('.popup')?.className).toContain('text-compact'));
    act(() => api.listeners['settings-changed']({ payload: { ...settings, textSize: 'large' } }));
    expect(container.querySelector('.popup')?.className).toContain('text-large');
    expect(container.querySelector('.popup')?.className).not.toContain('text-compact');
  });

  it('keeps the section tabs when expanded and shows each section in full', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    const opposed = { ...entry, meanings: [{ ...entry.meanings[0], antonyms: ['misfortune'] }] };
    send({ requestId: 1, status: 'ready', query: 'serendipity', entries: [opposed], expanded: true });
    expect(screen.getAllByRole('tab').map(tab => tab.textContent)).toEqual(['Definition', 'Synonyms']);
    expect(screen.getByText('A happy discovery.')).toBeTruthy();
    fireEvent.click(screen.getByRole('tab', { name: 'Synonyms' }));
    expect(screen.getByRole('button', { name: 'luck' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'misfortune' })).toBeTruthy();
  });

  it('looks up a word clicked in the definition', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', query: 'serendipity', entries: [entry] });
    const text = screen.getByText('A happy discovery.').firstChild!;
    const at = (offset: number) => Object.assign(document, { caretRangeFromPoint: () => { const range = document.createRange(); range.setStart(text, offset); return range; } });
    Range.prototype.getBoundingClientRect = () => ({ left: 0, right: 100, top: 0, bottom: 20 }) as DOMRect;
    at(4);
    fireEvent.click(screen.getByText('A happy discovery.'), { clientX: 10, clientY: 10 });
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('lookup', { word: 'happy' }));
    at(9);
    fireEvent.click(screen.getByText('A happy discovery.'), { clientX: 200, clientY: 10 });
    expect(api.invoke).not.toHaveBeenCalledWith('lookup', { word: 'discovery' });
  });

  it('shows speech status and playback controls in the footer', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['tts-state']).toBeDefined());
    const speech = (payload: object) => act(() => api.listeners['tts-state']({ payload: { progress: 0, message: '', owner: '', ...payload } }));
    send({ requestId: 1, status: 'ready', entries: [entry] });
    speech({ status: 'notInstalled' });
    expect(screen.getByText('Voice not installed')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Read definition aloud' })).toBeNull();
    speech({ status: 'downloading', progress: 0.42 });
    expect(screen.getByText('Downloading voice 42%')).toBeTruthy();
    speech({ status: 'ready' });
    expect(screen.getByText('Kokoro voice · US English')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Read definition aloud' }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('tts_speak', { text: 'serendipity. A happy discovery.', owner: 'popup' }));
    speech({ status: 'playing', owner: 'popup' });
    fireEvent.click(screen.getByRole('button', { name: 'Pause speech' }));
    expect(api.invoke).toHaveBeenCalledWith('tts_pause', undefined);
    speech({ status: 'paused', owner: 'popup' });
    fireEvent.click(screen.getByRole('button', { name: 'Resume speech' }));
    expect(api.invoke).toHaveBeenCalledWith('tts_resume', undefined);
    fireEvent.click(screen.getByRole('button', { name: 'Stop speech' }));
    expect(api.invoke).toHaveBeenCalledWith('tts_stop', undefined);
    speech({ status: 'error', message: 'No audio output device is available.' });
    expect(screen.getByRole('status').textContent).toBe('No audio output device is available.');
    fireEvent.click(screen.getByRole('button', { name: 'Speech settings' }));
    expect(api.invoke).toHaveBeenCalledWith('show_settings', undefined);
  });

  it('switches the Kokoro voice from the footer', async () => {
    api.invoke.mockImplementation((command: string) => Promise.resolve(({ popup_state: initial, get_settings: { ...settings, voice: 'af_heart' }, tts_state: { status: 'ready', progress: 0, message: '', owner: '' }, tts_voices: [{ id: 'af_heart', name: 'Heart', accent: 'us' }, { id: 'bf_emma', name: 'Emma', accent: 'uk' }] } as Record<string, unknown>)[command]));
    render(<PopupView />);
    const picker = await screen.findByRole('combobox', { name: 'Voice' }) as HTMLSelectElement;
    expect(picker.selectedOptions[0].textContent).toBe('Kokoro Heart · US English');
    fireEvent.change(picker, { target: { value: 'bf_emma' } });
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('update_settings', { next: { ...settings, voice: 'bf_emma' } }));
  });

  it('shows the selection being read aloud', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 3, status: 'reading', reading: 'The selected paragraph…' });
    expect(screen.getByText('Reading selection aloud')).toBeTruthy();
    expect(screen.getByText('The selected paragraph…')).toBeTruthy();
    expect(screen.queryByText(/Expand/)).toBeNull();
  });

  it('keeps dismissal available when a lookup fails', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'network', message: 'Dictionary service returned 502 Bad Gateway.' });
    expect(screen.queryByText('Expand')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Back to app' }));
    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(api.invoke).toHaveBeenCalledWith('back', undefined);
    expect(api.invoke).toHaveBeenCalledWith('close_popup', undefined);
  });
});

describe('ago', () => {
  const now = new Date(2026, 10, 27, 15, 0).getTime();
  const at = (date: Date) => Math.floor(date.getTime() / 1000);
  it('describes how long ago a word was saved', () => {
    expect(ago(at(new Date(now - 2 * 60000)), now)).toBe('2 min ago');
    expect(ago(at(new Date(now - 3 * 3600000)), now)).toBe('3 hr ago');
    expect(ago(at(new Date(2026, 10, 26, 22, 0)), now)).toBe('Yesterday');
    expect(ago(at(new Date(2026, 10, 20)), now)).not.toMatch(/ago|Yesterday/);
  });
});

describe('settings', () => {
  const speechSettings = { ...settings, accent: 'us', ttsEnabled: true, voice: 'af_heart', speed: 1, volume: 1, readerEnabled: true, readerShortcut: 'Ctrl+Alt+R', readerMaxChars: 2000 };
  const voices = [{ id: 'af_heart', name: 'Heart', accent: 'us' }, { id: 'bf_emma', name: 'Emma', accent: 'uk' }];
  const open = (speech: object) => {
    window.history.replaceState(null, '', '/?settings');
    api.invoke.mockImplementation((command: string) => Promise.resolve(({ get_settings: speechSettings, saved_words: [], tts_voices: voices, tts_state: { status: 'notInstalled', progress: 0, message: '', owner: '', ...speech } } as Record<string, unknown>)[command]));
    return render(<AppView />);
  };
  afterEach(() => window.history.replaceState(null, '', '/'));

  it('offers a retry when settings fail to load', async () => {
    window.history.replaceState(null, '', '/?settings');
    let attempts = 0;
    api.invoke.mockImplementation((command: string) => {
      if (command === 'get_settings') return ++attempts === 1 ? Promise.reject('Settings could not be loaded.') : Promise.resolve(speechSettings);
      return Promise.resolve(({ saved_words: [], tts_voices: voices, tts_state: { status: 'ready', progress: 0, message: '', owner: '' } } as Record<string, unknown>)[command]);
    });
    render(<AppView />);
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Settings could not be loaded.');
    fireEvent.click(screen.getByRole('button', { name: 'Retry loading settings' }));
    expect(await screen.findByRole('heading', { name: 'General' })).toBeTruthy();
    expect(attempts).toBe(2);
  });

  it('keeps a large saved-word collection searchable and initially bounded', async () => {
    window.history.replaceState(null, '', '/?settings');
    const saved = Array.from({ length: 1284 }, (_, index) => ({ word: `vocabulary ${index + 1}`, added: 1_700_000_000 - index }));
    api.invoke.mockImplementation((command: string) => Promise.resolve(({ get_settings: speechSettings, saved_words: saved } as Record<string, unknown>)[command]));
    const { container } = render(<AppView />);
    fireEvent.click(await screen.findByRole('tab', { name: 'Saved words' }));
    expect(container.querySelectorAll('.saved-list button')).toHaveLength(60);
    fireEvent.click(screen.getByRole('button', { name: 'Show more saved words' }));
    expect(container.querySelectorAll('.saved-list button')).toHaveLength(120);
    fireEvent.change(screen.getByRole('searchbox', { name: 'Search saved words' }), { target: { value: 'vocabulary 1284' } });
    expect(container.querySelectorAll('.saved-list button')).toHaveLength(1);
    expect(screen.getByRole('button', { name: 'vocabulary 1284' })).toBeTruthy();
    fireEvent.change(screen.getByRole('searchbox', { name: 'Search saved words' }), { target: { value: 'unlisted word' } });
    expect(screen.getByText('No saved words match your search.')).toBeTruthy();
  });

  it('changes the text size from Appearance with a live preview', async () => {
    const { container } = open({});
    fireEvent.click(await screen.findByRole('tab', { name: 'Appearance' }));
    expect(container.querySelector('.popup-sample')?.className).toContain('text-compact');
    fireEvent.change(screen.getByRole('combobox', { name: /Text size/ }), { target: { value: 'large' } });
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('update_settings', { next: expect.objectContaining({ textSize: 'large' }) }));
    expect(container.querySelector('.popup-sample')?.className).toContain('text-large');
  });

  it('installs the voice and saves speech preferences', async () => {
    open({});
    fireEvent.click(await screen.findByRole('tab', { name: 'Pronunciation' }));
    fireEvent.click(screen.getByRole('button', { name: 'Install voice' }));
    expect(api.invoke).toHaveBeenCalledWith('tts_install', undefined);
    fireEvent.change(screen.getByLabelText(/^Voice/), { target: { value: 'bf_emma' } });
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('update_settings', { next: { ...speechSettings, voice: 'bf_emma' } }));
    fireEvent.change(screen.getByLabelText(/^Accent/), { target: { value: 'uk' } });
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('update_settings', { next: { ...speechSettings, voice: 'bf_emma', accent: 'uk' } }));
    fireEvent.click(screen.getByRole('tab', { name: 'Shortcuts' }));
    fireEvent.focus(screen.getByLabelText('Read-aloud shortcut'));
    fireEvent.keyDown(screen.getByLabelText('Read-aloud shortcut'), { key: '!', code: 'Digit1', ctrlKey: true, shiftKey: true });
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('update_settings', { next: { ...speechSettings, voice: 'bf_emma', accent: 'uk', readerShortcut: 'Ctrl+Shift+1' } }));
    expect(screen.queryByText('Save settings')).toBeNull();
  });

  it('reverts a change the backend rejects', async () => {
    window.history.replaceState(null, '', '/?settings');
    api.invoke.mockImplementation((command: string) => command === 'update_settings' ? Promise.reject('Could not register Ctrl+Alt+X: in use. The existing shortcut remains active.') : Promise.resolve(({ get_settings: speechSettings, saved_words: [], tts_voices: voices, tts_state: { status: 'ready', progress: 0, message: '', owner: '' } } as Record<string, unknown>)[command]));
    render(<AppView />);
    const startup = await screen.findByRole('switch', { name: 'Start DIO when I sign in' }) as HTMLInputElement;
    fireEvent.click(startup);
    expect(await screen.findByRole('alert')).toBeTruthy();
    expect(screen.getByRole('alert').textContent).toMatch(/existing shortcut remains active/);
    expect(startup.checked).toBe(false);
  });

  it('offers removal and cancellation for installed or downloading voices', async () => {
    const { unmount } = open({ status: 'ready' });
    fireEvent.click(await screen.findByRole('tab', { name: 'Pronunciation' }));
    fireEvent.click(screen.getByRole('button', { name: 'Remove voice' }));
    expect(api.invoke).toHaveBeenCalledWith('tts_remove', undefined);
    unmount();
    open({ status: 'downloading', progress: 0.5 });
    fireEvent.click(await screen.findByRole('tab', { name: 'Pronunciation' }));
    expect(screen.getByText('Downloading… 50%')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(api.invoke).toHaveBeenCalledWith('tts_cancel_install', undefined);
  });
});
