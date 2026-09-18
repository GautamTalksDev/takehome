(function () {
  var A = '#0b57d0', S = 'https://takehome.gautamkhosla.com', I = 'takehome-embed';
  function scripts() {
    var c = document.currentScript;
    if (c && c.getAttribute('data-province')) return [c];
    return [].slice.call(document.querySelectorAll('script[data-province][data-gross]')).filter(function (n) {
      return /embed\.js(\?|$)/.test(n.getAttribute('src') || '');
    });
  }
  function money(r) {
    var t = String(r).replace(/,/g, '').trim();
    if (!/^\d+(\.\d{1,2})?$/.test(t)) return null;
    return t.indexOf('.') < 0 ? t + '.00' : t.split('.')[1].length === 1 ? t + '0' : t;
  }
  function dir(s) {
    return new URL(s.src || s.getAttribute('src'), document.baseURI).href.replace(/[^/]+$/, '');
  }
  function mount(s) {
    var h = document.createElement('aside');
    h.setAttribute('data-testid', I);
    s.parentNode.insertBefore(h, s.nextSibling);
    var r = h.attachShadow({ mode: 'open' });
    var y = document.createElement('style');
    y.textContent = ':host{all:initial;display:block}div{font:1rem/1.4 system-ui,sans-serif;max-width:22rem;padding:.75rem 0;border:.0625rem solid #c8c8c8;border-width:.0625rem 0;color:#111}p{margin:.35rem 0}.n{font-size:1.5rem;font-weight:700;color:' + A + '}a{color:' + A + '}';
    var b = document.createElement('div');
    b.textContent = 'Calculating…';
    r.appendChild(y); r.appendChild(b);
    return h;
  }
  function box(h) { return h.shadowRoot.lastChild; }
  function fail(h, m) { box(h).textContent = m; h.setAttribute('data-ready', 'true'); }
  function draw(h, p, g, n, q) {
    var b = box(h); b.textContent = '';
    var l = document.createElement('p'), e = document.createElement('p'), f = document.createElement('p'), a = document.createElement('a');
    l.textContent = p + ' take-home on $' + g.replace(/\B(?=(\d{3})+(?=\.))/g, ',') + (q === 1 ? ' a year' : '');
    e.className = 'n'; e.setAttribute('data-testid', I + '-net'); e.textContent = n;
    f.appendChild(document.createTextNode('T4127 in this browser. '));
    a.href = S + '/'; a.textContent = 'Takehome'; f.appendChild(a);
    b.appendChild(l); b.appendChild(e); b.appendChild(f);
    h.setAttribute('data-ready', 'true');
  }
  function req(s) {
    var g = money(s.getAttribute('data-gross')), p = (s.getAttribute('data-province') || '').trim();
    var raw = s.getAttribute('data-period'), q = raw ? parseInt(raw, 10) : 1;
    if (!g || !p || !q) return null;
    return { as_of: s.getAttribute('data-as-of') || '2026-07-01', province: p, pay_period: q, gross_pay: g, federal_claim_code: 1, provincial_claim_code: 1 };
  }
  var pending = scripts();
  if (!pending.length) return;
  import(dir(pending[0]) + 'engine/takehome_wasm.js').then(function (m) {
    return m.default().then(function () { return m; });
  }).then(function (m) {
    pending.forEach(function (s) {
      var h = mount(s), r = req(s);
      if (!r) { fail(h, 'Needs data-province and data-gross.'); return; }
      var o = JSON.parse(m.calculate(JSON.stringify(r)));
      if (o.error) {
        fail(h, o.error.code === 'jurisdiction_not_supported' ? r.province + ' is not supported.' : o.error.message || o.error.code);
        return;
      }
      draw(h, r.province, r.gross_pay, o.employee.net_pay, r.pay_period);
    });
  }).catch(function () {
    pending.forEach(function (s) {
      var h = s.nextSibling;
      if (!h || h.getAttribute('data-testid') !== I) h = mount(s);
      fail(h, 'Engine failed to load.');
    });
  });
})();
