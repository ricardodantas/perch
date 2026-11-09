// @ts-check
import { defineConfig, fontProviders } from 'astro/config';

// https://astro.build/config
export default defineConfig({
  site: 'https://perch.ricardodantas.me',
  base: '/',
  build: {
    assets: 'assets'
  },
  // Self-hosted variable fonts from the @fontsource-variable packages (pinned in
  // package-lock.json), so builds need no network and visitors make no
  // third-party font requests. One latin file per family covers every weight.
  fonts: [
    {
      provider: fontProviders.local(),
      name: 'Inter',
      cssVariable: '--font-inter',
      display: 'swap',
      fallbacks: ['-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
      options: {
        variants: [
          {
            src: ['@fontsource-variable/inter/files/inter-latin-wght-normal.woff2'],
            weight: '100 900',
            style: 'normal'
          }
        ]
      }
    },
    {
      provider: fontProviders.local(),
      name: 'JetBrains Mono',
      cssVariable: '--font-jetbrains-mono',
      display: 'swap',
      fallbacks: ['Fira Code', 'Consolas', 'monospace'],
      options: {
        variants: [
          {
            src: ['@fontsource-variable/jetbrains-mono/files/jetbrains-mono-latin-wght-normal.woff2'],
            weight: '100 800',
            style: 'normal'
          }
        ]
      }
    }
  ],
  vite: {
    build: {
      // Lightning CSS (Vite's default CSS minifier) otherwise emits media query
      // range syntax such as `(width<=768px)`, which Safari < 16.4 ignores and
      // which breaks the mobile nav there. Targeting late-2021 browsers lowers it
      // to `(max-width:768px)`.
      cssTarget: ['chrome93', 'edge93', 'firefox91', 'safari15', 'ios15']
    }
  }
});
