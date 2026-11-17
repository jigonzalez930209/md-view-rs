import { defineConfig } from 'vitepress'

const base = process.env.DOCS_BASE || '/md-view-rs/'
const repository = 'https://github.com/jigonzalez930209/md-view-rs'

export default defineConfig({
  lang: 'en-US',
  title: 'MD View',
  description:
    'A native Markdown, text, and code editor for Windows, macOS, and Linux, built with Rust and egui.',
  base,
  cleanUrls: true,
  lastUpdated: true,
  head: [
    ['link', { rel: 'icon', href: `${base}logo.png` }],
    ['meta', { name: 'theme-color', content: '#0d4f8b' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'MD View' }],
    [
      'meta',
      {
        property: 'og:description',
        content:
          'Native Markdown reading view, code editor, workspace explorer, search, themes, math, Mermaid, and rich media without a webview.',
      },
    ],
  ],
  themeConfig: {
    logo: '/logo.png',
    siteTitle: 'MD View',
    nav: [
      { text: 'Guide', link: '/guide/' },
      { text: 'Features', link: '/guide/features' },
      { text: 'Download', link: '/guide/install' },
      { text: 'GitHub', link: repository },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Get started',
          items: [
            { text: 'Overview', link: '/guide/' },
            { text: 'Install', link: '/guide/install' },
            { text: 'Workspace and tabs', link: '/guide/workspace' },
          ],
        },
        {
          text: 'Use MD View',
          items: [
            { text: 'Editing', link: '/guide/editor' },
            { text: 'Markdown and rich content', link: '/guide/markdown' },
            { text: 'Themes and reading', link: '/guide/themes' },
          ],
        },
        {
          text: 'Reference',
          items: [
            { text: 'Feature matrix', link: '/guide/features' },
            { text: 'Release 1.0.0-rc.3', link: '/guide/release' },
            { text: 'Project docs', link: `${repository}/tree/main/docs` },
          ],
        },
      ],
    },
    socialLinks: [{ icon: 'github', link: repository }],
    search: { provider: 'local' },
    editLink: {
      pattern: `${repository}/edit/main/website/:path`,
      text: 'Edit this page on GitHub',
    },
    footer: {
      message: 'Released under the MIT or Apache-2.0 license.',
      copyright: 'MD View contributors',
    },
    outline: { level: [2, 3] },
  },
  markdown: {
    lineNumbers: false,
  },
})
