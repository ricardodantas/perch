// Generates the social card and the Apple touch icon from SVG templates.
// Run manually after changing the branding or the timeline screenshot:
//   npm run generate:social-images
// Outputs (committed): public/og-image.png (1200×630), public/apple-touch-icon.png (180×180).
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { create as createFont } from 'fontkit';
import sharp from 'sharp';

const fromHere = (path) => fileURLToPath(new URL(path, import.meta.url));
const require = createRequire(import.meta.url);

// Palette from src/layouts/Layout.astro (:root custom properties).
const color = {
  bgPrimary: '#282a36',
  bgSecondary: '#1e1f29',
  bgTertiary: '#44475a',
  textPrimary: '#f8f8f2',
  textSecondary: '#6272a4',
  accentPurple: '#bd93f9',
};

// The site's fonts (Inter, JetBrains Mono) as static instances from the
// @fontsource devDependencies. fontkit shapes each string (GSUB + GPOS kerning)
// and the glyph outlines are inlined as SVG paths, so the output never depends
// on the fonts installed on the machine running this script.
const loadFont = (file) => createFont(readFileSync(require.resolve(file)));
const font = {
  inter500: loadFont('@fontsource/inter/files/inter-latin-500-normal.woff'),
  inter700: loadFont('@fontsource/inter/files/inter-latin-700-normal.woff'),
  inter800: loadFont('@fontsource/inter/files/inter-latin-800-normal.woff'),
  mono400: loadFont('@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff'),
};

// Renders `text` with its baseline starting at (x, y) and returns a <g> of glyph
// paths. `letterSpacing` (px) is added after each glyph, like CSS letter-spacing.
function textPath(face, text, { x, y, size, letterSpacing = 0, attrs }) {
  const scale = size / face.unitsPerEm;
  const run = face.layout(text);
  let pen = x;
  const paths = run.glyphs.map((glyph, i) => {
    const pos = run.positions[i];
    const gx = pen + pos.xOffset * scale;
    const gy = y - pos.yOffset * scale;
    pen += pos.xAdvance * scale + letterSpacing;
    const d = glyph.path.toSVG();
    // Font units are y-up; flip into SVG's y-down space at the glyph origin.
    return d ? `<path transform="matrix(${scale} 0 0 ${-scale} ${gx.toFixed(3)} ${gy.toFixed(3)})" d="${d}"/>` : '';
  });
  return `<g ${attrs}>${paths.join('')}</g>`;
}

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
    ${textPath(font.inter800, 'Perch', { x: 68, y: 318, size: 124, letterSpacing: -3, attrs: `fill="${color.textPrimary}"` })}
    ${textPath(font.inter500, 'A beautiful terminal', { x: 72, y: 402, size: 40, attrs: `fill="${color.textPrimary}" fill-opacity="0.82"` })}
    ${textPath(font.inter500, 'social client', { x: 72, y: 452, size: 40, attrs: `fill="${color.textPrimary}" fill-opacity="0.82"` })}
    ${textPath(font.inter700, 'for Mastodon & Bluesky', { x: 72, y: 514, size: 40, attrs: `fill="${color.accentPurple}"` })}
    ${textPath(font.mono400, 'perch.ricardodantas.me', { x: 72, y: 578, size: 24, attrs: `fill="${color.textSecondary}"` })}

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
