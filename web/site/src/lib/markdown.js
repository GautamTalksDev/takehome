function escapeHtml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function inline(text) {
  let out = '';
  let i = 0;
  while (i < text.length) {
    if (text.startsWith('**', i)) {
      const end = text.indexOf('**', i + 2);
      if (end !== -1) {
        out += `<strong>${inline(text.slice(i + 2, end))}</strong>`;
        i = end + 2;
        continue;
      }
    }
    if (text[i] === '`') {
      const end = text.indexOf('`', i + 1);
      if (end !== -1) {
        out += `<code>${escapeHtml(text.slice(i + 1, end))}</code>`;
        i = end + 1;
        continue;
      }
    }
    if (text[i] === '[') {
      const close = text.indexOf(']', i);
      if (close !== -1 && text[close + 1] === '(') {
        const endParen = text.indexOf(')', close + 2);
        if (endParen !== -1) {
          const label = text.slice(i + 1, close);
          const href = text.slice(close + 2, endParen);
          if (/^https?:\/\//.test(href)) {
            out += `<a href="${escapeHtml(href)}">${inline(label)}</a>`;
          } else {
            out += `<code>${escapeHtml(label)}</code>`;
          }
          i = endParen + 1;
          continue;
        }
      }
    }
    out += escapeHtml(text[i]);
    i += 1;
  }
  return out;
}

function isTableSeparator(line) {
  return /^\s*\|?[\s:|-]+\|[\s:|-]+/.test(line);
}

function splitRow(line) {
  const trimmed = line.trim().replace(/^\|/, '').replace(/\|$/, '');
  return trimmed.split('|').map((cell) => cell.trim());
}

export function renderMarkdown(source) {
  const lines = source.replace(/\r\n/g, '\n').split('\n');
  const html = [];
  let i = 0;
  let listType = null;

  const closeList = () => {
    if (listType) {
      html.push(`</${listType}>`);
      listType = null;
    }
  };

  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) {
      closeList();
      i += 1;
      continue;
    }
    if (line.startsWith('### ')) {
      closeList();
      html.push(`<h3>${inline(line.slice(4))}</h3>`);
      i += 1;
      continue;
    }
    if (line.startsWith('## ')) {
      closeList();
      html.push(`<h2>${inline(line.slice(3))}</h2>`);
      i += 1;
      continue;
    }
    if (line.startsWith('# ')) {
      closeList();
      html.push(`<h1>${inline(line.slice(2))}</h1>`);
      i += 1;
      continue;
    }
    if (line.startsWith('|') && i + 1 < lines.length && isTableSeparator(lines[i + 1])) {
      closeList();
      const headers = splitRow(line);
      html.push('<table><thead><tr>');
      for (const cell of headers) {
        html.push(`<th>${inline(cell)}</th>`);
      }
      html.push('</tr></thead><tbody>');
      i += 2;
      while (i < lines.length && lines[i].startsWith('|')) {
        const cells = splitRow(lines[i]);
        html.push('<tr>');
        for (const cell of cells) {
          html.push(`<td>${inline(cell)}</td>`);
        }
        html.push('</tr>');
        i += 1;
      }
      html.push('</tbody></table>');
      continue;
    }
    const ul = line.match(/^[-*] (.+)$/);
    if (ul) {
      if (listType !== 'ul') {
        closeList();
        html.push('<ul>');
        listType = 'ul';
      }
      html.push(`<li>${inline(ul[1])}</li>`);
      i += 1;
      continue;
    }
    closeList();
    html.push(`<p>${inline(line)}</p>`);
    i += 1;
  }
  closeList();
  return html.join('\n');
}
