// Copies the repository's root `screenshots/` directory into `public/screenshots/`.
// The root directory is the single source of truth (VHS tapes write there and the
// README links there); the copy under `public/` is generated and git-ignored.
import { cpSync, existsSync, rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const source = fileURLToPath(new URL('../../screenshots/', import.meta.url));
const target = fileURLToPath(new URL('../public/screenshots/', import.meta.url));

if (!existsSync(source)) {
  console.error(`sync-screenshots: source directory not found: ${source}`);
  process.exit(1);
}

// Start clean so screenshots deleted at the root don't linger in the site.
rmSync(target, { recursive: true, force: true });
cpSync(source, target, { recursive: true });
console.log('sync-screenshots: copied ../screenshots to public/screenshots');
