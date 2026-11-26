import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from './main';
import { entries } from './shared';

afterEach(cleanup);
beforeEach(() => window.history.replaceState(null, '', '/'));

describe('lookup concept picker', () => {
  it('previews an example and navigates Quiet Reference synonyms and back', () => {
    const originalFact = entries.fact;
    entries.fact = { ...originalFact, examples: [] };
    try {
      render(<App />);
      expect(screen.getByText('Example')).toBeTruthy();
      expect(screen.getByText('Don’t lie to me. Tell me the truth.')).toBeTruthy();
      expect(screen.queryByRole('button', { name: 'Forward to next word' })).toBeNull();
      expect(screen.queryByRole('tab', { name: 'Related' })).toBeNull();
      fireEvent.keyDown(screen.getByRole('tablist', { name: 'Entry sections' }), { key: 'ArrowRight' });
      expect(screen.getByRole('tab', { name: 'Examples' }).getAttribute('aria-selected')).toBe('true');
      fireEvent.click(screen.getByRole('tab', { name: 'Synonyms' }));
      fireEvent.click(screen.getByRole('button', { name: 'fact' }));
      expect(screen.getByRole('heading', { name: 'fact' })).toBeTruthy();
      expect(screen.queryByText('Example')).toBeNull();
      expect(screen.queryByRole('tab', { name: 'Examples' })).toBeNull();
      expect(screen.queryByRole('tab', { name: 'Synonyms' })).toBeNull();
      fireEvent.click(screen.getByRole('button', { name: 'Speech settings' }));
      expect(screen.getByRole('dialog', { name: 'Speech settings' })).toBeTruthy();
      fireEvent.click(screen.getByRole('button', { name: 'Back to previous word' }));
      expect(screen.getByRole('heading', { name: 'truth' })).toBeTruthy();
      expect(screen.queryByRole('dialog', { name: 'Speech settings' })).toBeNull();
      fireEvent.click(screen.getByRole('button', { name: 'Forward to next word' }));
      expect(screen.getByRole('heading', { name: 'fact' })).toBeTruthy();
      expect(screen.queryByRole('button', { name: 'Forward to next word' })).toBeNull();
      fireEvent.click(screen.getByRole('button', { name: 'Back to previous word' }));
      fireEvent.click(screen.getByRole('tab', { name: 'Synonyms' }));
      fireEvent.click(screen.getByRole('button', { name: 'fact' }));
      expect(screen.queryByRole('button', { name: 'Forward to next word' })).toBeNull();
      fireEvent.click(screen.getByRole('button', { name: 'Back to previous word' }));
      fireEvent.click(screen.getByRole('button', { name: 'Back to reading' }));
      expect(screen.getByText('Lookup closed')).toBeTruthy();
      fireEvent.change(screen.getByLabelText('Scenario'), { target: { value: 'serendipity' } });
      fireEvent.click(screen.getByRole('button', { name: 'Expand' }));
      expect(screen.getByText('Coined by Horace Walpole in 1754.')).toBeTruthy();
      expect(screen.getByText('Source: Free Dictionary API')).toBeTruthy();
    } finally {
      entries.fact = originalFact;
    }
  });

  it('replaces the forward tail after a new lookup', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('tab', { name: 'Synonyms' }));
    fireEvent.click(screen.getByRole('button', { name: 'fact' }));
    fireEvent.click(screen.getByRole('button', { name: 'Back to previous word' }));
    expect(screen.getByRole('button', { name: 'Forward to next word' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Word Stage' }));
    fireEvent.click(screen.getByRole('button', { name: /Related words/ }));
    fireEvent.click(screen.getByRole('button', { name: 'reality' }));
    fireEvent.click(screen.getByRole('button', { name: 'Back to previous word' }));
    fireEvent.click(screen.getByRole('button', { name: 'Quiet Reference' }));
    fireEvent.click(screen.getByRole('button', { name: 'Forward to next word' }));
    expect(screen.getByRole('heading', { name: 'reality' })).toBeTruthy();
  });

  it('switches three distinct working layouts', () => {
    render(<App />);
    expect(screen.getByRole('region', { name: 'Quiet Reference lookup' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Word Stage' }));
    expect(screen.getByRole('region', { name: 'Word Stage lookup' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Fast Lookup' }));
    expect(screen.getByRole('region', { name: 'Fast Lookup reference panel' })).toBeTruthy();
  });

  it('preserves lookup, back, save, and expansion behavior across concepts', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: 'Word Stage' }));
    fireEvent.click(screen.getByRole('button', { name: /Related words/ }));
    fireEvent.click(screen.getByRole('button', { name: 'fact' }));
    expect(screen.getByRole('heading', { name: 'fact' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Back to previous word' }));
    expect(screen.getByRole('heading', { name: 'truth' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(screen.getByRole('button', { name: 'Saved' }).getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: 'Expand entry' }));
    expect(screen.getByRole('heading', { name: 'More meanings' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Fast Lookup' }));
    expect(screen.getByRole('heading', { name: 'truth' })).toBeTruthy();
  });

  it('shows status scenarios and closes then reopens the popup', () => {
    render(<App />);
    fireEvent.change(screen.getByLabelText('Scenario'), { target: { value: 'not-found' } });
    expect(screen.getByText('No definition found. Try another selection.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Close lookup' }));
    expect(screen.getByText('Lookup closed')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Open lookup' }));
    expect(screen.getByRole('region', { name: 'Quiet Reference lookup' })).toBeTruthy();
  });

  it('switches the two Fast Lookup sections like Quiet Reference tabs', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: 'Fast Lookup' }));
    const panel = screen.getByRole('region', { name: 'Fast Lookup reference panel' });
    expect(panel.querySelector('.fast-footer')?.querySelectorAll('button')).toHaveLength(2);
    fireEvent.click(screen.getByRole('button', { name: 'Save word' }));
    expect(screen.getByRole('button', { name: 'Remove saved word' }).getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: 'Voice' }));
    expect(screen.getByRole('dialog', { name: 'Speech settings' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Close speech settings' }));
    expect(screen.getAllByRole('tab')).toHaveLength(2);
    expect(screen.getByRole('tab', { name: 'More meanings' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByText('True facts, genuine depiction or statements of reality.')).toBeTruthy();
    fireEvent.click(screen.getByRole('tab', { name: 'Related & source' }));
    expect(screen.getByRole('button', { name: 'fact' })).toBeTruthy();
    fireEvent.keyDown(screen.getByRole('tablist', { name: 'Entry sections' }), { key: 'ArrowRight' });
    expect(screen.getByRole('tab', { name: 'More meanings' }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByRole('heading', { name: 'Definition' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Expand' }));
    expect(screen.getByText('Don’t lie to me. Tell me the truth.')).toBeTruthy();
    fireEvent.change(screen.getByLabelText('Scenario'), { target: { value: 'loading' } });
    expect(screen.getByRole('button', { name: 'Voice settings' })).toBeTruthy();
  });
});
