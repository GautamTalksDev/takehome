export const PAY_PERIODS = [
  { value: 1, label: '1 — annually' },
  { value: 2, label: '2 — semi-annually' },
  { value: 4, label: '4 — quarterly' },
  { value: 10, label: '10' },
  { value: 12, label: '12 — monthly' },
  { value: 13, label: '13 — four-weekly' },
  { value: 22, label: '22' },
  { value: 24, label: '24 — semi-monthly' },
  { value: 26, label: '26 — biweekly' },
  { value: 27, label: '27 — biweekly (27-period year)' },
  { value: 52, label: '52 — weekly' },
  { value: 53, label: '53 — weekly (53-week year)' },
  { value: 240, label: '240 — daily' },
  { value: 2000, label: '2000' },
];

export const CLAIM_CODES = [
  '0',
  '1',
  '2',
  '3',
  '4',
  '5',
  '6',
  '7',
  '8',
  '9',
  '10',
  'E',
];

export const PROVINCES = [
  { slug: 'ab', code: 'AB', name: 'Alberta', supported: true },
  { slug: 'bc', code: 'BC', name: 'British Columbia', supported: true },
  { slug: 'mb', code: 'MB', name: 'Manitoba', supported: true },
  { slug: 'nb', code: 'NB', name: 'New Brunswick', supported: true },
  { slug: 'nl', code: 'NL', name: 'Newfoundland and Labrador', supported: true },
  { slug: 'ns', code: 'NS', name: 'Nova Scotia', supported: true },
  { slug: 'nt', code: 'NT', name: 'Northwest Territories', supported: true },
  { slug: 'nu', code: 'NU', name: 'Nunavut', supported: true },
  { slug: 'on', code: 'ON', name: 'Ontario', supported: true },
  { slug: 'pe', code: 'PE', name: 'Prince Edward Island', supported: true },
  { slug: 'qc', code: 'QC', name: 'Quebec', supported: false },
  { slug: 'sk', code: 'SK', name: 'Saskatchewan', supported: true },
  { slug: 'yt', code: 'YT', name: 'Yukon', supported: true },
  {
    slug: 'outsidecanada',
    code: 'OutsideCanada',
    name: 'Outside Canada',
    supported: true,
  },
];

export function provinceBySlug(slug) {
  return PROVINCES.find((row) => row.slug === slug);
}
