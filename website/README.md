# Perch Website

The official website for Perch, built with [Astro](https://astro.build).

## 🚀 Development

```bash
# Install dependencies
npm install

# Start dev server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

Screenshots live in the repository's root `screenshots/` directory, which the VHS
tapes write to. `npm run dev` and `npm run build` copy them into
`public/screenshots/` first (`npm run sync-screenshots`); don't commit that copy.

## 📁 Structure

```
website/
├── public/
│   ├── screenshots/    # Generated from ../screenshots (git-ignored)
│   ├── favicon.svg
│   ├── CNAME
│   └── robots.txt
├── src/
│   ├── layouts/
│   │   └── Layout.astro    # Base layout with nav/footer
│   └── pages/
│       ├── index.astro     # Homepage
│       ├── docs.astro      # Documentation
│       └── themes.astro    # Theme showcase
├── astro.config.mjs
└── package.json
```

## 🌐 Deployment

The website is automatically deployed to GitHub Pages when changes are pushed to the `main` branch.

URL: https://perch.ricardodantas.me
