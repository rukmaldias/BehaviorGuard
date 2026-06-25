import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docs: [
    {
      type: 'doc',
      id: 'intro',
      label: 'Getting Started',
    },
    {
      type: 'doc',
      id: 'integration-guide',
      label: 'Integration Guide',
    },
    {
      type: 'doc',
      id: 'api-reference',
      label: 'API Reference',
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
  ],
};

export default sidebars;
