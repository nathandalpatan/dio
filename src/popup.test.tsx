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

  it('uses the actual expansion direction and handles keyboard dismissal', async () => {
    const { container } = render(<PopupView />);
    await waitFor(() => expect(api.listeners['popup-state']).toBeDefined());
    send({ requestId: 1, status: 'ready', entries: [entry], direction: 'nw', canBack: true });
    expect(container.querySelector('.reduced-motion')).toBeTruthy();
    fireEvent.click(screen.getByText('↖ More'));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('expand', { expanded: true, reducedMotion: false }));
    send({ requestId: 1, status: 'ready', entries: [entry], direction: 'nw', expanded: true, canBack: true });
    expect(screen.getByText('↘ Less')).toBeTruthy();
    fireEvent.keyDown(window, { key: 'Backspace' });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(api.invoke).toHaveBeenCalledWith('back', undefined);
    expect(api.invoke).toHaveBeenCalledWith('close_popup', undefined);
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
