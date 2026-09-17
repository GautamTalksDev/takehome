import init, { calculate, listRuleSetVersions } from './takehome_wasm.js';

const OPTIONAL_MONEY = [
  'bonus',
  'ytd_cpp',
  'ytd_cpp2',
  'ytd_ei',
  'ytd_pensionable_earnings',
  'estimated_annual_expenses',
  'additional_tax_requested',
  'union_dues',
  'taxable_benefits',
];

function moneyToken(raw) {
  const token = String(raw).replace(/,/g, '').trim();
  if (!/^\d+(\.\d{1,2})?$/.test(token)) {
    return null;
  }
  if (!token.includes('.')) {
    return `${token}.00`;
  }
  if (token.split('.')[1].length === 1) {
    return `${token}0`;
  }
  return token;
}

function moneyToCents(token) {
  const [dollars, cents = '00'] = token.split('.');
  return Number.parseInt(dollars, 10) * 100 + Number.parseInt(cents, 10);
}

function centsToMoney(cents) {
  const abs = cents < 0 ? -cents : cents;
  const dollars = String(Math.trunc(abs / 100));
  const rest = String(abs % 100).padStart(2, '0');
  return cents < 0 ? `-${dollars}.${rest}` : `${dollars}.${rest}`;
}

function addMoney(a, b) {
  return centsToMoney(moneyToCents(a) + moneyToCents(b));
}

function claimWire(value) {
  return value === 'E' ? 'E' : Number.parseInt(value, 10);
}

function text(id, value) {
  const node = document.querySelector(`[data-testid="${id}"]`);
  if (node) {
    node.textContent = value == null ? '' : String(value);
  }
}

function editionLine(version, listing) {
  const row = listing.rule_set_versions.find((item) => item.version === version);
  if (!row) {
    return `Rule set ${version}.`;
  }
  const until = row.effective_to ? ` to ${row.effective_to}` : '';
  return `Rule set ${version}, effective ${row.effective_from}${until}.`;
}

function renderBreakdown(breakdown) {
  const rows = document.querySelectorAll('#breakdown-body tr');
  for (const row of rows) {
    const symbol = row.getAttribute('data-factor');
    const cell = row.querySelector('[data-role="value"]');
    if (cell) {
      cell.textContent = breakdown[symbol] ?? '';
    }
  }
}

function renderCitations(citations) {
  const list = document.querySelector('[data-testid="citations"]');
  if (!list) {
    return;
  }
  list.replaceChildren();
  const seen = new Set();
  for (const citation of citations) {
    const key = `${citation.factor}|${citation.source_url}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    const item = document.createElement('li');
    const link = document.createElement('a');
    link.href = citation.source_url;
    link.textContent = `${citation.factor}: ${citation.source_document}`;
    item.appendChild(link);
    list.appendChild(item);
  }
}

function employerCost(gross, employer) {
  return addMoney(
    addMoney(addMoney(gross, employer.cpp), employer.cpp2),
    employer.ei,
  );
}

function headlineNote(mode) {
  if (mode === 'employer') {
    return 'Headline is net pay; employer cost is the last line.';
  }
  if (mode === 'cpp') {
    return 'Employee CPP and CPP2 are the statutory pension lines; employer match is below.';
  }
  if (mode === 'ei') {
    return 'Employee EI is 1.63% of insurable earnings to the annual maximum; employer is 1.4×.';
  }
  if (mode === 'grossup') {
    return 'Gross was searched in integer cents so net meets the target.';
  }
  return '';
}

function renderSuccess(response, listing, gross, mode) {
  const employee = response.employee;
  const employer = response.employer;
  text('net-pay', employee.net_pay);
  text('federal-tax', employee.federal_tax);
  text('provincial-tax', employee.provincial_tax);
  text('cpp', employee.cpp);
  text('cpp2', employee.cpp2);
  text('ei', employee.ei);
  text('total-deductions', employee.total_deductions);
  text('employer-cpp', employer.cpp);
  text('employer-cpp2', employer.cpp2);
  text('employer-ei', employer.ei);
  text('employer-cost', employerCost(gross, employer));
  text('headline-note', headlineNote(mode));
  text('rule-set', editionLine(response.rule_set_version, listing));
  const prorated = (response.warnings || []).find(
    (warning) => warning.code === 'PRORATED_RECONCILIATION',
  );
  const warningNode = document.querySelector('[data-testid="prorated-warning"]');
  if (warningNode) {
    warningNode.textContent = prorated ? prorated.message : '';
    warningNode.classList.toggle('hidden', !prorated);
  }
  renderBreakdown(response.breakdown);
  renderCitations(response.citations || []);
}

function requestFromForm(form) {
  const gross = moneyToken(form.gross_pay.value);
  if (!gross) {
    return null;
  }
  const province = form.province ? form.province.value : form.dataset.province;
  const request = {
    as_of: form.as_of.value,
    province,
    pay_period: Number.parseInt(form.pay_period.value, 10),
    gross_pay: gross,
    federal_claim_code: claimWire(form.federal_claim_code.value),
    provincial_claim_code: claimWire(form.provincial_claim_code.value),
    cpp_months: 12,
  };
  for (const field of OPTIONAL_MONEY) {
    const node = form.elements.namedItem(field);
    if (!node || !node.value.trim()) {
      continue;
    }
    const token = moneyToken(node.value);
    if (token) {
      request[field] = token;
    }
  }
  return request;
}

function runCalculate(request) {
  return JSON.parse(calculate(JSON.stringify(request)));
}

function grossUp(form, listing) {
  const target = moneyToken(form.target_net.value);
  if (!target) {
    return null;
  }
  const targetCents = moneyToCents(target);
  let lo = 0;
  let hi = Math.max(targetCents * 4, 100);
  const seed = requestFromForm(form);
  if (!seed) {
    return null;
  }
  const fits = (cents) => {
    const trial = { ...seed, gross_pay: centsToMoney(cents) };
    const parsed = runCalculate(trial);
    if (parsed.error) {
      return { error: parsed.error, net: 0 };
    }
    return { response: parsed, net: moneyToCents(parsed.employee.net_pay), request: trial };
  };
  let high = fits(hi);
  while (!high.error && high.net < targetCents && hi < 100000000) {
    hi *= 2;
    high = fits(hi);
  }
  if (high.error) {
    return high;
  }
  while (lo < hi) {
    const mid = Math.trunc((lo + hi) / 2);
    const trial = fits(mid);
    if (trial.error) {
      return trial;
    }
    if (trial.net < targetCents) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  return fits(lo);
}

async function main() {
  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('/sw.js');
  }

  const form = document.getElementById('calculator');
  if (!form || form.dataset.supported !== 'true') {
    return;
  }

  form.addEventListener('submit', (event) => event.preventDefault());
  await init();
  const listing = JSON.parse(listRuleSetVersions());
  const mode = form.dataset.resultMode || 'net';

  const run = () => {
    if (mode === 'grossup' && form.target_net && form.target_net.value.trim()) {
      const found = grossUp(form, listing);
      if (!found) {
        return;
      }
      if (found.error) {
        text('rule-set', found.error.message);
        return;
      }
      form.gross_pay.value = found.request.gross_pay;
      renderSuccess(found.response, listing, found.request.gross_pay, mode);
      form.dataset.ready = 'true';
      return;
    }
    const request = requestFromForm(form);
    if (!request) {
      return;
    }
    const parsed = runCalculate(request);
    if (parsed.error) {
      text('rule-set', parsed.error.message);
      return;
    }
    renderSuccess(parsed, listing, request.gross_pay, mode);
    form.dataset.ready = 'true';
  };

  form.addEventListener('input', run);
  form.addEventListener('change', run);
  if (mode === 'grossup' && form.target_net && !form.target_net.value.trim()) {
    form.target_net.value = '1500.00';
  }
  run();
}

main();
