import { afterEach, beforeEach, expect, it, vi } from 'vitest';

let motionChange: (() => void) | undefined;
let reduced = false;
let animate: ReturnType<typeof vi.fn>;

beforeEach(async () => {
  vi.useFakeTimers();
  vi.resetModules();
  reduced = false;
  motionChange = undefined;
  document.documentElement.removeAttribute('data-motion');
  document.body.innerHTML = `<section data-demo>
    <div data-demo-scene class="is-active"></div><div data-demo-scene></div><div data-demo-scene></div>
    <div data-demo-copy class="is-active"></div><div data-demo-copy></div><div data-demo-copy></div>
    <button data-demo-play>Play</button><button data-demo-prev>Previous</button><button data-demo-next>Next</button>
    <span data-demo-status></span>
  </section>`;
  vi.stubGlobal('matchMedia', () => ({ get matches() { return reduced; }, addEventListener: (_: string, callback: () => void) => { motionChange = callback; } }));
  animate = vi.fn(() => ({ cancel: vi.fn() }));
  Object.defineProperty(HTMLElement.prototype, 'animate', { configurable: true, value: animate });
  await import('./assets/demo.ts');
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

it('keeps manual steps instant and cancels an interrupted timed reveal', () => {
  const play = document.querySelector<HTMLButtonElement>('[data-demo-play]')!;
  const previous = document.querySelector<HTMLButtonElement>('[data-demo-prev]')!;
  const next = document.querySelector<HTMLButtonElement>('[data-demo-next]')!;
  next.click();
  previous.click();
  expect(animate).not.toHaveBeenCalled();
  play.click();
  vi.advanceTimersByTime(5000);
  expect(animate).toHaveBeenCalledOnce();
  expect(animate.mock.calls[0][1]).toMatchObject({ duration: 200 });
  play.click();
  expect(animate.mock.results[0].value.cancel).toHaveBeenCalledOnce();
  expect(vi.getTimerCount()).toBe(0);
});

it('stops playback and hides Play when reduced motion becomes active', () => {
  const play = document.querySelector<HTMLButtonElement>('[data-demo-play]')!;
  play.click();
  reduced = true;
  motionChange?.();
  expect(play.hidden).toBe(true);
  expect(vi.getTimerCount()).toBe(0);
  expect(animate).not.toHaveBeenCalled();
});

it('skips timed reveals when site motion is off', () => {
  document.documentElement.dataset.motion = 'off';
  document.querySelector<HTMLButtonElement>('[data-demo-play]')!.click();
  vi.advanceTimersByTime(5000);
  expect(document.querySelector('[data-demo-status]')?.textContent).toBe('2 / 3');
  expect(animate).not.toHaveBeenCalled();
});

it('offers Replay as soon as the final step appears', () => {
  const play = document.querySelector<HTMLButtonElement>('[data-demo-play]')!;
  play.click();
  vi.advanceTimersByTime(10000);
  expect(document.querySelector('[data-demo-status]')?.textContent).toBe('3 / 3');
  expect(play.textContent).toBe('Replay');
  expect(vi.getTimerCount()).toBe(0);
});
