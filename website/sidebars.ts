import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docs: [
    {type: 'doc', id: 'intro', label: 'Getting Started'},
    {type: 'doc', id: 'concepts', label: 'Core Concepts'},
    {
      type: 'category',
      label: 'Integration',
      collapsed: false,
      items: [
        {type: 'doc', id: 'integration-guide', label: 'Integration Guide'},
        {type: 'doc', id: 'api-reference', label: 'API Reference'},
      ],
    },
    {
      type: 'category',
      label: 'Internals',
      collapsed: false,
      items: [
        {type: 'doc', id: 'architecture', label: 'Architecture'},
        {type: 'doc', id: 'threat-model', label: 'Threat Model'},
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: false,
      items: [
        {type: 'doc', id: 'glossary', label: 'Glossary'},
        {type: 'doc', id: 'faq', label: 'FAQ'},
        {type: 'doc', id: 'troubleshooting', label: 'Troubleshooting'},
      ],
    },
  ],
};

export default sidebars;
