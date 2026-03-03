import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'OneCrawl',
  tagline: 'High-performance browser automation engine in Rust',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  url: 'https://giulio-leone.github.io',
  baseUrl: '/onecrawl/',

  organizationName: 'giulio-leone',
  projectName: 'onecrawl',

  onBrokenLinks: 'throw',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/giulio-leone/onecrawl/tree/main/docs/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'OneCrawl',
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          href: 'https://github.com/giulio-leone/onecrawl',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Documentation',
          items: [
            { label: 'Getting Started', to: '/docs/getting-started' },
            { label: 'CLI Reference', to: '/docs/cli-reference' },
            { label: 'HTTP API', to: '/docs/http-api' },
            { label: 'MCP Tools', to: '/docs/mcp-tools' },
          ],
        },
        {
          title: 'SDKs',
          items: [
            { label: 'Node.js', to: '/docs/sdk-nodejs' },
            { label: 'Python', to: '/docs/sdk-python' },
          ],
        },
        {
          title: 'More',
          items: [
            { label: 'Architecture', to: '/docs/architecture' },
            { label: 'GitHub', href: 'https://github.com/giulio-leone/onecrawl' },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Giulio Leone. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'bash', 'toml', 'json'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
