/// <reference types="vite/client" />
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AppView, UpdatesPane } from './main';

const api = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: {} as Record<string, (event: { payload: unknown }) => void>,
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: api.invoke, isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async (name: string, listener: (event: { payload: unknown }) => void) => {
  api.listeners[name] = listener;
  return () => { delete api.listeners[name]; };
}) }));

const base = { status: 'idle', current: '0.2.0', version: '', notes: [], progress: 0, error: '', retry: '', lastChecked: null };
const available = { ...base, status: 'available', version: '0.2.1', notes: ['Faster lookups.', 'Clearer playback.'] };
let state: object = base;
const send = (payload: object) => act(() => api.listeners['update-state']({ payload: { ...base, ...payload } }));
const button = (name: string) => screen.getByRole('button', { name });
const commands = () => api.invoke.mock.calls.map(([command]) => command);

beforeEach(() => {
  api.listeners = {};
  state = base;
  api.invoke.mockReset().mockImplementation((command: string) => Promise.resolve(command === 'update_state' ? state : undefined));
});
afterEach(cleanup);

async function pane(mode: 'auto' | 'notify' | 'manual' = 'auto', onMode = vi.fn(), onError = vi.fn()) {
  render(<UpdatesPane mode={mode} onMode={onMode} onError={onError}/>);
  await screen.findByText(/DIO v0\.2\.0/);
  return { onMode, onError };
}

describe('updates pane', () => {
  it('shows the version and offers a check', async () => {
    await pane('manual');
    expect(screen.getByText('Not checked yet')).toBeTruthy();
    fireEvent.click(button('Check for updates'));
    expect(commands()).toContain('check_for_updates');
  });

  it('shows a successful check with when it happened', async () => {
    state = { ...base, status: 'upToDate', lastChecked: Math.floor(Date.now() / 1000) - 120 };
    await pane();
    expect(screen.getByText(/You're up to date · Last checked 2 min ago/)).toBeTruthy();
  });

  it('disables checking while an update is busy', async () => {
    state = { ...available, status: 'downloading', progress: 0.68 };
    await pane();
    expect((button('Check for updates') as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('68');
    expect(screen.getByText('68%')).toBeTruthy();
    fireEvent.click(button('Cancel download'));
    expect(commands()).toContain('cancel_update');
  });

  it('offers the available update with its notes', async () => {
    state = available;
    await pane();
    expect(screen.getByText('DIO v0.2.1 is available')).toBeTruthy();
    expect(screen.getByText('Current version: v0.2.0')).toBeTruthy();
    expect(screen.getByText('Clearer playback.')).toBeTruthy();
    fireEvent.click(button('Read full release notes'));
    expect(api.invoke).toHaveBeenCalledWith('open_release_notes', { version: '0.2.1' });
    fireEvent.click(button('Remind me later'));
    fireEvent.click(button('Download & install'));
    expect(commands()).toEqual(expect.arrayContaining(['remind_later', 'download_update']));
  });

  it('follows state events through verification to a restart', async () => {
    await pane();
    send({ ...available, status: 'verifying', progress: 1 });
    expect(screen.getByText('Verifying update')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Cancel download' })).toBeNull();
    send({ ...available, status: 'ready' });
    fireEvent.click(button('Later'));
    expect(commands()).toContain('postpone_update');
    send({ ...available, status: 'restartRequired' });
    expect(screen.getByText('Restart required')).toBeTruthy();
    fireEvent.click(button('Restart and update'));
    expect(commands()).toContain('install_update');
    send({ ...available, status: 'installing' });
    expect(screen.getByText('Installing DIO v0.2.1')).toBeTruthy();
    expect(screen.queryAllByRole('button', { name: /Restart|Later|Cancel/ })).toHaveLength(0);
  });

  it('explains failures and retries the failed step', async () => {
    await pane();
    send({ ...available, status: 'failed', retry: 'download', error: 'The update could not be verified. Its signature does not match DIO’s signing key, so it was discarded.' });
    expect(screen.getByRole('alert').textContent).toMatch(/Unable to download update.*could not be verified.*remains available/);
    fireEvent.click(button('Try again'));
    expect(commands()).toContain('retry_update');
    send({ ...available, status: 'failed', retry: 'install', error: 'The installer did not finish.' });
    expect(screen.getByRole('alert').textContent).toMatch(/Unable to install update.*DIO-setup\.exe.*settings and saved words are kept/);
    fireEvent.click(button('Dismiss'));
    expect(commands()).toContain('dismiss_update');
  });

  it('confirms an installed update and links to its notes', async () => {
    state = { ...base, status: 'updated', current: '0.2.1', notes: ['Faster lookups.'] };
    render(<UpdatesPane mode="auto" onMode={vi.fn()} onError={vi.fn()}/>);
    await screen.findByText('DIO has been updated to v0.2.1');
    fireEvent.click(button("View what's new"));
    expect(api.invoke).toHaveBeenCalledWith('open_release_notes', { version: '0.2.1' });
  });

  it('changes the update preference', async () => {
    const { onMode } = await pane('auto');
    expect((screen.getByRole('radio', { name: /Automatically install updates/ }) as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByRole('radio', { name: /Manually check for updates/ }));
    expect(onMode).toHaveBeenCalledWith('manual');
  });

  it('reports command errors', async () => {
    const { onError } = await pane();
    api.invoke.mockImplementation((command: string) => command === 'open_release_notes' ? Promise.reject('Could not open your web browser.') : Promise.resolve());
    fireEvent.click(button('Release notes'));
    await waitFor(() => expect(onError).toHaveBeenCalledWith('Could not open your web browser.'));
  });
});

describe('settings', () => {
  const settings = { shortcut: 'Ctrl+Alt+D', keyboardEnabled: true, middleClick: false, theme: 'light', size: 'standard', expanded: false, autoplay: false, startup: false, reducedMotion: false };

  it('saves a new update preference immediately', async () => {
    window.history.replaceState(null, '', '/?settings&section=updates');
    api.invoke.mockImplementation((command: string) => Promise.resolve(command === 'get_settings' ? settings : command === 'update_state' ? base : command === 'saved_words' ? [] : undefined));
    render(<AppView/>);
    fireEvent.click(await screen.findByRole('radio', { name: /Notify me when updates are available/ }));
    await waitFor(() => expect(api.invoke).toHaveBeenCalledWith('update_settings', { next: expect.objectContaining({ updateMode: 'notify' }) }));
    window.history.replaceState(null, '', '/');
  });

  it('opens on the section a notification asks for', async () => {
    window.history.replaceState(null, '', '/?settings');
    api.invoke.mockImplementation((command: string) => Promise.resolve(command === 'get_settings' ? settings : command === 'update_state' ? base : command === 'saved_words' ? [] : undefined));
    render(<AppView/>);
    await screen.findByRole('tab', { name: /General/ });
    await act(async () => api.listeners['settings-section']({ payload: 'updates' }));
    expect(screen.getByRole('tab', { name: /Updates/ }).getAttribute('aria-selected')).toBe('true');
    window.history.replaceState(null, '', '/');
  });
});

it('keeps the QA stub out of the production app', () => {
  const sources = import.meta.glob<string>(['./*.{ts,tsx}', '!./*.test.*'], { query: '?raw', import: 'default', eager: true });
  expect(Object.keys(sources)).toContain('./main.tsx');
  for (const [file, source] of Object.entries(sources)) expect(source, file).not.toMatch(/from ['"][^'"]*tauri-stub/);
});
