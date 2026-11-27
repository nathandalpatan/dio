import { cleanup, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { anchor, checkVersion, compareVersions, parseChangelog, releaseNotes } from '../tools/changelog-parser.mjs';
import { Changelog } from './changelog';
import changelog from '../CHANGELOG.md?raw';

afterEach(cleanup);

const sample = `# Changelog

## [1.1.0] - Unreleased

Summary: Next.

### New features

- Something new.

## [1.0.1] - 2026-12-02

Summary: Fixes.

### Bug fixes

- Fixed **popup** alignment,
  across monitors.
- See [the docs](https://example.com/docs).

## [1.0.0] - 2026-12-01

Summary: First.

### New features

- Lookups.

### Breaking changes

- Removed \`old.json\`.
`;

describe('changelog parser', () => {
  it('reads releases newest first with their sections', () => {
    const releases = parseChangelog(sample);
    expect(releases.map(release => [release.version, release.date])).toEqual([['1.1.0', null], ['1.0.1', '2026-12-02'], ['1.0.0', '2026-12-01']]);
    expect(releases[1].sections).toEqual([{ title: 'Bug fixes', items: ['Fixed **popup** alignment, across monitors.', 'See [the docs](https://example.com/docs).'] }]);
    expect(releases[2].sections.map(section => section.title)).toEqual(['New features', 'Breaking changes']);
  });

  it('writes notes DIO and GitHub read', () => {
    expect(releaseNotes(parseChangelog(sample)[2])).toBe('First.\n\n### New features\n\n- Lookups.\n\n### Breaking changes\n\n- Removed `old.json`.\n');
  });

  it('rejects notes that would publish inconsistently', () => {
    const entry = (version: string, body = 'Summary: x\n\n### Bug fixes\n\n- y\n') => `## [${version}] - 2026-12-01\n\n${body}\n`;
    expect(() => parseChangelog(entry('1.0.0', 'Summary: x\n\n### Misc\n\n- y'))).toThrow(/unknown section "Misc"/);
    expect(() => parseChangelog(entry('1.0.0', '### Bug fixes\n\n- y'))).toThrow(/needs a "Summary:"/);
    expect(() => parseChangelog(entry('1.0.0', 'Summary: x'))).toThrow(/lists no changes/);
    expect(() => parseChangelog(entry('1.0.0') + entry('1.0.1'))).toThrow(/newest first/);
    expect(() => parseChangelog(entry('1.0.0') + entry('1.0.0'))).toThrow(/newest first/);
    expect(() => parseChangelog('## 1.0.0\n')).toThrow(/expected "## \[x\.y\.z\]/);
    expect(() => parseChangelog(entry('1.0.0', 'Summary: x\n\n- y'))).toThrow(/under a ### section/);
  });

  it('orders versions and builds the anchors DIO links to', () => {
    expect(compareVersions('1.0.0', '1.0.0-rc.1')).toBeGreaterThan(0);
    expect(compareVersions('0.10.0', '0.9.9')).toBeGreaterThan(0);
    expect(anchor('0.2.1')).toBe('v0-2-1');
    expect(anchor('1.0.0-rc.1')).toBe('v1-0-0-rc-1');
  });

  it('parses the real CHANGELOG.md', () => {
    const releases = parseChangelog(changelog);
    expect(releases.length).toBeGreaterThan(1);
    expect(releases.every(release => release.summary)).toBe(true);
  });
});

describe('version check', () => {
  const files = (version: string, pubkey = 'key') => ({
    tauriConf: JSON.stringify({ version, plugins: { updater: { pubkey } } }),
    cargoToml: `[package]\nname = "dio"\nversion = "${version}"\n\n[dependencies]\nserde = { version = "1" }\n`,
    packageJson: JSON.stringify({ version }),
  });

  it('passes when every file agrees', () => {
    expect(checkVersion({ version: '1.0.1', changelog: sample, ...files('1.0.1') })).toEqual([]);
  });

  it('names each file with a different version', () => {
    const problems = checkVersion({ version: '1.0.1', changelog: sample, ...files('1.0.1'), packageJson: JSON.stringify({ version: '1.0.0' }) });
    expect(problems).toEqual(['package.json has version 1.0.0, expected 1.0.1.']);
  });

  it('requires a changelog entry', () => {
    expect(checkVersion({ version: '2.0.0', changelog: sample, ...files('2.0.0') })).toEqual(['CHANGELOG.md has no entry for 2.0.0.']);
  });

  it('allows an unreleased entry on main but not in a release', () => {
    expect(checkVersion({ version: '1.1.0', changelog: sample, ...files('1.1.0') })).toEqual([]);
    expect(checkVersion({ version: '1.1.0', changelog: sample, ...files('1.1.0'), release: true })).toEqual(['CHANGELOG.md: set the release date for 1.1.0 (it says Unreleased).']);
  });

  it('refuses to release an older entry or without the updater key', () => {
    expect(checkVersion({ version: '1.0.1', changelog: sample, ...files('1.0.1'), release: true })).toEqual(['CHANGELOG.md: 1.0.1 must be the newest entry.']);
    const released = sample.replace('1.1.0] - Unreleased', '1.1.0] - 2026-12-03');
    expect(checkVersion({ version: '1.1.0', changelog: released, ...files('1.1.0', ''), release: true })).toEqual([expect.stringMatching(/pubkey is empty/)]);
  });
});

describe('changelog page', () => {
  it('lists released versions newest first and marks the latest', () => {
    const { container } = render(<Changelog releases={parseChangelog(sample)}/>);
    const headings = within(container.querySelector('main')!).getAllByRole('heading', { level: 2 }).map(heading => heading.textContent);
    expect(headings).toEqual(['Version 1.0.1', 'Version 1.0.0']);
    expect(container.querySelector('#v1-0-1 .changelog-latest')?.textContent).toBe('Latest');
    expect(container.querySelector('#v1-0-0 .changelog-latest')).toBeNull();
    expect(screen.getByText('2 December 2026')).toBeTruthy();
    expect(screen.getByRole('link', { name: 'Version 1.0.1' }).getAttribute('href')).toBe('#v1-0-1');
  });

  it('renders inline formatting and only https links', () => {
    render(<Changelog releases={parseChangelog(sample.replace('https://example.com/docs', 'javascript:alert(1)'))}/>);
    expect(screen.getByText('popup').tagName).toBe('STRONG');
    expect(screen.queryByRole('link', { name: 'the docs' })).toBeNull();
  });

  it('links back to the home page and to itself', () => {
    render(<Changelog releases={parseChangelog(sample)}/>);
    expect(screen.getByRole('link', { name: 'How it works' }).getAttribute('href')).toBe('../#how');
    expect(screen.getByRole('link', { name: 'Changelog' }).getAttribute('href')).toBe('../changelog/');
  });
});
