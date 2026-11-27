// Release-note commands for CI and releases.
//
//   node tools/changelog.mjs version             Prints the version in src-tauri/tauri.conf.json.
//   node tools/changelog.mjs check <version>     Checks the version files agree and CHANGELOG.md has the entry.
//   node tools/changelog.mjs check <version> --release
//                                                Also requires a release date and the updater public key.
//   node tools/changelog.mjs notes <version>     Prints the release notes for GitHub and the update manifest.

import { readFileSync } from 'node:fs';
import { checkVersion, parseChangelog, releaseNotes } from './changelog-parser.mjs';

const root = new URL('../', import.meta.url);
const read = (path) => readFileSync(new URL(path, root), 'utf8');
const [command, value, flag] = process.argv.slice(2);
const version = value?.replace(/^v/, '');

if (command === 'version') {
  console.log(JSON.parse(read('src-tauri/tauri.conf.json')).version);
} else if (command === 'check' && version) {
  const problems = checkVersion({
    version,
    changelog: read('CHANGELOG.md'),
    tauriConf: read('src-tauri/tauri.conf.json'),
    cargoToml: read('src-tauri/Cargo.toml'),
    packageJson: read('package.json'),
    release: flag === '--release',
  });
  for (const problem of problems) console.error(problem);
  if (problems.length) process.exit(1);
  console.log(`${version} is consistent.`);
} else if (command === 'notes' && version) {
  const release = parseChangelog(read('CHANGELOG.md')).find(item => item.version === version);
  if (!release) {
    console.error(`CHANGELOG.md has no entry for ${version}.`);
    process.exit(1);
  }
  process.stdout.write(releaseNotes(release));
} else {
  console.error('Usage: node tools/changelog.mjs version | check <version> [--release] | notes <version>');
  process.exit(2);
}
