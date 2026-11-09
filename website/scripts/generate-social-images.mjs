// Generates the social card and the Apple touch icon from SVG templates.
// Run manually after changing the branding or the timeline screenshot:
//   npm run generate:social-images
// Outputs (committed): public/og-image.png (1200×630), public/apple-touch-icon.png (180×180).
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';

const fromHere = (path) => fileURLToPath(new URL(path, import.meta.url));

// Palette from src/layouts/Layout.astro (:root custom properties).
const color = {
  bgPrimary: '#282a36',
  bgSecondary: '#1e1f29',
  bgTertiary: '#44475a',
  textPrimary: '#f8f8f2',
  textSecondary: '#6272a4',
  accentPurple: '#bd93f9',
};

// Same stack the site falls back to; it doesn't load Inter as a web font, so
// visitors mostly see the system sans. Rendering uses whatever fontconfig finds.
const fontSans = `Inter, 'Helvetica Neue', Helvetica, Arial, sans-serif`;
const fontMono = `'JetBrains Mono', 'JetBrainsMono Nerd Font', Menlo, Consolas, monospace`;

// The 🐦 logo as vector paths (Twemoji, CC-BY 4.0, see scripts/social/README.md).
const birdSvg = readFileSync(fromHere('./social/twemoji-1f426.svg'), 'utf8');
const birdInner = birdSvg.replace(/^<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '');
const bird = (x, y, size) =>
  `<svg x="${x}" y="${y}" width="${size}" height="${size}" viewBox="0 0 36 36">${birdInner}</svg>`;

async function appleTouchIcon() {
  const size = 180;
  const logo = 124; // 28px padding each side
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}">
    <rect width="${size}" height="${size}" fill="${color.bgPrimary}"/>
    ${bird((size - logo) / 2, (size - logo) / 2, logo)}
  </svg>`;
  // flatten() guarantees an opaque image: iOS fills transparency with black.
  await sharp(Buffer.from(svg))
    .flatten({ background: color.bgPrimary })
    .png({ compressionLevel: 9 })
    .toFile(fromHere('../public/apple-touch-icon.png'));
}

async function ogImage() {
  const width = 1200;
  const height = 630;

  // Header and first posts of the timeline pane, cut on a blank line so no text
  // row is sliced. The frame deliberately runs off the right edge.
  const crop = { left: 0, top: 0, width: 700, height: 540 };
  const shot = { x: 640, y: 68, width: 640, height: Math.round((crop.height * 640) / crop.width) };
  const screenshot = await sharp(fromHere('../../screenshots/01-timeline.png'))
    .extract(crop)
    .resize(shot.width, shot.height)
    .png()
    .toBuffer();

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
      width="${width}" height="${height}">
    <defs>
      <clipPath id="shot"><rect x="${shot.x}" y="${shot.y}" width="${shot.width}" height="${shot.height}" rx="14"/></clipPath>
    </defs>
    <rect width="${width}" height="${height}" fill="${color.bgPrimary}"/>

    ${bird(72, 96, 104)}
    <text x="68" y="318" font-family="${fontSans}" font-size="124" font-weight="800"
      fill="${color.textPrimary}" letter-spacing="-3">Perch</text>
    <text font-family="${fontSans}" font-size="40" font-weight="500" fill="${color.textPrimary}" fill-opacity="0.82">
      <tspan x="72" y="402">A beautiful terminal</tspan>
      <tspan x="72" y="452">social client</tspan>
    </text>
    <text x="72" y="514" font-family="${fontSans}" font-size="40" font-weight="700"
      fill="${color.accentPurple}">for Mastodon &amp; Bluesky</text>
    <text x="72" y="578" font-family="${fontMono}" font-size="24"
      fill="${color.textSecondary}">perch.ricardodantas.me</text>

    <rect x="${shot.x - 2}" y="${shot.y - 2}" width="${shot.width + 4}" height="${shot.height + 4}"
      rx="16" fill="${color.bgSecondary}" stroke="${color.bgTertiary}" stroke-width="2"/>
    <image x="${shot.x}" y="${shot.y}" width="${shot.width}" height="${shot.height}" clip-path="url(#shot)"
      xlink:href="data:image/png;base64,${screenshot.toString('base64')}"/>
  </svg>`;

  await sharp(Buffer.from(svg))
    .flatten({ background: color.bgPrimary })
    .png({ compressionLevel: 9, palette: true, quality: 90, effort: 10 })
    .toFile(fromHere('../public/og-image.png'));
}

await Promise.all([appleTouchIcon(), ogImage()]);
console.log('generate-social-images: wrote public/og-image.png and public/apple-touch-icon.png');
