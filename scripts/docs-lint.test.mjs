import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  findBannedWords,
  findEmDashes,
  findEllipsisPlaceholders,
  findPunctuationDoubleHyphens,
  hasAudienceBlock,
  hasLastReviewed,
  lintSource,
  stripCode,
  wordCount,
  sentences,
} from './docs-lint.mjs';

describe('docs-lint house style', () => {
  it('skips em dashes and banned words inside fenced and inline code', () => {
    const src = [
      '# Title',
      '',
      '**Who this is for:** Developers.',
      '**When you finish:** You can call the API.',
      '**Last reviewed:** 2026-09-20',
      '**Engine:** 0.1.0',
      '',
      'Prose is clean.',
      '',
      '```bash',
      'curl --dry-run https://example.com',
      'echo "— not scanned"',
      '```',
      '',
      'Use `just` only in code? No: `flag --just` is skipped.',
    ].join('\n');
    const prose = stripCode(src);
    assert.equal(findEmDashes(prose).length, 0);
    assert.equal(findBannedWords(prose).length, 0);
    const { errors } = lintSource('sample.md', src);
    assert.equal(errors.length, 0);
  });

  it('fails on em dashes and double hyphens in prose', () => {
    const src = 'Hello — world. Also word--word here.\n';
    assert.ok(findEmDashes(src).length >= 1);
    assert.ok(findPunctuationDoubleHyphens(src).length >= 1);
  });

  it('fails on banned words that blame the reader', () => {
    const hits = findBannedWords('This is simply easy and obviously trivial, of course.');
    assert.ok(hits.length >= 4);
  });

  it('warns on long sentences', () => {
    const long =
      'Word '.repeat(40).trim() + '.';
    assert.ok(wordCount(long) > 35);
    assert.equal(sentences(long).length, 1);
  });

  it('requires audience and last-reviewed blocks', () => {
    assert.equal(hasAudienceBlock('# Hi\n'), false);
    assert.equal(
      hasAudienceBlock(
        '**Who this is for:** A.\n**When you finish:** B.\n',
      ),
      true,
    );
    assert.equal(hasLastReviewed('**Last reviewed:** 2026-09-20\n**Engine:** 0.1.0\n'), true);
  });

  it('rejects incomplete runnable examples', () => {
    const src = '```javascript\nconst x = ...\n```\n';
    assert.equal(findEllipsisPlaceholders(src).length, 1);
  });

  it('mutation: a clean page fails when an em dash is inserted', () => {
    const clean = [
      '**Who this is for:** Readers.',
      '**When you finish:** You understand the rule.',
      '**Last reviewed:** 2026-09-20',
      '**Engine:** 0.1.0',
      '',
      'The engine resolves the rule set by date.',
    ].join('\n');
    assert.equal(lintSource('ok.md', clean).errors.length, 0);
    const broken = clean.replace('by date.', 'by date — always.');
    assert.ok(lintSource('bad.md', broken).errors.some((e) => /em\/en dash/.test(e)));
  });
});
