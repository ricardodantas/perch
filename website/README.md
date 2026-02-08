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

## 📁 Structure

```
website/
├── public/
│   ├── screenshots/    # TUI screenshots
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
