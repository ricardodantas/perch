# Social image sources

Inputs for `npm run generate:social-images` (`scripts/generate-social-images.mjs`),
which writes `public/og-image.png` and `public/apple-touch-icon.png`.

- `twemoji-1f426.svg`: the 🐦 bird used as Perch's logo across the site, taken
  from [Twemoji](https://github.com/jdecked/twemoji) v15.1.0
  (`assets/svg/1f426.svg`). Copyright Twitter, Inc. and other contributors,
  licensed under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).
  Vendored so the images render identically everywhere instead of depending on
  the machine's emoji font.

The card also crops `../../screenshots/01-timeline.png` from the repository root.

Text is set in the site's fonts, Inter (500/700/800) and JetBrains Mono (400),
from the `@fontsource/inter` and `@fontsource/jetbrains-mono` devDependencies
(SIL Open Font License 1.1). `fontkit` shapes each string (kerning included)
and the glyph outlines are inlined as SVG paths, so the result doesn't depend
on the fonts installed on the machine. These packages are only used by this
script, never by the site build.
