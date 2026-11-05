import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf-8'));

// Stammdaten & Identifikatoren
const version = pkg.version;
const productName = pkg.productName || 'FAIRmorph Workbench';
const identifier = pkg.identifier || 'org.fairmorph.workbench';
const isPrivate = Boolean(pkg.private);
const description = pkg.description;
const shortDescription = pkg.shortDescription || description.split(',')[0];
const license = pkg.license || 'MIT';
const keywords = Array.isArray(pkg.keywords) ? pkg.keywords : [];

// Autoren-Normalisierung
let authorName = 'FAIRmorph Team';
let authorEmail = '';
let authorUrl = '';

if (typeof pkg.author === 'string') {
  authorName = pkg.author.replace(/<.*>/, '').trim();
  const emailMatch = pkg.author.match(/<([^>]+)>/);
  if (emailMatch) authorEmail = emailMatch[1];
} else if (typeof pkg.author === 'object' && pkg.author !== null) {
  authorName = pkg.author.name || authorName;
  authorEmail = pkg.author.email || '';
  authorUrl = pkg.author.url || '';
}

const publisherFormatted = authorEmail ? `${authorName} <${authorEmail}>` : authorName;

// URLs
const homepage = pkg.homepage || '';
const docsUrl = pkg.documentation || '';
const bugsUrl = typeof pkg.bugs === 'string' ? pkg.bugs : (pkg.bugs?.url || '');
const rawRepo = typeof pkg.repository === 'string' ? pkg.repository : (pkg.repository?.url || '');
const repoUrl = rawRepo.replace(/\.git$/, '');

const currentYear = new Date().getFullYear();
const todayIso = new Date().toISOString().split('T')[0];

console.log(`🔄 Synchronisiere Metadaten (${productName} v${version})...`);

// 1. src-tauri/tauri.conf.json
const tauriPath = path.join(root, 'src-tauri/tauri.conf.json');
if (fs.existsSync(tauriPath)) {
  const tauri = JSON.parse(fs.readFileSync(tauriPath, 'utf-8'));

  tauri.productName = productName;
  tauri.identifier = identifier;

  if (Array.isArray(tauri.app?.windows) && tauri.app.windows.length > 0) {
    tauri.app.windows[0].title = productName;
  }

  if (tauri.bundle) {
    tauri.bundle.publisher = publisherFormatted;
    tauri.bundle.copyright = `Copyright © ${currentYear} ${authorName}`;
    tauri.bundle.license = license;
    tauri.bundle.shortDescription = shortDescription;
    tauri.bundle.longDescription = description;
    if (homepage) tauri.bundle.homepage = homepage;
  }

  fs.writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + '\n');
  console.log('  ✓ src-tauri/tauri.conf.json');
}

// 2. src-tauri/Cargo.toml
const cargoPath = path.join(root, 'src-tauri/Cargo.toml');
if (fs.existsSync(cargoPath)) {
  let cargo = fs.readFileSync(cargoPath, 'utf-8');

  cargo = cargo.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
  cargo = cargo.replace(/^description\s*=\s*"[^"]+"/m, `description = "${description}"`);
  cargo = cargo.replace(/^license\s*=\s*"[^"]+"/m, `license = "${license}"`);
  cargo = cargo.replace(/^authors\s*=\s*\[[^\]]*\]/m, `authors = ["${authorName}"]`);

  if (homepage) {
    cargo = cargo.replace(/^homepage\s*=\s*"[^"]+"/m, `homepage = "${homepage}"`);
  }
  if (repoUrl) {
    cargo = cargo.replace(/^repository\s*=\s*"[^"]+"/m, `repository = "${repoUrl}"`);
  }
  if (docsUrl) {
    if (cargo.includes('documentation =')) {
      cargo = cargo.replace(/^documentation\s*=\s*"[^"]+"/m, `documentation = "${docsUrl}"`);
    } else {
      cargo = cargo.replace(/^repository\s*=\s*"[^"]+"/m, `repository = "${repoUrl}"\ndocumentation = "${docsUrl}"`);
    }
  }

  // Keywords in Cargo.toml synchronisieren
  if (keywords.length > 0) {
    const formattedKeywords = JSON.stringify(keywords);
    cargo = cargo.replace(/^keywords\s*=\s*\[[^\]]*\]/m, `keywords = ${formattedKeywords}`);
  }

  fs.writeFileSync(cargoPath, cargo);
  console.log('  ✓ src-tauri/Cargo.toml');
}

// 3. citation.cff
const cffPath = path.join(root, 'CITATION.cff');
if (fs.existsSync(cffPath)) {
  let cff = fs.readFileSync(cffPath, 'utf-8');

  cff = cff.replace(/^title:\s*.+$/m, `title: "${productName}"`);
  cff = cff.replace(/^version:\s*.+$/m, `version: ${version}`);
  cff = cff.replace(/^abstract:\s*.+$/m, `abstract: "${description}"`);
  cff = cff.replace(/^license:\s*.+$/m, `license: ${license}`);
  cff = cff.replace(/^date-released:\s*.+$/m, `date-released: "${todayIso}"`);

  if (homepage) cff = cff.replace(/^url:\s*.+$/m, `url: "${homepage}"`);
  if (repoUrl) cff = cff.replace(/^repository-code:\s*.+$/m, `repository-code: "${repoUrl}"`);

  // Authors-Block dynamisch aufbauen & sauber einrücken
  const authorLines = ['authors:', `  - name: "${authorName}"`];
  if (authorEmail) authorLines.push(`    email: "${authorEmail}"`);
  if (authorUrl || homepage) authorLines.push(`    website: "${authorUrl || homepage}"`);
  const authorsYaml = authorLines.join('\n');

  // Ersetzt den gesamten bisherigen authors-Block bis zum nächsten Root-Key
  if (/^authors:/m.test(cff)) {
    cff = cff.replace(/^authors:(?:\r?\n(?![a-zA-Z0-9_#-]+:).*)*\r?\n?/m, `${authorsYaml}\n`);
  } else {
    cff += `\n${authorsYaml}\n`;
  }

  fs.writeFileSync(cffPath, cff);
  console.log('  ✓ citation.cff');
}

// 4. codemeta.json
const codemetaPath = path.join(root, 'codemeta.json');
if (fs.existsSync(codemetaPath)) {
  const codemeta = JSON.parse(fs.readFileSync(codemetaPath, 'utf-8'));

  // Identifikation & Version
  codemeta.identifier = identifier;
  codemeta.name = productName;
  codemeta.version = version;
  codemeta.description = description;
  codemeta.private = isPrivate;
  codemeta.license = `https://spdx.org/licenses/${license}`;
  codemeta.dateModified = todayIso;

  // URLs
  if (homepage) codemeta.url = homepage;
  if (repoUrl) codemeta.codeRepository = repoUrl;
  if (docsUrl) codemeta.documentation = docsUrl;
  if (bugsUrl) codemeta.issueTracker = bugsUrl;

  // Keywords
  if (keywords.length > 0) {
    codemeta.keywords = keywords;
  }

  // Autor (Organisation oder Person)
  if (Array.isArray(codemeta.author) && codemeta.author.length > 0) {
    codemeta.author[0].name = authorName;
    if (authorEmail) codemeta.author[0].email = authorEmail;
    if (authorUrl || homepage) codemeta.author[0].url = authorUrl || homepage;
  }

  fs.writeFileSync(codemetaPath, JSON.stringify(codemeta, null, 2) + '\n');
  console.log('  ✓ codemeta.json');
}

console.log('✨ Alle Metadaten synchronisiert.');