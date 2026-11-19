import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AppView, PopupView } from './main';

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

const settings = { shortcut: 'Ctrl+Alt+D', keyboardEnabled: true, middleClick: false, theme: 'system', size: 'standard', expanded: false, autoplay: false, pronunciation: 'first', startup: false, reducedMotion: true };
const initial = { requestId: 0, status: 'idle', query: '', entries: [], message: '', stale: false, expanded: false, direction: 'se', canBack: false, saved: false };
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

  it('hides absent fields and follows related words', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', query: 'serendipity', entries: [entry], expanded: true });
    expect(screen.getByText('serendipity')).toBeTruthy();
    expect(screen.queryByText('Origin')).toBeNull();
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
    fireEvent.click(screen.getByRole('tab', { name: 'Synonyms' }));
    expect(screen.getByText('Alternatives based on this meaning')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'luck' }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('lookup', { word: 'luck' }));
    const withExample = { ...entry, origin: 'Coined by Horace Walpole.', meanings: [{ ...entry.meanings[0], definitions: [{ ...entry.meanings[0].definitions[0], example: 'A fortunate find.' }] }] };
    send({ requestId: 2, status: 'ready', query: 'serendipity', entries: [withExample] });
    expect(screen.getByRole('tab', { name: 'Definition' }).getAttribute('aria-selected')).toBe('true');
    fireEvent.click(screen.getByRole('tab', { name: 'Examples' }));
    expect(screen.getByText('A fortunate find.')).toBeTruthy();
    fireEvent.click(screen.getByRole('tab', { name: 'Related' }));
    expect(screen.getByText('Origin')).toBeTruthy();
  });

  it('uses the actual expansion direction and handles keyboard dismissal', async () => {
    const { container } = render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', entries: [entry], direction: 'nw', canBack: true });
    expect(container.querySelector('.reduced-motion')).toBeTruthy();
    fireEvent.click(screen.getByText('More ↖'));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('expand', { expanded: true, reducedMotion: false }));
    send({ requestId: 1, status: 'ready', entries: [entry], direction: 'nw', expanded: true, canBack: true });
    expect(screen.getByText('Less ↘')).toBeTruthy();
    fireEvent.keyDown(window, { key: 'Backspace' });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(api.invoke).toHaveBeenCalledWith('back', undefined);
    expect(api.invoke).toHaveBeenCalledWith('close_popup', undefined);
  });

  it('shows sourced IPA in the preferred accent with every variant under More', async () => {
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

  it('pronounces with a matching recording first, otherwise with Kokoro', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['tts-state']).toBeDefined());
    act(() => api.listeners['tts-state']({ payload: { status: 'ready', progress: 0, message: '', owner: '' } }));
    send({ requestId: 1, status: 'ready', entries: [entry] });
    fireEvent.click(screen.getByRole('button', { name: 'Play pronunciation' }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('tts_speak', { text: 'serendipity', owner: 'word' }));
    const recorded = { ...entry, phonetics: [{ text: '', audio: 'https://api.dictionaryapi.dev/media/serendipity-us.mp3', sourceUrl: '' }] };
    send({ requestId: 2, status: 'ready', entries: [recorded] });
    fireEvent.click(screen.getByRole('button', { name: 'Play pronunciation' }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('pronunciation', { url: recorded.phonetics[0].audio }));
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
    expect(screen.queryByText(/More/)).toBeNull();
  });

  it('keeps dismissal available when a lookup fails', async () => {
    render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'network', message: 'Dictionary service returned 502 Bad Gateway.' });
    expect(screen.queryByText('More')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(api.invoke).toHaveBeenCalledWith('back', undefined);
    expect(api.invoke).toHaveBeenCalledWith('close_popup', undefined);
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
    fireEvent.keyDown(screen.getByLabelText('Read-aloud shortcut'), { key: 'q', ctrlKey: true, shiftKey: true });
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('update_settings', { next: { ...speechSettings, voice: 'bf_emma', accent: 'uk', readerShortcut: 'Ctrl+Shift+Q' } }));
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
