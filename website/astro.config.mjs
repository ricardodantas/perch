// @ts-check
import { defineConfig } from 'astro/config';

// https://astro.build/config
export default defineConfig({
  site: 'https://perch.ricardodantas.me',
  base: '/',
  build: {
    assets: 'assets'
  },
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
