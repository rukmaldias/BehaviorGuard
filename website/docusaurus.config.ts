import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

// Catppuccin Frappé — dark mode Prism theme
const catppuccinFrappe = {
  plain: {color: '#c6d0f5', backgroundColor: '#292c3c'},
  styles: [
    {types: ['comment', 'prolog', 'cdata'], style: {color: '#838ba7', fontStyle: 'italic' as const}},
    {types: ['punctuation'], style: {color: '#c6d0f5'}},
    {types: ['deleted', 'number', 'boolean', 'constant', 'symbol'], style: {color: '#ef9f76'}},
    {types: ['string', 'char', 'inserted'], style: {color: '#a6d189'}},
    {types: ['operator', 'url', 'variable'], style: {color: '#99d1db'}},
    {types: ['keyword', 'atrule'], style: {color: '#ca9ee6'}},
    {types: ['function', 'class-name'], style: {color: '#8caaee'}},
    {types: ['tag'], style: {color: '#e78284'}},
    {types: ['attr-name', 'selector'], style: {color: '#e5c890'}},
    {types: ['attr-value'], style: {color: '#a6d189'}},
    {types: ['property'], style: {color: '#babbf1'}},
    {types: ['regex', 'important'], style: {color: '#e5c890'}},
    {types: ['important', 'bold'], style: {fontWeight: 'bold' as const}},
    {types: ['italic'], style: {fontStyle: 'italic' as const}},
  ],
};

// Catppuccin Latte — light mode Prism theme
const catppuccinLatte = {
  plain: {color: '#4c4f69', backgroundColor: '#dce0e8'},
  styles: [
    {types: ['comment', 'prolog', 'cdata'], style: {color: '#8c8fa1', fontStyle: 'italic' as const}},
    {types: ['punctuation'], style: {color: '#4c4f69'}},
    {types: ['deleted', 'number', 'boolean', 'constant', 'symbol'], style: {color: '#fe640b'}},
    {types: ['string', 'char', 'inserted'], style: {color: '#40a02b'}},
    {types: ['operator', 'url', 'variable'], style: {color: '#04a5e5'}},
    {types: ['keyword', 'atrule'], style: {color: '#8839ef'}},
    {types: ['function', 'class-name'], style: {color: '#1e66f5'}},
    {types: ['tag'], style: {color: '#d20f39'}},
    {types: ['attr-name', 'selector'], style: {color: '#df8e1d'}},
    {types: ['attr-value'], style: {color: '#40a02b'}},
    {types: ['property'], style: {color: '#7287fd'}},
    {types: ['regex', 'important'], style: {color: '#df8e1d'}},
    {types: ['important', 'bold'], style: {fontWeight: 'bold' as const}},
    {types: ['italic'], style: {fontStyle: 'italic' as const}},
  ],
};

const config: Config = {
  title: 'BehaviorGuard',
  tagline: 'On-device behavioral biometrics for Android',
  favicon: 'img/favicon.svg',

  url: 'https://rukmaldias.github.io',
  baseUrl: '/BehaviorGuard/',

  organizationName: 'rukmaldias',
  projectName: 'BehaviorGuard',
  trailingSlash: false,

  onBrokenLinks: 'throw',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          path: '../docs',
          sidebarPath: './sidebars.ts',
          editUrl:
            'https://github.com/rukmaldias/BehaviorGuard/edit/main/',
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
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    image: 'img/logo.svg',
    navbar: {
      title: 'BehaviorGuard',
      logo: {
        alt: 'BehaviorGuard — shield with biometric waveform',
        src: 'img/logo.svg',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docs',
          position: 'left',
          label: 'Docs',
        },
        {
          href: 'https://github.com/rukmaldias/BehaviorGuard',
          position: 'right',
          className: 'header-github-link',
          'aria-label': 'GitHub repository',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {label: 'Getting Started', to: '/docs/intro'},
            {label: 'Integration Guide', to: '/docs/integration-guide'},
            {label: 'API Reference', to: '/docs/api-reference'},
          ],
        },
        {
          title: 'More',
          items: [
            {label: 'Architecture', to: '/docs/architecture'},
            {label: 'Threat Model', to: '/docs/threat-model'},
            {label: 'GitHub', href: 'https://github.com/rukmaldias/BehaviorGuard'},
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} BehaviorGuard. Built with Docusaurus.`,
    },
    prism: {
      theme: catppuccinLatte,
      darkTheme: catppuccinFrappe,
      additionalLanguages: ['rust', 'kotlin', 'bash', 'toml', 'groovy', 'python'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
