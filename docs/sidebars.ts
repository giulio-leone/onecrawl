import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'getting-started',
    {
      type: 'category',
      label: 'Reference',
      items: [
        'cli-reference',
        'http-api',
        'mcp-tools',
      ],
    },
    {
      type: 'category',
      label: 'SDKs',
      items: [
        'sdk-nodejs',
        'sdk-python',
      ],
    },
    'architecture',
  ],
};

export default sidebars;
