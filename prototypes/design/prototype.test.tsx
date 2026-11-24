import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { WebsiteA } from './website/WebsiteA';
import { WebsiteB } from './website/WebsiteB';
import { WebsiteC } from './website/WebsiteC';
import { SettingsA } from './settings/SettingsA';
import { SettingsC } from './settings/SettingsC';
import { Picker, useVariant } from './Picker';

beforeEach(() => window.history.replaceState(null, '', '/prototypes/website/'));
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });

describe('website concepts', () => {
  it('keeps the install path and product story in every variation', () => {
    for (const Concept of [WebsiteA, WebsiteB, WebsiteC]) {
      const view = render(<Concept/>);
      expect(screen.getAllByRole('link', { name: 'Download for Windows' }).length).toBeGreaterThan(1);
      expect(screen.getByRole('heading', { name: 'Install DIO in 3 steps' })).toBeTruthy();
      view.unmount();
    }
  });

  it('changes the walkthrough and real-app gallery', () => {
    render(<WebsiteA/>);
    fireEvent.click(screen.getByRole('button', { name: 'Next step' }));
    expect(screen.getByRole('heading', { name: 'Open DIO beside it' })).toBeTruthy();
    fireEvent.click(screen.getByRole('tab', { name: 'Acrobat' }));
    expect(screen.getByAltText('DIO popup in Acrobat')).toBeTruthy();
    fireEvent.click(screen.getAllByRole('button', { name: /Expand/ })[0]);
    expect(screen.getByRole('button', { name: /Collapse/ })).toBeTruthy();
  });

  it('shows the Context product scene and opens a keyboard-dismissable menu', () => {
    render(<WebsiteB/>);
    expect(screen.getAllByText('collisions').length).toBeGreaterThan(1);
    expect(screen.getByRole('img', { name: /Animated demonstration: Neo asks Remi/ })).toBeTruthy();
    expect(screen.getByText(/Something that is ephemeral is only temporary/)).toBeTruthy();
    expect(screen.getByText('Select a word, press Ctrl + Alt + D, and read beside your source.')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Next step' })).toBeNull();
    const menu = screen.getByRole('button', { name: 'Menu' });
    fireEvent.click(menu);
    expect(menu.getAttribute('aria-expanded')).toBe('true');
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(menu.getAttribute('aria-expanded')).toBe('false');
  });

  it('pauses and replays the Context animation', () => {
    render(<WebsiteB/>);
    const demo = screen.getByRole('img', { name: /Animated demonstration/ }).parentElement!;
    const firstCycle = demo.querySelector('.context-demo__animation');
    expect(demo.dataset.running).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: 'Pause demo' }));
    expect(demo.dataset.running).toBe('false');
    fireEvent.click(screen.getByRole('button', { name: 'Replay demo' }));
    expect(demo.dataset.running).toBe('true');
    expect(demo.querySelector('.context-demo__animation')).not.toBe(firstCycle);
  });

  it('pauses the Context animation when hidden or out of view', () => {
    let notify: IntersectionObserverCallback = () => {};
    vi.stubGlobal('IntersectionObserver', class {
      constructor(callback: IntersectionObserverCallback) { notify = callback; }
      observe() {}
      disconnect() {}
    });
    const hidden = vi.spyOn(document, 'hidden', 'get').mockReturnValue(false);
    render(<WebsiteB/>);
    const demo = screen.getByRole('img', { name: /Animated demonstration/ }).parentElement!;
    expect(demo.dataset.running).toBe('false');
    act(() => notify([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver));
    expect(demo.dataset.running).toBe('true');
    act(() => notify([{ isIntersecting: false } as IntersectionObserverEntry], {} as IntersectionObserver));
    expect(demo.dataset.running).toBe('false');
    act(() => notify([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver));
    hidden.mockReturnValue(true);
    fireEvent(document, new Event('visibilitychange'));
    expect(demo.dataset.running).toBe('false');
  });

  it('shows the completed Context lookup without motion when reduced motion is preferred', () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true, addEventListener() {}, removeEventListener() {} }));
    render(<WebsiteB/>);
    const demo = screen.getByRole('img', { name: /Animated demonstration/ }).parentElement!;
    expect(demo.dataset.reducedMotion).toBe('true');
    expect(demo.dataset.running).toBe('false');
    expect(screen.queryByRole('button', { name: 'Pause demo' })).toBeNull();
  });
});

describe('Settings concepts', () => {
  it('navigates, changes controls, and reports the local voice state', () => {
    render(<SettingsA/>);
    fireEvent.click(screen.getByRole('tab', { name: 'Pronunciation' }));
    fireEvent.click(screen.getByRole('switch', { name: 'Auto-play pronunciation' }));
    expect(screen.getByRole('switch', { name: 'Auto-play pronunciation' })).toHaveProperty('checked', true);
    fireEvent.click(screen.getByRole('button', { name: 'Install voice' }));
    expect(screen.getByText('Voice ready')).toBeTruthy();
  });

  it('keeps task navigation tied to its panel and changes appearance', () => {
    render(<SettingsC/>);
    fireEvent.click(screen.getByRole('tab', { name: /Appearance/ }));
    expect(screen.getByRole('heading', { name: 'Appearance' })).toBeTruthy();
    fireEvent.change(screen.getByLabelText('Theme'), { target: { value: 'dark' } });
    expect(document.querySelector('.settings-stage')?.classList.contains('theme-dark')).toBe(true);
  });

  it('opens a saved word preview inside the isolated Settings view', () => {
    render(<SettingsA/>);
    fireEvent.click(screen.getByRole('tab', { name: 'Saved words' }));
    fireEvent.click(screen.getByRole('button', { name: /serendipity/ }));
    expect(screen.getByText('The occurrence of a pleasant discovery by chance.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Close word preview' }));
    expect(screen.queryByText('The occurrence of a pleasant discovery by chance.')).toBeNull();
  });
});

function PickerHarness() {
  const { variant, setVariant, setReplay } = useVariant();
  return <Picker variant={variant} setVariant={setVariant} setReplay={setReplay}/>;
}

it('switches variants with the picker and keyboard', () => {
  render(<PickerHarness/>);
  fireEvent.click(screen.getByRole('button', { name: 'Context' }));
  expect(screen.getByRole('button', { name: 'Context' }).getAttribute('aria-current')).toBe('true');
  fireEvent.keyDown(window, { key: '3' });
  expect(screen.getByRole('button', { name: 'Direct' }).getAttribute('aria-current')).toBe('true');
});
