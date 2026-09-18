/**
 * Line-oriented unified diff. Small T4127 pages and CSV tables only.
 */

export function unifiedDiff(fromText, toText, fromFile, toFile) {
  const a = fromText.split('\n');
  const b = toText.split('\n');
  const ops = diffOps(a, b);
  const lines = [`--- ${fromFile}`, `+++ ${toFile}`];
  let hunk = [];
  const flush = () => {
    if (hunk.length === 0) {
      return;
    }
    lines.push('@@');
    lines.push(...hunk);
    hunk = [];
  };
  for (const op of ops) {
    if (op.type === 'equal') {
      if (hunk.length > 0) {
        hunk.push(` ${op.line}`);
        if (hunk.filter((row) => row.startsWith('-') || row.startsWith('+')).length > 0) {
          flush();
        } else {
          hunk = [];
        }
      }
      continue;
    }
    if (op.type === 'del') {
      hunk.push(`-${op.line}`);
    } else {
      hunk.push(`+${op.line}`);
    }
  }
  flush();
  if (lines.length === 2) {
    lines.push('@@');
  }
  return lines.join('\n');
}

function diffOps(a, b) {
  const m = a.length;
  const n = b.length;
  const dp = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0));
  for (let i = 1; i <= m; i += 1) {
    for (let j = 1; j <= n; j += 1) {
      dp[i][j] =
        a[i - 1] === b[j - 1]
          ? dp[i - 1][j - 1] + 1
          : Math.max(dp[i - 1][j], dp[i][j - 1]);
    }
  }
  const ops = [];
  let i = m;
  let j = n;
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && a[i - 1] === b[j - 1]) {
      ops.push({ type: 'equal', line: a[i - 1] });
      i -= 1;
      j -= 1;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      ops.push({ type: 'add', line: b[j - 1] });
      j -= 1;
    } else {
      ops.push({ type: 'del', line: a[i - 1] });
      i -= 1;
    }
  }
  ops.reverse();
  return ops;
}
