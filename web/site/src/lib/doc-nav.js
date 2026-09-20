/**
 * Documentation navigation groups (item 48).
 */
export const DOC_NAV = [
  {
    label: 'Getting started',
    items: [
      { href: '/docs/quickstart/', title: 'Quickstart' },
      { href: '/docs/what-are-payroll-deductions/', title: 'What are payroll deductions?' },
      { href: '/docs/how-the-calculation-works/', title: 'How the calculation works' },
      { href: '/docs/offline-and-self-host/', title: 'Offline and self-host' },
    ],
  },
  {
    label: 'Concepts',
    items: [
      { href: '/docs/effective-dates/', title: 'Effective dates' },
      { href: '/docs/glossary/', title: 'Glossary' },
      { href: '/docs/request-fields/', title: 'Request fields' },
      { href: '/docs/response-fields/', title: 'Response fields' },
      { href: '/docs/errors/', title: 'Errors' },
      { href: '/docs/rate-limits-and-billing/', title: 'Rate limits and billing' },
      { href: '/docs/webhooks/', title: 'Webhooks' },
    ],
  },
  {
    label: 'API reference',
    items: [
      { href: '/docs/api-reference/', title: 'API reference' },
      { href: '/docs/quickstart/', title: 'First authenticated call' },
    ],
  },
  {
    label: 'Conformance and findings',
    items: [
      { href: '/docs/conformance/', title: 'Conformance' },
      { href: '/conformance/', title: 'Full disagreement list' },
      { href: '/docs/findings/', title: 'Findings index' },
      { href: '/docs/security/', title: 'Security' },
    ],
  },
  {
    label: 'Operations',
    items: [
      { href: '/docs/adr/', title: 'Architecture decisions' },
      { href: '/docs/CONFORMANCE-OPERATIONS/', title: 'PDOC operations' },
      { href: 'https://github.com/takehome-ca/takehome/blob/main/ARCHITECTURE.md', title: 'ARCHITECTURE.md' },
      { href: 'https://github.com/takehome-ca/takehome/blob/main/CONTRIBUTING.md', title: 'Contributing' },
      { href: 'https://github.com/takehome-ca/takehome/blob/main/SECURITY.md', title: 'Report a vulnerability' },
    ],
  },
];

export const DOC_PAGES = [
  'quickstart',
  'what-are-payroll-deductions',
  'how-the-calculation-works',
  'glossary',
  'api-reference',
  'request-fields',
  'response-fields',
  'effective-dates',
  'errors',
  'webhooks',
  'rate-limits-and-billing',
  'offline-and-self-host',
  'conformance',
  'security',
  'pricing',
  'jurisdictions',
  'year-projection',
  'CONFORMANCE-OPERATIONS',
  'SECRETS',
  'FIVE-MINUTE-TEST',
];
