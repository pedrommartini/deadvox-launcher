import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, ''));
const cargo = spawnSync('cargo', ['metadata', '--manifest-path', 'app/src-tauri/Cargo.toml',
  '--locked', '--offline', '--format-version', '1', '--filter-platform', 'x86_64-pc-windows-msvc'], {
  cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024,
  env: { ...process.env, CARGO_HOME: path.join(root, '.cache/cargo-home') },
});
if (cargo.error || cargo.status !== 0) throw new Error(cargo.error?.message || cargo.stderr);
const metadata = JSON.parse(cargo.stdout);
const lock = readJson(path.join(root, 'app/package-lock.json'));
const app = readJson(path.join(root, 'app/package.json'));
const entries = [];
const overrides = readJson(path.join(root, 'docs/third-party-license-overrides.json'));

function licenseFiles(directory, explicit) {
  const files = new Set();
  function visit(current, depth = 0) {
    for (const item of fs.readdirSync(current, { withFileTypes: true })) {
      const full = path.join(current, item.name);
      if (item.isFile() && /^(licen[cs]e|copying|copyright|notice|unlicense)([._-]|$)/i.test(item.name)) files.add(full);
      if (item.isDirectory() && depth < 2 && /^(licen[cs]es?|notices?)$/i.test(item.name)) visit(full, depth + 1);
    }
  }
  visit(directory);
  if (explicit) {
    const full = path.resolve(directory, explicit);
    if (!full.startsWith(directory + path.sep)) throw new Error('License path escapes its dependency.');
    files.add(full);
  }
  return [...files].sort().map(file => ({
    name: path.relative(directory, file).split(path.sep).join('/'),
    text: fs.readFileSync(file, 'utf8').replace(/\r\n/g, '\n').trim(),
  }));
}

for (const pkg of metadata.packages.filter(pkg => pkg.source)) {
  const directory = path.dirname(pkg.manifest_path);
  let licenses = licenseFiles(directory, pkg.license_file);
  const supplement = overrides[`${pkg.name}@${pkg.version}`];
  if (!licenses.length && supplement) {
    const vcs = readJson(path.join(directory, '.cargo_vcs_info.json'));
    if (vcs.git.sha1 !== supplement.revision) throw new Error(`Upstream revision changed: ${pkg.name}`);
    licenses = supplement.licenses;
  }
  if (!pkg.license || !licenses.length) throw new Error(`Missing license metadata or text: Rust ${pkg.name} ${pkg.version}`);
  entries.push({ ecosystem: 'Rust', name: pkg.name, version: pkg.version, license: pkg.license,
    url: `https://crates.io/api/v1/crates/${pkg.name}/${pkg.version}/download`, licenses });
}
for (const [location, pkg] of Object.entries(lock.packages)) {
  if (!location || pkg.dev) continue;
  const directory = path.join(root, 'app', location);
  const installed = readJson(path.join(directory, 'package.json'));
  if (installed.version !== pkg.version) throw new Error(`Installed dependency differs from lockfile: ${location}`);
  const licenses = licenseFiles(directory);
  if (!installed.license || !licenses.length) throw new Error(`Missing license metadata or text: npm ${installed.name}`);
  entries.push({ ecosystem: 'npm', name: installed.name, version: pkg.version, license: installed.license,
    url: `https://www.npmjs.com/package/${installed.name}/v/${pkg.version}`, licenses });
}
entries.sort((a, b) => a.ecosystem.localeCompare(b.ecosystem) || a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
const texts = new Map();
for (const pkg of entries) for (const file of pkg.licenses) {
  if (!texts.has(file.text)) texts.set(file.text, texts.size + 1);
}
const lines = [
  `DeadVox ${app.version} - Third-party notices`, '',
  'DeadVox application source is proprietary. Third-party components retain their own licenses.',
  'This inventory includes the locked Windows Rust dependency graph (including build tools)',
  'and frontend production dependencies. License texts are copied from upstream packages or pinned upstream repositories.',
  'The unmodified source of Rust dependencies, including MPL-covered components, is available at the links below.',
  'Identical texts appear once and are referenced by number below.', '',
  'Assets and system components',
  'Deadlock, its characters and hero portraits belong to Valve Corporation and their respective owners.',
  'Portraits were obtained through Deadlock API (https://deadlock-api.com).',
  'DeadVox is an independent community project, not affiliated with or endorsed by Valve.',
  'Windows system fonts and the WebView2 runtime are supplied by the operating system/runtime provider;',
  'their font files and runtime installer are not bundled in this portable application.', '',
  'Dependency inventory', '',
];
for (const pkg of entries) {
  lines.push(`${pkg.ecosystem}: ${pkg.name} ${pkg.version}`, `License expression: ${pkg.license}`, `Source: ${pkg.url}`);
  for (const file of pkg.licenses) {
    lines.push(`  ${file.name}: text ${texts.get(file.text)}`);
    if (file.source) lines.push(`  License source: ${file.source}`);
  }
  lines.push('');
}
lines.push('License texts', '');
for (const [license, id] of texts) lines.push(`===== Text ${id} =====`, license, '');
const output = path.join(root, 'docs/THIRD-PARTY-NOTICES.txt');
fs.writeFileSync(output, lines.join('\n'));
console.log(`Generated notices: ${entries.length} dependencies, ${texts.size} distinct license texts.`);
