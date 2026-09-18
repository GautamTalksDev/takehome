import { codeToHtml } from 'shiki';

const THEME = {
  name: 'takehome',
  type: 'light',
  fg: 'var(--grey-1)',
  bg: 'var(--grey-4)',
  settings: [
    { settings: { foreground: 'var(--grey-1)' } },
    {
      scope: ['comment', 'punctuation.definition.comment'],
      settings: { foreground: 'var(--grey-2)' },
    },
    {
      scope: ['string', 'punctuation.definition.string'],
      settings: { foreground: 'var(--positive)' },
    },
    {
      scope: ['constant', 'constant.numeric', 'constant.language'],
      settings: { foreground: 'var(--accent)' },
    },
    {
      scope: ['keyword', 'storage', 'support.function'],
      settings: { foreground: 'var(--accent)' },
    },
    {
      scope: ['variable', 'support.type', 'entity.name'],
      settings: { foreground: 'var(--grey-1)' },
    },
  ],
};

export async function highlight(code, lang) {
  return codeToHtml(code, {
    lang,
    theme: THEME,
  });
}
