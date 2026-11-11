import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PopupView } from './main';

const api = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: {} as Record<string, (event: { payload: unknown }) => void>,
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke }));
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
  api.invoke.mockReset().mockImplementation((command: string) => Promise.resolve(command === 'popup_state' ? initial : command === 'get_settings' ? settings : undefined));
});
afterEach(cleanup);

describe('popup', () => {
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
});
