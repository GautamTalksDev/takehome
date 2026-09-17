(function () {
  var A = '#0b57d0';
  var SITE = 'https://takehome.gautamkhosla.com';
  function scripts() {
    var cur = document.currentScript;
    if (cur && cur.getAttribute('data-province')) return [cur];
    return [].slice
      .call(document.querySelectorAll('script[data-province][data-gross]'))
      .filter(function (n) {
        return /embed\.js(\?|$)/.test(n.getAttribute('src') || '');
      });
  }
  function grossToken(raw) {
    var t = String(raw).replace(/,/g, '').trim();
    if (!/^\d+(\.\d{1,2})?$/.test(t)) return null;
    if (t.indexOf('.') < 0) return t + '.00';
    return t.split('.')[1].length === 1 ? t + '0' : t;
  }
  function dir(script) {
    return new URL(
      script.src || script.getAttribute('src'),
      document.baseURI,
    ).href.replace(/[^/]+$/, '');
  }
  function mount(script) {
    var root = document.createElement('aside');
    root.setAttribute('data-testid', 'takehome-embed');
    root.style.cssText =
      'font:1rem/1.4 system-ui,sans-serif;max-width:22rem;padding:.75rem 0;border:.0625rem solid #c8c8c8;border-width:.0625rem 0';
    root.textContent = 'Calculating…';
    script.parentNode.insertBefore(root, script.nextSibling);
    return root;
  }
  function fail(root, msg) {
    root.textContent = msg;
    root.setAttribute('data-ready', 'true');
  }
  function draw(root, province, gross, net, period) {
    root.textContent = '';
    var lead = document.createElement('p');
    lead.textContent =
      province +
      ' take-home on $' +
      gross.replace(/\B(?=(\d{3})+(?=\.))/g, ',') +
      (period === 1 ? ' a year' : '');
    var netEl = document.createElement('p');
    netEl.setAttribute('data-testid', 'takehome-embed-net');
    netEl.style.cssText =
      'margin:.35rem 0;font-size:1.5rem;font-weight:700;color:' + A;
    netEl.textContent = net;
    var foot = document.createElement('p');
    foot.style.fontSize = '.9rem';
    foot.appendChild(document.createTextNode('T4127 in this browser. '));
    var link = document.createElement('a');
    link.href = SITE + '/';
    link.textContent = 'Takehome';
    link.style.color = A;
    foot.appendChild(link);
    root.appendChild(lead);
    root.appendChild(netEl);
    root.appendChild(foot);
    root.setAttribute('data-ready', 'true');
  }
  function requestOf(script) {
    var gross = grossToken(script.getAttribute('data-gross'));
    var province = (script.getAttribute('data-province') || '').trim();
    var raw = script.getAttribute('data-period');
    var period = raw ? parseInt(raw, 10) : 1;
    if (!gross || !province || !period) return null;
    return {
      as_of: script.getAttribute('data-as-of') || '2026-07-01',
      province: province,
      pay_period: period,
      gross_pay: gross,
      federal_claim_code: 1,
      provincial_claim_code: 1,
    };
  }
  var pending = scripts();
  if (!pending.length) return;
  import(dir(pending[0]) + 'engine/takehome_wasm.js')
    .then(function (mod) {
      return mod.default().then(function () {
        return mod;
      });
    })
    .then(function (mod) {
      pending.forEach(function (script) {
        var root = mount(script);
        var request = requestOf(script);
        if (!request) {
          fail(root, 'Needs data-province and data-gross.');
          return;
        }
        var out = JSON.parse(mod.calculate(JSON.stringify(request)));
        if (out.error) {
          fail(
            root,
            out.error.code === 'jurisdiction_not_supported'
              ? request.province + ' is not supported.'
              : out.error.message || out.error.code,
          );
          return;
        }
        draw(
          root,
          request.province,
          request.gross_pay,
          out.employee.net_pay,
          request.pay_period,
        );
      });
    })
    .catch(function () {
      pending.forEach(function (script) {
        var root = script.nextSibling;
        if (!root || root.getAttribute('data-testid') !== 'takehome-embed') {
          root = mount(script);
        }
        fail(root, 'Engine failed to load.');
      });
    });
})();
