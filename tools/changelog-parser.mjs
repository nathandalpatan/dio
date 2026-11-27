// Reads CHANGELOG.md, the single source for release notes on GitHub, in the update manifest DIO
// reads, and on the website's changelog page. Pure functions only, so the website can use it too.

/** @typedef {{ title: string, items: string[] }} Section */
/** @typedef {{ version: string, date: string | null, summary: string, sections: Section[] }} Release */

export const SECTIONS = ['New features', 'Improvements', 'Bug fixes', 'Breaking changes'];

const HEADING = /^## \[(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)\] - (\d{4}-\d{2}-\d{2}|Unreleased)\s*$/;

/** @param {string} version */
function parts(version) {
  const [core, pre = ''] = version.split(/-(.*)/s);
  return { numbers: core.split('.').map(Number), pre };
}

/** Semantic version order: negative when `a` is older than `b`. @param {string} a @param {string} b */
export function compareVersions(a, b) {
  const left = parts(a);
  const right = parts(b);
  for (let i = 0; i < 3; i++) if (left.numbers[i] !== right.numbers[i]) return left.numbers[i] - right.numbers[i];
  if (left.pre === right.pre) return 0;
  if (!left.pre) return 1;
  if (!right.pre) return -1;
  return left.pre < right.pre ? -1 : 1;
}

/**
 * Parses every release, newest first. Throws on anything that would publish inconsistent notes:
 * unknown sections, entries without a summary, duplicates or releases out of order.
 * @param {string} text
 * @returns {Release[]}
 */
export function parseChangelog(text) {
  /** @type {Release[]} */
  const releases = [];
  /** @type {Release | null} */
  let release = null;
  /** @type {Section | null} */
  let section = null;
  for (const [index, line] of text.split(/\r?\n/).entries()) {
    const where = `CHANGELOG.md line ${index + 1}`;
    if (line.startsWith('## ')) {
      const match = HEADING.exec(line);
      if (!match) throw new Error(`${where}: expected "## [x.y.z] - YYYY-MM-DD" or "## [x.y.z] - Unreleased".`);
      release = { version: match[1], date: match[2] === 'Unreleased' ? null : match[2], summary: '', sections: [] };
      section = null;
      releases.push(release);
    } else if (!release) {
      continue;
    } else if (line.startsWith('### ')) {
      const title = line.slice(4).trim();
      if (!SECTIONS.includes(title)) throw new Error(`${where}: unknown section "${title}". Use one of: ${SECTIONS.join(', ')}.`);
      section = { title, items: [] };
      release.sections.push(section);
    } else if (line.startsWith('Summary:')) {
      release.summary = line.slice('Summary:'.length).trim();
    } else if (/^[-*] /.test(line)) {
      if (!section) throw new Error(`${where}: list items must be under a ### section.`);
      section.items.push(line.slice(2).trim());
    } else if (/^\s+\S/.test(line) && section?.items.length) {
      section.items[section.items.length - 1] += ` ${line.trim()}`;
    } else if (line.trim()) {
      throw new Error(`${where}: unexpected text. Put it in "Summary:" or a list item.`);
    }
  }
  for (const [index, entry] of releases.entries()) {
    if (!entry.summary) throw new Error(`CHANGELOG.md: ${entry.version} needs a "Summary:" line.`);
    if (!entry.sections.some(item => item.items.length)) throw new Error(`CHANGELOG.md: ${entry.version} lists no changes.`);
    const next = releases[index + 1];
    if (next && compareVersions(entry.version, next.version) <= 0) throw new Error(`CHANGELOG.md: ${entry.version} must come before ${next.version}, newest first.`);
  }
  return releases;
}

/** The page anchor for a version, e.g. `v0-2-1`. DIO builds the same anchor for its links. @param {string} version */
export function anchor(version) {
  return `v${version.replace(/[^0-9A-Za-z]/g, '-')}`;
}

/** Markdown for the GitHub release and the update manifest: the summary, then each section. @param {Release} release */
export function releaseNotes(release) {
  const sections = release.sections
    .filter(section => section.items.length)
    .map(section => `### ${section.title}\n\n${section.items.map(item => `- ${item}`).join('\n')}`);
  return [release.summary, ...sections].join('\n\n') + '\n';
}

/**
 * Checks that a version is ready to build: the three version files agree and CHANGELOG.md has an
 * entry. A release also needs a date and the updater's public key. Returns the problems found.
 * @param {{ version: string, changelog: string, tauriConf: string, cargoToml: string, packageJson: string, release?: boolean }} input
 * @returns {string[]}
 */
export function checkVersion({ version, changelog, tauriConf, cargoToml, packageJson, release = false }) {
  const problems = [];
  const conf = JSON.parse(tauriConf);
  const versions = {
    'src-tauri/tauri.conf.json': conf.version,
    'src-tauri/Cargo.toml': /^\[package\][^[]*?^version\s*=\s*"([^"]+)"/m.exec(cargoToml)?.[1],
    'package.json': JSON.parse(packageJson).version,
  };
  for (const [file, found] of Object.entries(versions)) {
    if (found !== version) problems.push(`${file} has version ${found ?? '(none)'}, expected ${version}.`);
  }
  let releases = [];
  try {
    releases = parseChangelog(changelog);
  } catch (error) {
    problems.push(/** @type {Error} */ (error).message);
  }
  const entry = releases.find(item => item.version === version);
  if (!entry && !problems.some(problem => problem.startsWith('CHANGELOG.md'))) problems.push(`CHANGELOG.md has no entry for ${version}.`);
  if (release) {
    if (entry && !entry.date) problems.push(`CHANGELOG.md: set the release date for ${version} (it says Unreleased).`);
    if (entry && releases[0] !== entry) problems.push(`CHANGELOG.md: ${version} must be the newest entry.`);
    if (!conf.plugins?.updater?.pubkey) problems.push('src-tauri/tauri.conf.json: plugins.updater.pubkey is empty, so installed copies could not verify this release.');
  }
  return problems;
}
