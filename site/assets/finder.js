/* Windows Finder landing: an explorable Finder window (views, navigation, Quick Look, menus). Needs fs.js. */
(function () {
  'use strict';
  var W = window.WFData; if (!W) return;
  var S = W.S, T = W.T, LANG = W.LANG, A = W.ASSETS;
  var LOC = LANG === 'fr' ? 'fr-FR' : 'en-US';
  var NF = new Intl.NumberFormat(LOC, { maximumFractionDigits: 1 });
  var TF = new Intl.DateTimeFormat(LOC, { hour: 'numeric', minute: '2-digit' });
  var DF = new Intl.DateTimeFormat(LOC, { day: 'numeric', month: 'short', year: 'numeric' });
  var MONEY = new Intl.NumberFormat(LOC, { style: 'currency', currency: 'EUR' });
  var TAGS = { red: '#f5524f', orange: '#f7822b', yellow: '#f0b82a', green: '#3fbf5a', blue: '#2f7df4', purple: '#a550a7', gray: '#8e8e93' };
  var TAG_ORDER = ['red', 'orange', 'yellow', 'green', 'blue', 'purple', 'gray'];

  /* ---------- small helpers ---------- */
  function esc(s) { return String(s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; }); }
  function h(tag, cls, html) { var e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; }
  function norm(s) { return String(s).toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, ''); }
  function ext(n) { if (n.ext) return n.ext; var i = n.name.lastIndexOf('.'); return i > 0 ? n.name.slice(i + 1).toLowerCase() : ''; }
  function stem(name) { var i = name.lastIndexOf('.'); return i > 0 ? name.slice(0, i) : name; }
  function fmtSize(b) {
    if (b == null) return '--';
    var u = LANG === 'fr' ? ['octets', 'Ko', 'Mo', 'Go'] : ['bytes', 'KB', 'MB', 'GB'], i = 0;
    while (b >= 1000 && i < 3) { b /= 1000; i++; }
    return (i === 0 ? Math.round(b) : NF.format(b >= 10 ? Math.round(b) : Math.round(b * 10) / 10)) + ' ' + u[i];
  }
  function fmtDate(t) {
    var d = new Date(t), n = new Date(), y = new Date(n.getTime() - 864e5);
    var same = function (a, b) { return a.toDateString() === b.toDateString(); };
    if (same(d, n)) return S.today + ' ' + TF.format(d);
    if (same(d, y)) return S.yesterday + ' ' + TF.format(d);
    return DF.format(d) + ' ' + S.at + ' ' + TF.format(d);
  }
  function kindKey(n) {
    if (n.dir) return n.special === 'disk' ? 'drive' : 'folder';
    if (n.app) return 'app';
    var e = ext(n); return S.kinds[e] ? e : (e === 'bashrc' || e === 'sh' ? 'sh' : 'txt');
  }
  function kindOf(n) { return S.kinds[kindKey(n)] || S.kinds.txt; }
  function qlType(n) {
    if (n.dir) return 'folder';
    if (n.art || n.img) return 'image';
    if (n.video) return 'video';
    if (n.pdf) return 'pdf';
    if (n.csv) return 'csv';
    if (n.text != null) return 'text';
    return 'generic';
  }
  function sizeOf(n) {
    if (!n.dir) return n.text != null ? Math.max(1, new Blob([n.text]).size) : (n.csv ? new Blob([n.csv.map(function (r) { return r.join(','); }).join('\n')]).size : n.size);
    return n.children.reduce(function (a, k) { return a + sizeOf(k); }, 0);
  }
  function isIn(n, anc) { for (var p = n; p; p = p.parent) if (p === anc) return true; return false; }
  function walk(n, fn) { fn(n); if (n.dir) n.children.forEach(function (k) { walk(k, fn); }); }

  /* ---------- icons ---------- */
  var P = {
    clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
    apps: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
    desktop: '<rect x="2.5" y="4" width="19" height="13" rx="2"/><path d="M8 21h8M12 17v4"/>',
    documents: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5M9 13h6M9 17h4"/>',
    downloads: '<circle cx="12" cy="12" r="9"/><path d="M12 7.5v9m-3.5-3.5L12 16.5l3.5-3.5"/>',
    pictures: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="2"/><path d="M21 16l-5-5-9 9"/>',
    music: '<path d="M9 18V5l11-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="17" cy="16" r="3"/>',
    videos: '<rect x="2.5" y="5" width="15" height="14" rx="2"/><path d="M17.5 10l4-2.5v9l-4-2.5"/>',
    onedrive: '<path d="M7 18a4.5 4.5 0 0 1-.6-9 6 6 0 0 1 11.5 1.5A3.8 3.8 0 0 1 17.5 18z"/>',
    disk: '<rect x="2.5" y="7" width="19" height="10" rx="2"/><path d="M6.5 13h.01M10 13h7"/>',
    wsl: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M13 15h4"/>',
    trash: '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>',
    home: '<path d="M3 11l9-7 9 7v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>',
    l: '<path d="M15 6l-6 6 6 6"/>', r: '<path d="M9 6l6 6-6 6"/>', down: '<path d="M6 9l6 6 6-6"/>',
    icons: '<rect x="4" y="4" width="6.5" height="6.5" rx="1.5"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1.5"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1.5"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.5"/>',
    list: '<path d="M9 6h11M9 12h11M9 18h11M4.5 6h.01M4.5 12h.01M4.5 18h.01"/>',
    columns: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/>',
    gallery: '<rect x="3" y="3.5" width="18" height="12" rx="2"/><path d="M4 20h3M10.5 20h3M17 20h3"/>',
    group: '<rect x="3" y="4" width="7" height="7" rx="1.5"/><rect x="14" y="4" width="7" height="7" rx="1.5"/><path d="M3 16h18M3 20h12"/>',
    share: '<path d="M12 3v12M8 7l4-4 4 4"/><path d="M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-7"/>',
    tag: '<path d="M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9z"/><circle cx="7.5" cy="7.5" r="1.5"/>',
    more: '<circle cx="12" cy="12" r="9"/><path d="M8 12h.01M12 12h.01M16 12h.01"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/>',
    x: '<path d="M6 6l12 12M18 6L6 18"/>', side: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>',
    play: '<path d="M7 4.5v15l12-7.5z" fill="currentColor" stroke="none"/>', pause: '<path d="M7 5h3.5v14H7zM13.5 5H17v14h-3.5z" fill="currentColor" stroke="none"/>',
    vol: '<path d="M4 9v6h4l5 4V5L8 9z"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12"/>', mute: '<path d="M4 9v6h4l5 4V5L8 9z"/><path d="M17 9l5 6M22 9l-5 6"/>',
    full: '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>', rotate: '<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/>'
  };
  function ico(k, cls) { return '<svg class="fi' + (cls ? ' ' + cls : '') + '" viewBox="0 0 24 24" aria-hidden="true">' + (P[k] || '') + '</svg>'; }

  var DOC_COLORS = { pdf: '#e5483e', csv: '#2ea043', txt: '#8a8f99', zip: '#7a7f8a', exe: '#2f7df4', mp3: '#e2468b', sh: '#4a4f5a', bashrc: '#4a4f5a', mp4: '#8a4bd8' };
  var GID = 0;
  function docSVG(e) {
    var g = 'dg' + (++GID);
    var c = DOC_COLORS[e] || '#8a8f99', label = (e || 'doc').toUpperCase().slice(0, 4);
    var lines = (e === 'txt' || e === 'sh' || e === 'bashrc' || e === 'pdf') ? '<g stroke="#c9ccd3" stroke-width="2.2" stroke-linecap="round"><path d="M19 20h20M19 26h26M19 32h24"/></g>' : '';
    var grid = e === 'csv' ? '<g stroke="#9fd3ae" stroke-width="1.6" fill="none"><path d="M18 18h28v20H18zM18 25h28M18 31.5h28M27 18v20M36.5 18v20"/></g>' : '';
    var zip = e === 'zip' ? '<path d="M30 6v4h4v4h-4v4h4v4h-4v4h4" stroke="#9aa0ab" stroke-width="2.5" fill="none"/>' : '';
    return '<svg class="doc" viewBox="0 0 64 64" aria-hidden="true"><defs><linearGradient id="' + g + '" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#f1f2f5"/></linearGradient></defs>' +
      '<path d="M15 4h24l13 13v40a3 3 0 0 1-3 3H15a3 3 0 0 1-3-3V7a3 3 0 0 1 3-3z" fill="url(#' + g + ')" stroke="#cdd1d8" stroke-width="1.2"/>' +
      '<path d="M39 4v10a3 3 0 0 0 3 3h10" fill="#e6e8ec" stroke="#cdd1d8" stroke-width="1.2"/>' + lines + grid + zip +
      '<rect x="15" y="43" width="34" height="11" rx="3" fill="' + c + '"/><text x="32" y="51.4" text-anchor="middle" font-family="Segoe UI,system-ui,sans-serif" font-size="8" font-weight="700" fill="#fff" letter-spacing=".4">' + label + '</text></svg>';
  }
  var APPS = {
    calc: ['#4b5563', '#1f2937', '<g fill="#fff"><rect x="18" y="16" width="28" height="9" rx="2" opacity=".9"/><circle cx="21" cy="33" r="3"/><circle cx="32" cy="33" r="3"/><circle cx="43" cy="33" r="3" fill="#f7822b"/><circle cx="21" cy="44" r="3"/><circle cx="32" cy="44" r="3"/><circle cx="43" cy="44" r="3" fill="#f7822b"/></g>'],
    note: ['#60a5fa', '#2563eb', '<rect x="18" y="14" width="28" height="36" rx="3" fill="#fff"/><g stroke="#93c5fd" stroke-width="2.4" stroke-linecap="round"><path d="M23 23h18M23 30h18M23 37h12"/></g>'],
    paint: ['#fbbf24', '#f97316', '<circle cx="32" cy="32" r="15" fill="#fff"/><circle cx="26" cy="27" r="3.2" fill="#ef4444"/><circle cx="35" cy="24" r="3.2" fill="#3b82f6"/><circle cx="39" cy="33" r="3.2" fill="#22c55e"/><circle cx="29" cy="38" r="4" fill="#f59e0b"/>'],
    term: ['#374151', '#030712', '<path d="M20 24l8 8-8 8" stroke="#a7f3d0" stroke-width="3.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/><path d="M32 41h12" stroke="#a7f3d0" stroke-width="3.5" stroke-linecap="round"/>'],
    gear: ['#9ca3af', '#4b5563', '<circle cx="32" cy="32" r="13" fill="none" stroke="#fff" stroke-width="5" stroke-dasharray="6 4.2"/><circle cx="32" cy="32" r="6" fill="#fff"/>']
  };
  function appIcon(k) {
    if (k === 'wf') return '<img class="appimg" src="' + A + 'logo-128.webp" alt="" draggable="false">';
    var a = APPS[k] || APPS.gear, g = 'ag' + (++GID);
    return '<svg class="app" viewBox="0 0 64 64" aria-hidden="true"><defs><linearGradient id="' + g + '" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="' + a[0] + '"/><stop offset="1" stop-color="' + a[1] + '"/></linearGradient></defs><rect x="6" y="6" width="52" height="52" rx="13" fill="url(#' + g + ')"/>' + a[2] + '</svg>';
  }
  var FOLDER_ICON = { documents: 'DocumentsFolderIcon', downloads: 'DownloadsFolder', desktop: 'DesktopFolderIcon', pictures: 'PicturesFolderIcon', music: 'MusicFolderIcon', videos: 'MovieFolderIcon' };
  function thumbSrc(n) { return n.img || (n.art ? W.artURL(n.art) : n.video ? W.artURL(n.video) : null); }
  /* Icon for a node. big = Quick Look/preview size (affects nothing but the class). */
  function iconHTML(n, big) {
    if (n.dir) {
      if (n.special === 'disk') return '<svg class="drive" viewBox="0 0 64 64" aria-hidden="true"><rect x="6" y="18" width="52" height="28" rx="6" fill="#d9dce2"/><rect x="6" y="18" width="52" height="20" rx="6" fill="#eef0f3"/><circle cx="48" cy="40" r="2.5" fill="#3fbf5a"/><path d="M14 40h20" stroke="#aeb3bc" stroke-width="2.5" stroke-linecap="round"/></svg>';
      if (n.special === 'trash') return '<svg class="drive" viewBox="0 0 64 64" aria-hidden="true"><path d="M16 18h32l-3 38H19z" fill="#e6e8ec" stroke="#b9bec7" stroke-width="2"/><path d="M12 16h40" stroke="#9aa0ab" stroke-width="4" stroke-linecap="round"/><path d="M26 26v22M32 26v22M38 26v22" stroke="#c4c8cf" stroke-width="2"/></svg>';
      return '<img class="fold" src="' + A + 'icons/' + (FOLDER_ICON[n.special] || 'GenericFolderIcon') + '.png" alt="" draggable="false">';
    }
    if (n.app) return appIcon(n.app);
    var src = thumbSrc(n);
    if (src) return '<span class="thumb' + (n.video ? ' vid' : '') + (big ? ' big' : '') + '"><img src="' + src + '" alt="" draggable="false"></span>';
    return docSVG(ext(n));
  }
  function tagDots(n) {
    if (!n.tags || !n.tags.length) return '';
    return '<span class="tdots">' + n.tags.map(function (t) { return '<i style="background:' + TAGS[t] + '"></i>'; }).join('') + '</span>';
  }

  window.WFUtil = { esc: esc, h: h, norm: norm, ext: ext, stem: stem, fmtSize: fmtSize, fmtDate: fmtDate, kindOf: kindOf, qlType: qlType, sizeOf: sizeOf, isIn: isIn, walk: walk, ico: ico, iconHTML: iconHTML, tagDots: tagDots, TAGS: TAGS, TAG_ORDER: TAG_ORDER, MONEY: MONEY, LOC: LOC };

  /* ---------- Quick Look content ---------- */
  function pdfPages(n) {
    var p = n.pdf, out = [];
    var para = function (k) {
      var en = ['The parties agree to the terms below. This document is a sample created for the Windows Finder demo and has no legal value.',
        'Payment is due within thirty days of the date shown above. Late payments may be subject to a fee agreed in writing by both parties.',
        'Either party may end this agreement with one month of written notice. Keys, badges and shared files are returned on the last day.',
        'Questions about this document can be sent to the address in the header. Please quote the reference number in every message.'];
      var fr = ['Les parties acceptent les conditions ci-dessous. Ce document est un exemple créé pour la démo de Windows Finder et n’a aucune valeur juridique.',
        'Le paiement est dû dans les trente jours suivant la date indiquée ci-dessus. Un retard peut entraîner des frais convenus par écrit.',
        'Chaque partie peut mettre fin à cet accord avec un préavis écrit d’un mois. Clés, badges et fichiers partagés sont rendus le dernier jour.',
        'Les questions sur ce document peuvent être envoyées à l’adresse de l’en-tête. Merci de rappeler la référence dans chaque message.'];
      return '<p>' + (LANG === 'fr' ? fr : en)[k % 4] + '</p>';
    };
    var lines = function (c) { var o = ''; for (var i = 0; i < c; i++) o += '<i style="width:' + (60 + ((i * 37) % 38)) + '%"></i>'; return '<div class="pl">' + o + '</div>'; };
    if (p.kind === 'invoice') {
      var rows = [[T('Design sprint (3 days)', 'Sprint de design (3 jours)'), .5], [T('Prototype and user tests', 'Prototype et tests utilisateurs'), .3], [T('Hosting, 12 months', 'Hébergement, 12 mois'), .2]];
      out.push('<div class="pg inv"><div class="inv-h"><div><b>Northwind Studio</b><small>12 rue des Lilas, 75011 Paris</small></div><div class="inv-no"><b>' + T('Invoice', 'Facture') + '</b><small>#' + p.no + '</small></div></div>' +
        '<div class="inv-to"><small>' + T('Billed to', 'Facturé à') + '</small><b>Alex Martin</b></div><table><thead><tr><th>' + T('Description', 'Description') + '</th><th>' + T('Amount', 'Montant') + '</th></tr></thead><tbody>' +
        rows.map(function (r) { return '<tr><td>' + r[0] + '</td><td>' + MONEY.format(p.amount * r[1]) + '</td></tr>'; }).join('') +
        '</tbody><tfoot><tr><td>' + T('Total due', 'Total à payer') + '</td><td>' + MONEY.format(p.amount) + '</td></tr></tfoot></table>' + para(1) + '<div class="inv-paid">' + T('Paid', 'Payée') + '</div></div>');
    } else if (p.kind === 'report') {
      out.push('<div class="pg"><h4>' + esc(p.title) + '</h4><small class="pg-sub">Northwind Studio</small><svg class="chart" viewBox="0 0 300 140" aria-hidden="true">' +
        [38, 52, 47, 66, 74, 90].map(function (v, i) { return '<rect x="' + (14 + i * 47) + '" y="' + (130 - v * 1.3) + '" width="30" height="' + (v * 1.3) + '" rx="4" fill="' + (i === 5 ? '#2f7df4' : '#b9d3fb') + '"/>'; }).join('') +
        '<path d="M8 130h284" stroke="#d0d4db"/></svg>' + para(0) + lines(5) + '</div>');
      out.push('<div class="pg">' + lines(4) + para(3) + lines(7) + '</div>');
    } else if (p.kind === 'form') {
      var box = function (l, v) { return '<div class="fbox"><small>' + l + '</small><b>' + v + '</b></div>'; };
      out.push('<div class="pg"><h4>' + esc(p.title) + '</h4><div class="fgrid">' + box(T('Name', 'Nom'), 'Alex Martin') + box(T('Tax number', 'N° fiscal'), '•••• ••• 482') +
        box(T('Salaries', 'Salaires'), MONEY.format(41200)) + box(T('Other income', 'Autres revenus'), MONEY.format(1850)) + box(T('Deductions', 'Déductions'), MONEY.format(522.89)) + box(T('Refund', 'Remboursement'), MONEY.format(312)) + '</div>' + para(0) + '</div>');
      out.push('<div class="pg">' + lines(6) + para(2) + lines(4) + '</div>');
    } else {
      out.push('<div class="pg"><h4>' + esc(p.title) + '</h4><small class="pg-sub">' + T('Reference', 'Référence') + ' WF-' + (n.id * 37 % 900 + 100) + '</small>' + para(0) + para(1) + lines(5) + '</div>');
      for (var i = 1; i < (p.pages || 2); i++) out.push('<div class="pg">' + para(i + 1) + lines(6) + para(i + 2) + lines(3) + '</div>');
    }
    return out;
  }
  function openWithLabel(t) {
    var app = { image: 'Photos', video: T('Media Player', 'Lecteur multimédia'), pdf: 'Microsoft Edge', csv: T('Notepad', 'Bloc-notes'), text: T('Notepad', 'Bloc-notes') }[t];
    return app ? S.openWithApp(app) : S.open;
  }
  function selectAll(node) { try { var r = document.createRange(); r.selectNodeContents(node); var s = getSelection(); s.removeAllRanges(); s.addRange(r); } catch (e) { } }

  /* Builds the body of a Quick Look panel. ctx: { onEdit(node) }. Returns { el, type, destroy, toggle }. */
  function qlContent(n, ctx) {
    ctx = ctx || {};
    var t = qlType(n), el = h('div', 'ql-c ql-' + t), timers = [], res = { el: el, type: t };
    res.destroy = function () { timers.forEach(function (x) { clearInterval(x); clearTimeout(x); }); };
    if (t === 'image') {
      el.innerHTML = '<img src="' + thumbSrc(n) + '" alt="' + esc(n.name) + '" draggable="false">';
    } else if (t === 'video') {
      var dur = n.dur || 20, pos = 0, playing = true, muted = false;
      var mmss = function (s) { s = Math.max(0, Math.round(s)); return Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0'); };
      el.innerHTML = '<div class="vid-frame playing">' + decodeURIComponent(thumbSrc(n).split(',')[1]) + '<span class="vid-sheen"></span></div>' +
        '<div class="vid-ctl"><button type="button" class="vc-play">' + ico('pause') + '</button><span class="vc-t">0:00</span>' +
        '<div class="vc-track" aria-hidden="true"><i></i></div><span class="vc-r">-' + mmss(dur) + '</span>' +
        '<button type="button" class="vc-mute" aria-label="' + T('Mute', 'Couper le son') + '">' + ico('vol') + '</button></div>';
      var frame = el.querySelector('.vid-frame'), bar = el.querySelector('.vc-track i'), tl = el.querySelector('.vc-t'), tr = el.querySelector('.vc-r'),
        pb = el.querySelector('.vc-play'), mb = el.querySelector('.vc-mute'), ctl = el.querySelector('.vid-ctl'), hideT;
      var paint = function () { bar.style.width = (pos / dur * 100) + '%'; tl.textContent = mmss(pos); tr.textContent = '-' + mmss(dur - pos); };
      var setPlay = function (p) { playing = p; frame.classList.toggle('playing', p); pb.innerHTML = ico(p ? 'pause' : 'play'); pb.setAttribute('aria-label', p ? T('Pause', 'Pause') : T('Play', 'Lecture')); wake(); };
      var wake = function () { ctl.classList.remove('hid'); clearTimeout(hideT); hideT = setTimeout(function () { if (playing) ctl.classList.add('hid'); }, 2500); timers.push(hideT); };
      pb.setAttribute('aria-label', T('Pause', 'Pause'));
      pb.onclick = function (e) { e.stopPropagation(); setPlay(!playing); };
      mb.onclick = function (e) { e.stopPropagation(); muted = !muted; mb.innerHTML = ico(muted ? 'mute' : 'vol'); };
      el.querySelector('.vc-track').onclick = function (e) { var r = this.getBoundingClientRect(); pos = Math.max(0, Math.min(dur, (e.clientX - r.left) / r.width * dur)); paint(); };
      frame.onclick = function () { setPlay(!playing); };
      timers.push(setInterval(function () { if (!playing) return; pos += .25; if (pos >= dur) pos = 0; paint(); }, 250));
      el.addEventListener('pointermove', wake); wake(); paint();
      res.toggle = function () { setPlay(!playing); };
    } else if (t === 'pdf') {
      var pages = pdfPages(n);
      el.innerHTML = '<div class="pdf-scroll" tabindex="0">' + pages.join('') + '</div><div class="pdf-ind">' + S.page(1, pages.length) + '</div>';
      var sc = el.querySelector('.pdf-scroll'), ind = el.querySelector('.pdf-ind');
      sc.addEventListener('scroll', function () {
        var pg = sc.querySelectorAll('.pg'), i = 0;
        for (var k = 0; k < pg.length; k++) if (pg[k].offsetTop - sc.scrollTop < sc.clientHeight / 2) i = k;
        ind.textContent = S.page(i + 1, pg.length);
      }, { passive: true });
    } else if (t === 'csv') {
      el.innerHTML = '<div class="csv-scroll"><table><tbody>' + n.csv.map(function (r, i) {
        return '<tr>' + r.map(function (c, j) { var tg = i ? 'td' : 'th'; return '<' + tg + ' contenteditable="true" spellcheck="false" data-r="' + i + '" data-c="' + j + '">' + esc(c) + '</' + tg + '>'; }).join('') + '</tr>';
      }).join('') + '</tbody></table></div>';
      el.addEventListener('input', function (e) {
        var c = e.target; if (!c.dataset || c.dataset.r == null) return;
        n.csv[+c.dataset.r][+c.dataset.c] = c.textContent; n.date = Date.now(); ctx.onEdit && ctx.onEdit(n);
      });
      el.addEventListener('keydown', function (e) {
        var c = e.target; if (!c.dataset || c.dataset.r == null) return;
        if (e.key !== 'Enter' && e.key !== 'Tab') return;
        e.preventDefault();
        var r = +c.dataset.r, k = +c.dataset.c, rows = n.csv.length, cols = n.csv[0].length;
        if (e.key === 'Enter') r = e.shiftKey ? Math.max(0, r - 1) : Math.min(rows - 1, r + 1);
        else { k += e.shiftKey ? -1 : 1; if (k >= cols) { k = 0; r = Math.min(rows - 1, r + 1); } if (k < 0) { k = cols - 1; r = Math.max(0, r - 1); } }
        var nx = el.querySelector('[data-r="' + r + '"][data-c="' + k + '"]'); if (nx) { nx.focus(); selectAll(nx); }
      });
    } else if (t === 'text') {
      el.innerHTML = '<textarea spellcheck="false" aria-label="' + esc(n.name) + '"></textarea>';
      var ta = el.querySelector('textarea'); ta.value = n.text;
      ta.addEventListener('input', function () { n.text = ta.value; n.date = Date.now(); ctx.onEdit && ctx.onEdit(n); });
    } else {
      var info = n.dir ? S.nItems(n.children.length) + ' · ' + fmtSize(sizeOf(n)) : kindOf(n) + ' · ' + fmtSize(sizeOf(n));
      el.innerHTML = '<div class="ql-gen"><div class="ql-gi">' + iconHTML(n, true) + '</div><div class="ql-gt"><b>' + esc(n.name) + '</b><span>' + info + '</span><span>' + S.modifiedS + (LANG === 'fr' ? ' : ' : ': ') + fmtDate(n.date) + '</span></div></div>';
    }
    return res;
  }
  window.WFUtil.qlContent = qlContent; window.WFUtil.openWithLabel = openWithLabel;

  /* ---------- the window ---------- */
  var SIDE = [
    ['favorites', [['recents', 'clock', S.recents], ['apps', 'apps', S.applications], ['desktop', 'desktop', S.desktop], ['docs', 'documents', S.documents],
      ['downloads', 'downloads', S.downloads], ['pictures', 'pictures', S.pictures], ['music', 'music', S.music], ['videos', 'videos', S.videos]]],
    ['locations', [['onedrive', 'onedrive', S.onedrive], ['c', 'disk', S.disk], ['wsl', 'wsl', S.wsl], ['trash', 'trash', S.trash]]],
    ['tagsH', TAG_ORDER.slice(0, 6).map(function (t) { return ['tag:' + t, null, S[t]]; })]
  ];
  var VIEWS = ['icons', 'list', 'columns', 'gallery'];
  var VLABEL = { icons: S.vIcons, list: S.vList, columns: S.vColumns, gallery: S.vGallery };

  function Finder(mount, opts) {
    this.o = opts || {};
    this.fs = W.buildFS();
    this.st = { loc: null, view: this.o.view || 'icons', sel: [], anchor: null, hist: [], hpos: -1, sort: { k: 'name', asc: true }, exp: {}, trail: [] };
    this.fns = []; this.undos = []; this.typed = ''; this.click = { id: 0, t: 0 };
    this.build(mount);
    this.go(this.resolve(this.o.start || 'docs'));
  }
  var F = Finder.prototype;

  F.on = function (fn) { this.fns.push(fn); };
  F.emit = function (e, d) { this.fns.forEach(function (fn) { try { fn(e, d); } catch (x) { } }); };

  F.build = function (mount) {
    var self = this, el = h('div', 'fw' + (this.o.compact ? ' compact' : ''));
    el.tabIndex = 0;
    el.setAttribute('role', 'application');
    el.setAttribute('aria-roledescription', T('file manager demo', 'démo de gestionnaire de fichiers'));
    el.setAttribute('aria-label', this.o.label || T('Windows Finder demo window', 'Fenêtre de démo de Windows Finder'));
    var side = '<div class="fw-lights" aria-hidden="true"><i></i><i></i><i></i></div>';
    SIDE.forEach(function (sec) {
      side += '<div class="fw-sec"><h6>' + S[sec[0]] + '</h6>';
      sec[1].forEach(function (it) {
        var icon = it[1] ? ico(it[1]) : '<span class="tg" style="background:' + TAGS[it[0].slice(4)] + '"></span>';
        side += '<button type="button" class="fw-si" data-loc="' + it[0] + '">' + icon + '<span>' + esc(it[2]) + '</span></button>';
      });
      side += '</div>';
    });
    el.innerHTML =
      '<aside class="fw-side">' + side + '</aside>' +
      '<div class="fw-main">' +
      '<div class="fw-bar">' +
      '<button type="button" class="fw-cap fw-sbtn" data-act="side" aria-label="' + S.sidebar + '">' + ico('side') + '</button>' +
      '<div class="fw-cap fw-nav"><button type="button" data-act="back" aria-label="' + S.back + '">' + ico('l') + '</button><button type="button" data-act="fwd" aria-label="' + S.fwd + '">' + ico('r') + '</button></div>' +
      '<div class="fw-title"></div>' +
      '<div class="fw-cap fw-seg" role="group" aria-label="' + S.views + '">' + VIEWS.map(function (v, i) {
        return '<button type="button" data-view="' + v + '" aria-label="' + VLABEL[v] + ' (' + (i + 1) + ')" title="' + VLABEL[v] + ' (' + (i + 1) + ')">' + ico(v) + '</button>';
      }).join('') + '</div>' +
      '<div class="fw-cap fw-x1"><button type="button" data-act="group" aria-label="' + S.group + '">' + ico('group') + ico('down', 'dd') + '</button></div>' +
      '<div class="fw-cap fw-x2"><button type="button" data-act="share" aria-label="' + S.share + '">' + ico('share') + '</button><button type="button" data-act="tags" aria-label="' + S.tagBtn + '">' + ico('tag') + '</button><button type="button" data-act="more" aria-label="' + S.more + '">' + ico('more') + '</button></div>' +
      '<div class="fw-cap fw-find"><button type="button" data-act="find" aria-label="' + S.search + '">' + ico('search') + '</button><input type="search" placeholder="' + S.searchPh + '" aria-label="' + S.search + '" autocomplete="off" spellcheck="false"><button type="button" class="fw-clr" data-act="clr" aria-label="' + S.close + '">' + ico('x') + '</button></div>' +
      '</div>' +
      '<div class="fw-scope" hidden></div>' +
      '<div class="fw-content"></div>' +
      '<div class="fw-path"></div>' +
      '<div class="fw-status"></div>' +
      '</div>' +
      '<div class="fw-toast" role="status" aria-live="polite"></div>';
    mount.classList.add('fw-host');
    mount.appendChild(el);
    this.el = el;
    this.$ = function (s) { return el.querySelector(s); };
    this.content = this.$('.fw-content');
    this.input = this.$('.fw-find input');
    this.bind();
  };

  /* ----- locations and listing ----- */
  F.resolve = function (k) {
    if (k === 'recents') return { v: 'recents' };
    if (k.indexOf('tag:') === 0) return { v: 'tag', tag: k.slice(4) };
    return { n: this.fs[k] };
  };
  F.locKey = function (l) {
    if (!l) return '';
    if (l.v === 'recents') return 'recents';
    if (l.v === 'tag') return 'tag:' + l.tag;
    if (l.v === 'search') return '';
    for (var k in this.fs) if (this.fs[k] === l.n) return k;
    return '';
  };
  F.title = function (l) {
    if (l.n) return l.n.name;
    if (l.v === 'recents') return S.recents;
    if (l.v === 'tag') return S[l.tag];
    return S.q(l.q);
  };
  F.alive = function (n) { return !isIn(n, this.fs.trash) && !isIn(n, this.fs.apps); };
  F.items = function (l) {
    var self = this, out = [], fs = this.fs;
    if (l.n) out = l.n.children.slice();
    else if (l.v === 'recents') {
      walk(fs.home, function (n) { if (!n.dir && self.alive(n)) out.push(n); });
      return out.sort(function (a, b) { return b.date - a.date; }).slice(0, 14);
    } else if (l.v === 'tag') walk(fs.root, function (n) { if (n !== fs.root && n.tags && n.tags.indexOf(l.tag) >= 0 && self.alive(n)) out.push(n); });
    else if (l.v === 'search') {
      var q = norm(l.q.trim()), K = { docs: ['pdf', 'csv', 'txt'], images: ['jpg', 'png'], media: ['mp4', 'mp3'] };
      walk(l.scope || fs.root, function (n) {
        if (n === fs.root || n === l.scope || !self.alive(n) || norm(n.name).indexOf(q) < 0) return;
        if (l.kind === 'folders' && !n.dir) return;
        if (K[l.kind] && (n.dir || K[l.kind].indexOf(ext(n)) < 0)) return;
        out.push(n);
      });
    }
    return this.sorted(out);
  };
  F.sorted = function (arr) {
    var s = this.st.sort, d = s.asc ? 1 : -1, self = this;
    return arr.sort(function (a, b) {
      if (a.dir !== b.dir) return a.dir ? -1 : 1;
      var r = 0;
      if (s.k === 'date') r = a.date - b.date;
      else if (s.k === 'size') r = (a.dir ? 0 : sizeOf(a)) - (b.dir ? 0 : sizeOf(b));
      else if (s.k === 'kind') r = kindOf(a).localeCompare(kindOf(b), LOC);
      if (!r) r = a.name.localeCompare(b.name, LOC, { numeric: true, sensitivity: 'base' });
      return r * d;
    });
  };
  /* Nodes in display order for keyboard navigation. */
  F.visible = function () {
    var st = this.st, self = this;
    if (st.view === 'list') {
      var out = [];
      (function add(arr, depth) { arr.forEach(function (n) { out.push(n); n._d = depth; if (n.dir && st.exp[n.id]) add(self.sorted(n.children.slice()), depth + 1); }); })(this.items(st.loc), 0);
      return out;
    }
    if (st.view === 'columns') {
      var t = st.trail, last = t[t.length - 1];
      return last && last.parent && t.length > 1 ? this.sorted(t[t.length - 2].children.slice()) : this.items(st.loc);
    }
    return this.items(st.loc);
  };
  F.byId = function (id) { var r = null; walk(this.fs.root, function (n) { if (n.id === id) r = n; }); return r; };
  F.find = function (path, from) {
    var n = from || this.st.loc.n;
    path.forEach(function (name) { n = n && n.dir ? n.children.filter(function (k) { return k.name === name; })[0] : null; });
    return n;
  };

  /* ----- navigation ----- */
  F.go = function (loc, o) {
    o = o || {};
    var st = this.st;
    if (!loc || (loc.n && !loc.n.dir)) return;
    if (!o.hist) {
      st.hist = st.hist.slice(0, st.hpos + 1);
      if (o.replace && st.hpos >= 0) st.hist[st.hpos] = loc; else { st.hist.push(loc); st.hpos = st.hist.length - 1; }
    }
    st.loc = loc; st.sel = []; st.anchor = null; st.trail = []; st.exp = {};
    if (loc.v !== 'search' && this.input.value) { this.input.value = ''; this.el.classList.remove('searching'); }
    this.closeQL(); this.closeInfo();
    this.el.classList.remove('side-open');
    this.render(true);
    if (!o.quiet && loc.n && st.hist.length > 1) this.emit('open-folder', loc.n);
  };
  F.back = function () { var s = this.st; if (s.hpos > 0) { s.hpos--; this.go(s.hist[s.hpos], { hist: true }); } };
  F.fwd = function () { var s = this.st; if (s.hpos < s.hist.length - 1) { s.hpos++; this.go(s.hist[s.hpos], { hist: true }); } };
  F.up = function () { var l = this.st.loc; if (l.n && l.n.parent && l.n.parent !== this.fs.root) this.go({ n: l.n.parent }); };
  F.setView = function (v, quiet) {
    if (VIEWS.indexOf(v) < 0 || v === this.st.view) return;
    var keep = this.st.sel.slice();
    this.st.view = v; this.st.trail = [];
    if (v === 'columns' && keep.length) this.st.trail = this.trailTo(keep[0]);
    this.render(true);
    if (keep.length && v !== 'columns') this.select(keep);
    if (v === 'gallery' && !this.st.sel.length) { var f = this.items(this.st.loc)[0]; if (f) this.select([f]); }
    if (!quiet) this.emit('view', v);
  };
  F.trailTo = function (n) {
    var base = this.st.loc.n, chain = [];
    if (!base) return [n];
    for (var p = n; p && p !== base; p = p.parent) chain.unshift(p);
    return chain[0] && chain[0].parent === base ? chain : [n];
  };

  /* ----- rendering ----- */
  F.render = function (fresh) {
    var st = this.st, el = this.el, key = this.locKey(st.loc);
    el.dataset.view = st.view;
    el.querySelectorAll('.fw-si').forEach(function (b) { var on = b.dataset.loc === key; b.classList.toggle('on', on); if (on) b.setAttribute('aria-current', 'true'); else b.removeAttribute('aria-current'); });
    el.querySelectorAll('.fw-seg button').forEach(function (b) { b.setAttribute('aria-pressed', b.dataset.view === st.view); });
    this.$('[data-act=back]').disabled = st.hpos <= 0;
    this.$('[data-act=fwd]').disabled = st.hpos >= st.hist.length - 1;
    this.$('.fw-title').textContent = this.title(st.loc);
    this.renderScope();
    var c = this.content, html;
    if (st.view === 'list') html = this.vList();
    else if (st.view === 'columns') html = this.vCols();
    else if (st.view === 'gallery') html = this.vGal();
    else html = this.vIcons();
    var top = fresh ? 0 : (c.firstChild && c.firstChild.scrollTop) || 0;
    c.innerHTML = html;
    if (fresh) { c.classList.remove('enter'); void c.offsetWidth; c.classList.add('enter'); }
    var sc = c.querySelector('.scroller'); if (sc && top) sc.scrollTop = top;
    if (st.view === 'columns') { var cols = c.querySelector('.fv-cols'); if (cols) cols.scrollLeft = cols.scrollWidth; }
    this.updateSel();
  };
  F.emptyMsg = function () { return '<div class="fv-empty">' + (this.st.loc.v === 'search' ? S.noResults : S.empty) + '</div>'; };
  F.attrs = function (n) { return ' data-id="' + n.id + '"' + (this.o.noDrag ? '' : ' draggable="true"') + ' role="option" aria-selected="false" aria-label="' + esc(n.name) + '"'; };
  F.vIcons = function () {
    var self = this, items = this.items(this.st.loc);
    if (!items.length) return this.emptyMsg();
    return '<div class="fv-icons scroller" role="listbox" aria-multiselectable="true">' + items.map(function (n) {
      return '<div class="it' + (n.dir ? ' dir' : '') + '"' + self.attrs(n) + '><div class="ib">' + iconHTML(n) + '</div><div class="nm"><span>' + esc(n.name) + '</span></div>' + tagDots(n) + '</div>';
    }).join('') + '</div>';
  };
  F.vList = function () {
    var self = this, st = this.st, rows = this.visible();
    var hd = function (k, l) { var on = st.sort.k === k; return '<button type="button" class="lh-' + k + (on ? ' on' : '') + '" data-sort="' + k + '">' + l + (on ? '<i class="' + (st.sort.asc ? 'up' : 'dn') + '"></i>' : '') + '</button>'; };
    var head = '<div class="lh">' + hd('name', S.name) + hd('date', S.modified) + hd('size', S.size) + hd('kind', S.kind) + '</div>';
    if (!rows.length) return head + this.emptyMsg();
    return head + '<div class="lb scroller" role="listbox" aria-multiselectable="true">' + rows.map(function (n, i) {
      var disc = n.dir ? '<button type="button" class="disc' + (st.exp[n.id] ? ' open' : '') + '" data-disc="' + n.id + '" tabindex="-1" aria-label="' + (st.exp[n.id] ? T('Collapse', 'Replier') : T('Expand', 'Déplier')) + '"></button>' : '<span class="disc-sp"></span>';
      return '<div class="row' + (i % 2 ? ' odd' : '') + (n.dir ? ' dir' : '') + '"' + self.attrs(n) + ' style="--d:' + (n._d || 0) + '"><span class="c-name">' + disc + '<span class="ri">' + iconHTML(n) + '</span><span class="nm">' + esc(n.name) + '</span>' + tagDots(n) + '</span>' +
        '<span class="c-date">' + fmtDate(n.date) + '</span><span class="c-size">' + (n.dir ? '--' : fmtSize(sizeOf(n))) + '</span><span class="c-kind">' + kindOf(n) + '</span></div>';
    }).join('') + '</div>';
  };
  F.vCols = function () {
    var self = this, st = this.st, cols = [this.items(st.loc)], t = st.trail;
    t.forEach(function (n) { if (n.dir) cols.push(self.sorted(n.children.slice())); });
    var html = '<div class="fv-cols scroller">' + cols.map(function (arr, i) {
      return '<div class="col" role="listbox" data-col="' + i + '">' + (arr.length ? arr.map(function (n) {
        var on = t[i] === n;
        return '<div class="crow' + (on ? (i === t.length - 1 ? ' sel' : ' path') : '') + (n.dir ? ' dir' : '') + '"' + self.attrs(n) + ' data-col="' + i + '"><span class="ri">' + iconHTML(n) + '</span><span class="nm">' + esc(n.name) + '</span>' + tagDots(n) + (n.dir ? '<span class="chev">' + ico('r') + '</span>' : '') + '</div>';
      }).join('') : '<div class="fv-empty sm">' + S.empty + '</div>') + '</div>';
    }).join('');
    var last = t[t.length - 1];
    if (last && !last.dir) html += '<div class="col prev">' + this.preview(last) + '</div>';
    return html + '</div>';
  };
  F.preview = function (n) {
    var rows = [[S.kind, kindOf(n)], [S.size, n.dir ? S.nItems(n.children.length) : fmtSize(sizeOf(n))], [S.created, fmtDate(n.cdate || n.date - 864e5 * 3)], [S.modifiedS, fmtDate(n.date)]];
    return '<div class="pv"><div class="pv-i">' + iconHTML(n, true) + '</div><b class="pv-n">' + esc(n.name) + '</b><span class="pv-k">' + kindOf(n) + ' · ' + (n.dir ? S.nItems(n.children.length) : fmtSize(sizeOf(n))) + '</span>' +
      (n.tags && n.tags.length ? '<span class="pv-t">' + n.tags.map(function (t) { return '<i style="background:' + TAGS[t] + '"></i>' + S[t]; }).join(' ') + '</span>' : '') +
      '<h6>' + S.infoH + '</h6><dl>' + rows.slice(2).map(function (r) { return '<dt>' + r[0] + '</dt><dd>' + r[1] + '</dd>'; }).join('') + '</dl></div>';
  };
  F.vGal = function () {
    var self = this, items = this.items(this.st.loc);
    if (!items.length) return this.emptyMsg();
    return '<div class="fv-gal"><div class="g-main"><div class="g-stage"></div><div class="g-strip scroller" role="listbox">' + items.map(function (n) {
      return '<div class="gt"' + self.attrs(n) + '>' + iconHTML(n) + '</div>';
    }).join('') + '</div></div><aside class="g-info"></aside></div>';
  };
  F.renderScope = function () {
    var l = this.st.loc, sc = this.$('.fw-scope');
    if (l.v !== 'search') { sc.hidden = true; return; }
    sc.hidden = false;
    var chip = function (k, lbl, on, attr) { return '<button type="button" ' + attr + '="' + k + '"' + (on ? ' class="on"' : '') + '>' + lbl + '</button>'; };
    var base = l.base && l.base.n ? l.base.n : null;
    sc.innerHTML = '<span>' + S.searching + '</span>' + chip('pc', S.thisPC, !l.scope, 'data-scope') + (base ? chip('here', S.q(base.name), l.scope === base, 'data-scope') : '') +
      '<span class="sp"></span>' + [['all', S.all], ['docs', S.kDocs], ['images', S.kImages], ['folders', S.kFolders], ['media', S.kMedia]].map(function (k) { return chip(k[0], k[1], l.kind === k[0], 'data-kind'); }).join('');
  };
  F.pathOf = function (n) {
    var chain = [];
    for (var p = n; p && p !== this.fs.root; p = p.parent) chain.unshift(p);
    return chain;
  };
  F.pathStr = function (n) {
    var c = this.pathOf(n), s = c.map(function (p) { return p.special === 'disk' ? 'C:' : p.name; });
    if (c[0] && c[0].special === 'wsl') return '\\\\wsl.localhost\\Ubuntu\\' + s.slice(1).join('\\');
    return s.join('\\').replace(/^C:\\?/, 'C:\\');
  };
  F.updateSel = function () {
    var st = this.st, self = this, ids = {};
    st.sel.forEach(function (n) { ids[n.id] = 1; });
    this.content.querySelectorAll('[data-id]').forEach(function (e) {
      if (e.classList.contains('crow')) return;
      var on = !!ids[e.dataset.id]; e.classList.toggle('sel', on); e.setAttribute('aria-selected', on);
    });
    if (st.view === 'gallery') {
      var n = st.sel[0] || null, stage = this.$('.g-stage'), info = this.$('.g-info');
      if (stage) stage.innerHTML = n ? '<div class="g-big' + (thumbSrc(n) ? ' img' : '') + '">' + iconHTML(n, true) + '</div>' : '';
      if (info) info.innerHTML = n ? this.preview(n) : '';
      near(this.content.querySelector('.gt.sel'));
    }
    // path bar shows the selection's path, or the folder's
    var target = st.sel.length === 1 ? st.sel[0] : st.loc.n, path = this.$('.fw-path');
    path.innerHTML = target ? this.pathOf(target).map(function (p, i, a) {
      return (i ? '<i class="sep">' + ico('r') + '</i>' : '') + '<button type="button" class="seg" data-pid="' + p.id + '"' + (p.dir ? '' : ' disabled') + '>' + '<span class="pi">' + iconHTML(p) + '</span>' + esc(p.special === 'disk' ? 'C:' : p.name) + '</button>';
    }).join('') : '<span class="seg muted">' + esc(this.title(st.loc)) + '</span>';
    path.scrollLeft = path.scrollWidth;
    var total = st.view === 'columns' ? this.visible().length : this.items(st.loc).length;
    var free = st.loc.n && (isIn(st.loc.n, this.fs.c) || st.loc.n === this.fs.home || isIn(st.loc.n, this.fs.home)) ? ', ' + S.free : '';
    this.$('.fw-status').textContent = (st.sel.length ? S.selOf(st.sel.length, total) : S.nItems(total)) + free;
    if (this.ql) this.refreshQL();
    if (this.info) this.renderInfo();
  };
  F.itemEl = function (n) { return n && this.content.querySelector((this.st.view === 'columns' ? '.crow' : '') + '[data-id="' + n.id + '"]'); };
  F.select = function (nodes, o) {
    o = o || {};
    var st = this.st;
    nodes = nodes.filter(Boolean);
    if (st.view === 'columns') {
      var n = nodes[nodes.length - 1];
      if (n) {
        st.trail = o.col != null ? st.trail.slice(0, o.col).concat([n]) : this.trailTo(n);
      } else st.trail = st.trail.slice(0, -1);
      st.sel = n ? [n] : [];
      this.render();
    } else {
      st.sel = nodes;
      this.updateSel();
    }
    st.anchor = nodes[0] || null;
    var e = this.itemEl(st.sel[st.sel.length - 1]); if (!o.noScroll) near(e);
  };

  /* Scrolls only the nearest inner scroller (never the page). */
  function near(e) {
    if (!e) return;
    var sc = e.parentElement;
    while (sc && sc !== document.body) {
      var cs = getComputedStyle(sc);
      if (/(auto|scroll)/.test(cs.overflowY + cs.overflowX)) break;
      sc = sc.parentElement;
    }
    if (!sc || sc === document.body) return;
    var r = e.getBoundingClientRect(), s = sc.getBoundingClientRect(), pad = 6;
    if (r.top < s.top + pad) sc.scrollTop -= s.top + pad - r.top;
    else if (r.bottom > s.bottom - pad) sc.scrollTop += r.bottom - s.bottom + pad;
    if (r.left < s.left) sc.scrollLeft -= s.left - r.left;
    else if (r.right > s.right) sc.scrollLeft += r.right - s.right;
  }
  function editable(t) { return t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable); }

  /* ----- events ----- */
  F.bind = function () {
    var self = this, el = this.el, st = this.st;
    el.addEventListener('pointerdown', function (e) {
      self.ptr = e.pointerType;
      self.lpFired = false;
      self.emit('interact');
      if (!editable(e.target) && !e.target.closest('.ql, .fw-info, button, input')) { try { el.focus({ preventScroll: true }); } catch (x) { } }
      if (e.pointerType === 'touch') {
        var it = e.target.closest('[data-id]');
        clearTimeout(self.lp);
        if (it) self.lp = setTimeout(function () { self.lpFired = true; self.ctxAt(it, e.clientX, e.clientY); }, 520);
      }
    });
    ['pointerup', 'pointercancel', 'pointermove'].forEach(function (t) { el.addEventListener(t, function (e) { if (t !== 'pointermove' || Math.abs(e.movementY) > 4) clearTimeout(self.lp); }); });
    el.addEventListener('keydown', function (e) { if (!e.target.closest('.ql')) self.emit('interact'); self.key(e); });

    el.addEventListener('click', function (e) {
      var t = e.target;
      if (self.lpFired) { self.lpFired = false; e.preventDefault(); return; }
      var b;
      if ((b = t.closest('[data-loc]'))) return self.go(self.resolve(b.dataset.loc));
      if ((b = t.closest('[data-view]')) && b.closest('.fw-seg')) return self.setView(b.dataset.view);
      if ((b = t.closest('[data-pid]'))) { var pn = self.byId(+b.dataset.pid); if (pn && pn.dir) self.go({ n: pn }); return; }
      if ((b = t.closest('[data-sort]'))) { var k = b.dataset.sort; if (st.sort.k === k) st.sort.asc = !st.sort.asc; else { st.sort.k = k; st.sort.asc = k === 'name'; } return self.render(); }
      if ((b = t.closest('[data-disc]'))) { var id = +b.dataset.disc; st.exp[id] = !st.exp[id]; return self.render(); }
      if ((b = t.closest('[data-scope]'))) { var l = st.loc; l.scope = b.dataset.scope === 'pc' ? null : l.base.n; return self.render(); }
      if ((b = t.closest('[data-kind]'))) { st.loc.kind = b.dataset.kind; return self.render(); }
      if ((b = t.closest('[data-act]'))) return self.act(b.dataset.act, b);
      if (t.closest('.ql, .fw-info, .fw-toast')) return;
      var it = t.closest('.fw-content [data-id]');
      if (it) return self.clickItem(self.byId(+it.dataset.id), e, it);
      if (t.closest('.fw-content') && st.view !== 'columns' && st.view !== 'gallery') self.select([]);
      if (t.closest('.fw-side') === null) el.classList.remove('side-open');
    });
    el.addEventListener('click', function () {
      setTimeout(function () { var a = document.activeElement; if (!self.menuOpen() && (!a || a === document.body || !el.contains(a))) { try { el.focus({ preventScroll: true }); } catch (x) { } } }, 0);
    });
    el.addEventListener('contextmenu', function (e) {
      if (editable(e.target) || e.target.closest('.ql')) return;
      var it = e.target.closest('.fw-content [data-id]');
      if (!e.target.closest('.fw-content')) return;
      e.preventDefault();
      self.emit('interact');
      if (it) self.ctxAt(it, e.clientX, e.clientY); else self.bgMenu(e.clientX, e.clientY);
    });
    this.input.addEventListener('input', function () { self.search(self.input.value); });
    this.input.addEventListener('keydown', function (e) {
      if (e.key === 'Escape') { e.stopPropagation(); self.endSearch(); el.focus({ preventScroll: true }); }
      if (e.key === 'Enter' || e.key === 'ArrowDown') { e.preventDefault(); var f = self.visible()[0]; if (f) { el.focus({ preventScroll: true }); self.select([f]); } }
    });
    this.input.addEventListener('blur', function () { if (!self.input.value) el.classList.remove('searching'); });
    this.bindDrag();
  };

  F.clickItem = function (n, e, it) {
    if (!n) return;
    var st = this.st, now = Date.now(), dbl = this.click.id === n.id && now - this.click.t < 450 && !e.shiftKey && !e.ctrlKey && !e.metaKey;
    var wasOnly = st.sel.length === 1 && st.sel[0] === n;
    this.click = { id: n.id, t: dbl ? 0 : now };
    if (dbl || (this.ptr === 'touch' && wasOnly && now - (this.selT || 0) > 350)) return this.open(n);
    if (this.renaming) this.commitRename();
    if (st.view === 'columns') { this.select([n], { col: +it.dataset.col }); }
    else if (e.ctrlKey || e.metaKey) {
      var i = st.sel.indexOf(n), s = st.sel.slice();
      if (i >= 0) s.splice(i, 1); else s.push(n);
      st.sel = s; st.anchor = n; this.updateSel();
    } else if (e.shiftKey && st.anchor) {
      var v = this.visible(), a = v.indexOf(st.anchor), b = v.indexOf(n);
      st.sel = v.slice(Math.min(a, b), Math.max(a, b) + 1); this.updateSel();
    } else if (!wasOnly) this.select([n], { noScroll: true });
    this.selT = now;
  };
  F.open = function (n) {
    if (!n) return;
    if (n.dir) {
      if (this.st.view === 'columns') { var it = this.itemEl(n); this.select([n], { col: it ? +it.dataset.col : null }); var first = this.sorted(n.children.slice())[0]; if (first) this.select([first], { col: this.st.trail.length }); this.emit('open-folder', n); }
      else this.go({ n: n });
      return;
    }
    if (n.app === 'wf' || ext(n) === 'exe') return this.toast(T('In the demo nothing runs. The real app is one click away.', 'Dans la démo, rien ne se lance. La vraie app est à un clic.'), T('Download', 'Télécharger'), function () { var d = document.getElementById('download'); if (d) d.scrollIntoView({ behavior: 'smooth' }); });
    if (n.app) return this.toast(T('Apps open from here, like in the Start menu.', 'Les apps se lancent d’ici, comme depuis le menu Démarrer.'));
    if (this.st.sel.indexOf(n) < 0) this.select([n]);
    this.openQL();
  };

  F.key = function (e) {
    var st = this.st, t = e.target, k = e.key, mod = e.ctrlKey || e.metaKey;
    if (this.menuOpen()) return;
    if (editable(t)) {
      if (this.ql && t.closest('.ql')) {
        if (k === 'Escape') { e.preventDefault(); t.blur(); this.closeQL(); this.el.focus({ preventScroll: true }); }
        else if (mod && (k === 's' || k === 'S')) { e.preventDefault(); this.saved(); }
      }
      return;
    }
    if (this.ql) {
      if (k === ' ' || k === 'Escape') { e.preventDefault(); return this.closeQL(); }
      if (mod && (k === 's' || k === 'S')) { e.preventDefault(); return this.saved(); }
      if (k === 'k' && this.ql.c.toggle) { e.preventDefault(); return this.ql.c.toggle(); }
    }
    if (this.info && k === 'Escape') { e.preventDefault(); return this.closeInfo(); }
    if (!mod && !e.altKey && /^[1-4]$/.test(k)) { e.preventDefault(); return this.setView(VIEWS[+k - 1]); }
    if (k === ' ') { e.preventDefault(); return this.ql ? this.closeQL() : this.openQL(); }
    if (k === 'Enter' && mod) { e.preventDefault(); return this.open(st.sel[0]); }
    if (k === 'Enter' || k === 'F2') { if (st.sel.length === 1) { e.preventDefault(); this.rename(st.sel[0]); } return; }
    if ((mod && k === 'ArrowDown') || (mod && (k === 'o' || k === 'O'))) { e.preventDefault(); return this.open(st.sel[0]); }
    if ((mod || e.altKey) && k === 'ArrowUp') { e.preventDefault(); return this.up(); }
    if ((e.altKey && k === 'ArrowLeft') || (mod && k === '[')) { e.preventDefault(); return this.back(); }
    if ((e.altKey && k === 'ArrowRight') || (mod && k === ']')) { e.preventDefault(); return this.fwd(); }
    if (k === 'Delete' || (mod && k === 'Backspace')) { e.preventDefault(); return this.trash(st.sel); }
    if (mod && (k === 'a' || k === 'A')) { e.preventDefault(); if (st.view !== 'columns') { st.sel = this.visible(); this.updateSel(); } return; }
    if (mod && (k === 'z' || k === 'Z')) { e.preventDefault(); return this.undo(); }
    if (mod && (k === 'i' || k === 'I')) { e.preventDefault(); return this.toggleInfo(); }
    if (mod && (k === 'f' || k === 'F')) { e.preventDefault(); return this.act('find'); }
    if (k === 'Escape') { if (st.loc.v === 'search') this.endSearch(); else if (st.sel.length && st.view !== 'columns') this.select([]); return; }
    if (k.indexOf('Arrow') === 0) { e.preventDefault(); return this.arrow(k, e.shiftKey); }
    if (k === 'Home' || k === 'End') { e.preventDefault(); var v = this.visible(); return this.select([k === 'Home' ? v[0] : v[v.length - 1]]); }
    if (k.length === 1 && !mod && !e.altKey && /\S/.test(k)) {
      var self = this; clearTimeout(this.typeT); this.typed += norm(k); this.typeT = setTimeout(function () { self.typed = ''; }, 1000);
      var m = this.visible().filter(function (n) { return norm(n.name).indexOf(self.typed) === 0; })[0];
      if (m) this.select([m]);
    }
  };
  F.cols = function () {
    var its = this.content.querySelectorAll('.fv-icons .it'); if (its.length < 2) return 1;
    var top = its[0].offsetTop, c = 0; for (var i = 0; i < its.length && its[i].offsetTop === top; i++) c++; return c;
  };
  F.arrow = function (k, shift) {
    var st = this.st, v = this.visible(), cur = st.sel[st.sel.length - 1], i = v.indexOf(cur), view = st.view;
    if (view === 'columns') {
      var t = st.trail, last = t[t.length - 1];
      if (k === 'ArrowRight') { if (last && last.dir && last.children.length) this.select([this.sorted(last.children.slice())[0]], { col: t.length }); return; }
      if (k === 'ArrowLeft') { if (t.length > 1) { st.trail = t.slice(0, -1); st.sel = [st.trail[st.trail.length - 1]]; this.render(); } return; }
      var j = i < 0 ? 0 : i + (k === 'ArrowDown' ? 1 : -1);
      if (v[j]) this.select([v[j]], { col: Math.max(0, t.length - 1) });
      if (this.ql) this.refreshQL();
      return;
    }
    if (view === 'list' && cur && (k === 'ArrowRight' || k === 'ArrowLeft')) {
      if (k === 'ArrowRight' && cur.dir && !st.exp[cur.id]) { st.exp[cur.id] = true; this.render(); }
      else if (k === 'ArrowLeft') { if (cur.dir && st.exp[cur.id]) { st.exp[cur.id] = false; this.render(); } else if (cur._d && cur.parent) this.select([cur.parent]); }
      return;
    }
    var step = view === 'icons' ? ({ ArrowLeft: -1, ArrowRight: 1, ArrowUp: -this.cols(), ArrowDown: this.cols() })[k]
      : view === 'gallery' ? ({ ArrowLeft: -1, ArrowRight: 1, ArrowUp: -1, ArrowDown: 1 })[k] : ({ ArrowUp: -1, ArrowDown: 1 })[k];
    if (!step) return;
    var n = i < 0 ? v[0] : v[Math.max(0, Math.min(v.length - 1, i + step))];
    if (!n) return;
    if (shift && st.anchor && view !== 'gallery') { var a = v.indexOf(st.anchor), b = v.indexOf(n); st.sel = v.slice(Math.min(a, b), Math.max(a, b) + 1); this.updateSel(); near(this.itemEl(n)); }
    else { var anc = st.anchor; this.select([n]); if (shift) st.anchor = anc; }
  };

  /* ----- toolbar ----- */
  F.act = function (a, b) {
    var st = this.st, self = this;
    if (a === 'back') return this.back();
    if (a === 'fwd') return this.fwd();
    if (a === 'side') return this.el.classList.toggle('side-open');
    if (a === 'find') { this.el.classList.add('searching'); this.input.focus(); return; }
    if (a === 'clr') { this.endSearch(); return; }
    var r = b ? b.getBoundingClientRect() : this.el.getBoundingClientRect();
    if (a === 'tags') return st.sel.length ? this.menu([{ tags: true }], r.left, r.bottom + 6) : this.toast(T('Select a file first, then pick a colour.', 'Sélectionnez d’abord un fichier, puis une couleur.'));
    if (a === 'more') return st.sel.length ? this.ctxMenu(r.left, r.bottom + 6) : this.bgMenu(r.left, r.bottom + 6);
    if (a === 'group') return this.menu(this.sortItems(), r.left, r.bottom + 6);
    if (a === 'share') return this.toast(T('Sharing uses the Windows share sheet in the app.', 'Dans l’app, le partage passe par la feuille de partage de Windows.'));
    if (a === 'ql-open') return this.toast(T('In the app, this opens the file in its default program.', 'Dans l’app, cela ouvre le fichier dans son programme par défaut.'));
    if (a === 'ql-x') return this.closeQL();
    if (a === 'info-x') return this.closeInfo();
  };
  F.sortItems = function () {
    var self = this, st = this.st;
    return [['name', S.name], ['kind', S.kind], ['date', S.modified], ['size', S.size]].map(function (s) {
      return { label: s[1], check: st.sort.k === s[0], fn: function () { st.sort = { k: s[0], asc: s[0] === 'name' }; self.render(); } };
    });
  };

  /* ----- search ----- */
  F.search = function (q) {
    var st = this.st;
    if (!q.trim()) return this.endSearch(true);
    this.el.classList.add('searching');
    if (st.loc.v === 'search') { st.loc.q = q; st.sel = []; this.render(); }
    else {
      var base = st.loc;
      this.go({ v: 'search', q: q, scope: null, kind: 'all', base: base.n ? base : null }, { quiet: true });
      this.input.value = q;
    }
    var n = this.items(st.loc).length;
    if (q.trim().length >= 3 && n) this.emit('search', q);
  };
  F.endSearch = function (keepFocus) {
    var st = this.st;
    this.input.value = '';
    if (!keepFocus) this.el.classList.remove('searching');
    if (st.loc.v === 'search') {
      var b = st.loc.base || this.resolve('docs');
      this.go(b, { replace: true, quiet: true });
      if (keepFocus) { this.el.classList.add('searching'); this.input.focus(); }
    }
  };

  /* ----- file operations ----- */
  F.pushUndo = function (u) { this.undos.push(u); if (this.undos.length > 30) this.undos.shift(); };
  F.detach = function (n) { var p = n.parent, i = p.children.indexOf(n); if (i >= 0) p.children.splice(i, 1); return { node: n, parent: p, index: i }; };
  F.attach = function (n, p, i) { if (n.parent) this.detach(n); n.parent = p; p.children.splice(i == null ? p.children.length : Math.min(i, p.children.length), 0, n); };
  F.unique = function (dir, name, self) {
    var taken = function (nm) { return dir.children.some(function (k) { return k !== self && norm(k.name) === norm(nm); }); };
    if (!taken(name)) return name;
    var s = stem(name), e = name.slice(s.length);
    for (var i = 2; ; i++) if (!taken(s + ' ' + i + e)) return s + ' ' + i + e;
  };
  F.label = function (nodes) { return nodes.length === 1 ? S.q(nodes[0].name) : S.nItems(nodes.length); };
  F.trash = function (nodes) {
    var self = this, fs = this.fs;
    nodes = (nodes || []).filter(function (n) { return n.parent && n.parent !== fs.root && !isIn(n, fs.trash) && n.parent !== fs.apps; });
    if (!nodes.length) return;
    var rec = nodes.map(function (n) { var r = self.detach(n); n.origin = r.parent; self.attach(n, fs.trash); return r; });
    this.pushUndo({ t: 'move', rec: rec });
    this.st.sel = []; this.render();
    this.toast(S.toastTrash(this.label(nodes)), S.undo, function () { self.undo(); });
    this.emit('trash');
  };
  F.putBack = function (nodes) {
    var self = this;
    var rec = nodes.map(function (n) { var r = self.detach(n); var to = n.origin && !isIn(n.origin, self.fs.trash) ? n.origin : self.fs.docs; n.name = self.unique(to, n.name, n); self.attach(n, to); return r; });
    this.pushUndo({ t: 'move', rec: rec });
    this.st.sel = []; this.render();
    this.toast(S.toastPut(this.label(nodes)), S.undo, function () { self.undo(); });
  };
  F.moveTo = function (nodes, dest) {
    var self = this;
    if (dest === this.fs.trash) return this.trash(nodes);
    nodes = nodes.filter(function (n) { return n !== dest && !isIn(dest, n) && n.parent !== dest; });
    if (!nodes.length || dest === this.fs.root || dest === this.fs.apps) return;
    var rec = nodes.map(function (n) { var r = self.detach(n); n.name = self.unique(dest, n.name, n); self.attach(n, dest); return r; });
    this.pushUndo({ t: 'move', rec: rec });
    this.st.sel = []; this.render();
    this.toast(S.toastMove(this.label(nodes), dest.name), S.undo, function () { self.undo(); });
    this.emit('move');
  };
  F.undo = function () {
    var u = this.undos.pop(), self = this;
    if (!u) return;
    if (u.t === 'move') u.rec.slice().reverse().forEach(function (r) { self.attach(r.node, r.parent, r.index); });
    else if (u.t === 'rename') u.node.name = u.old;
    else if (u.t === 'add') this.detach(u.node);
    else if (u.t === 'tags') u.rec.forEach(function (r) { r.node.tags = r.tags; });
    this.st.sel = []; this.render();
    this.toast(S.toastUndone);
  };
  F.newFolder = function () {
    var d = this.st.loc.n; if (!d || d === this.fs.root || d === this.fs.apps || d === this.fs.trash) return;
    var n = { id: 9000 + Math.floor(Math.random() * 1e6), name: this.unique(d, S.untitled), dir: true, children: [], date: Date.now() };
    this.attach(n, d);
    this.pushUndo({ t: 'add', node: n });
    if (this.st.view === 'columns') this.st.trail = [];
    this.render(); this.select([n]); this.rename(n);
  };
  F.duplicate = function (nodes) {
    var self = this, last;
    nodes.forEach(function (n) {
      if (n.dir || !n.parent) return;
      var c = {}; for (var k in n) c[k] = n[k];
      c.id = 9000 + Math.floor(Math.random() * 1e6); c.parent = null; c.tags = (n.tags || []).slice(); c.date = Date.now();
      if (n.csv) c.csv = n.csv.map(function (r) { return r.slice(); });
      c.name = self.unique(n.parent, stem(n.name) + S.copySuffix + n.name.slice(stem(n.name).length));
      self.attach(c, n.parent, n.parent.children.indexOf(n) + 1);
      self.pushUndo({ t: 'add', node: c }); last = c;
    });
    if (last) { this.render(); this.select([last]); }
  };
  F.setTag = function (nodes, tag) {
    if (!nodes.length) return;
    var on = !nodes.every(function (n) { return n.tags && n.tags.indexOf(tag) >= 0; });
    this.pushUndo({ t: 'tags', rec: nodes.map(function (n) { return { node: n, tags: (n.tags || []).slice() }; }) });
    nodes.forEach(function (n) {
      var t = (n.tags || []).filter(function (x) { return x !== tag; });
      if (on) t.push(tag);
      n.tags = t;
    });
    var keep = this.st.sel.slice(); this.render(); if (this.st.view !== 'columns') this.select(keep, { noScroll: true });
    if (on) this.emit('tag', tag);
  };
  F.copyPath = function (n) {
    var p = this.pathStr(n), self = this;
    try { navigator.clipboard.writeText(p).then(function () { self.toast(S.toastCopy + ' · ' + p); }, function () { self.toast(p); }); } catch (e) { this.toast(p); }
  };
  F.emptyTrash = function () {
    var tr = this.fs.trash; if (!tr.children.length) return;
    var rec = tr.children.slice().map(function (n, i) { return { node: n, parent: tr, index: i }; });
    tr.children = [];
    this.pushUndo({ t: 'move', rec: rec });
    this.render();
    var self = this; this.toast(S.toastEmpty, S.undo, function () { self.undo(); });
  };

  /* ----- rename ----- */
  F.rename = function (n) {
    var e = this.itemEl(n); if (!e || n.parent === this.fs.root || n.parent === this.fs.apps) return;
    var lbl = e.querySelector('.nm'); if (!lbl) return;
    if (this.st.view === 'gallery') return;
    var self = this, inp = h('input', 'fw-rn');
    inp.value = n.name; inp.setAttribute('aria-label', S.rename); inp.spellcheck = false;
    lbl.innerHTML = ''; lbl.appendChild(inp);
    e.draggable = false;
    this.renaming = { n: n, inp: inp };
    inp.focus({ preventScroll: true });
    inp.setSelectionRange(0, n.dir ? n.name.length : stem(n.name).length);
    inp.addEventListener('keydown', function (ev) {
      ev.stopPropagation();
      if (ev.key === 'Enter') { ev.preventDefault(); self.commitRename(); self.el.focus({ preventScroll: true }); }
      if (ev.key === 'Escape') { ev.preventDefault(); self.renaming = null; self.render(); self.select([n]); self.el.focus({ preventScroll: true }); }
    });
    inp.addEventListener('blur', function () { if (self.renaming && self.renaming.inp === inp) self.commitRename(); });
    inp.addEventListener('click', function (ev) { ev.stopPropagation(); });
  };
  F.commitRename = function () {
    var r = this.renaming; if (!r) return;
    this.renaming = null;
    var v = r.inp.value.trim().replace(/[\\/:*?"<>|]/g, '');
    if (v && v !== r.n.name) {
      this.pushUndo({ t: 'rename', node: r.n, old: r.n.name });
      r.n.name = this.unique(r.n.parent, v, r.n);
      r.n.date = Date.now();
      this.emit('rename', r.n);
    }
    var keep = this.st.sel.slice(); this.render(); if (this.st.view !== 'columns') this.select(keep, { noScroll: true });
  };

  /* ----- drag and drop (mouse) ----- */
  F.dropTarget = function (t) {
    var e = t.closest('.fw-content [data-id], .fw-si[data-loc], .fw-path [data-pid]');
    if (!e) return null;
    var n = e.dataset.id ? this.byId(+e.dataset.id) : e.dataset.pid ? this.byId(+e.dataset.pid) : (e.dataset.loc.indexOf(':') < 0 && e.dataset.loc !== 'recents' ? this.fs[e.dataset.loc] : null);
    return n && n.dir && this.drag && this.drag.indexOf(n) < 0 && !this.drag.some(function (d) { return isIn(n, d); }) ? { el: e, n: n } : null;
  };
  F.bindDrag = function () {
    var self = this, el = this.el, cur = null;
    var clear = function () { if (cur) cur.el.classList.remove('drop'); cur = null; };
    el.addEventListener('dragstart', function (e) {
      var it = e.target.closest && e.target.closest('.fw-content [data-id]');
      if (!it || self.renaming) { e.preventDefault(); return; }
      var n = self.byId(+it.dataset.id);
      if (self.st.sel.indexOf(n) < 0) self.select([n], { noScroll: true, col: it.dataset.col != null ? +it.dataset.col : null });
      self.drag = self.st.sel.slice();
      try { e.dataTransfer.setData('text/plain', self.drag.map(function (d) { return d.name; }).join('\n')); e.dataTransfer.effectAllowed = 'move'; } catch (x) { }
      el.classList.add('dragging');
      self.emit('interact');
    });
    el.addEventListener('dragover', function (e) {
      if (!self.drag) return;
      var d = self.dropTarget(e.target);
      if (!d) { clear(); return; }
      e.preventDefault();
      try { e.dataTransfer.dropEffect = 'move'; } catch (x) { }
      if (!cur || cur.el !== d.el) { clear(); cur = d; d.el.classList.add('drop'); }
    });
    el.addEventListener('drop', function (e) {
      if (!self.drag) return;
      e.preventDefault();
      var d = self.dropTarget(e.target), nodes = self.drag;
      clear(); self.drag = null; el.classList.remove('dragging');
      if (d) self.moveTo(nodes, d.n);
    });
    el.addEventListener('dragend', function () { clear(); self.drag = null; el.classList.remove('dragging'); });
  };

  /* ----- toast ----- */
  F.toast = function (msg, actLabel, fn) {
    var t = this.$('.fw-toast'), self = this;
    t.innerHTML = '<span>' + esc(msg) + '</span>' + (actLabel ? '<button type="button">' + esc(actLabel) + '</button>' : '');
    if (actLabel) t.querySelector('button').onclick = function (e) { e.stopPropagation(); t.classList.remove('show'); fn(); };
    t.classList.add('show');
    clearTimeout(this.toastT);
    this.toastT = setTimeout(function () { t.classList.remove('show'); }, 4200);
  };

  /* ----- menus (one shared floating menu for the page) ----- */
  var MENU = null;
  function closeMenu() { if (MENU) { MENU.root.remove(); if (MENU.ret && MENU.ret.isConnected) MENU.ret.focus({ preventScroll: true }); MENU = null; } }
  document.addEventListener('pointerdown', function (e) { if (MENU && !e.target.closest('.wf-menu')) closeMenu(); }, true);
  window.addEventListener('resize', closeMenu);
  window.addEventListener('scroll', function () { if (MENU && Date.now() - MENU.t > 300) closeMenu(); }, { passive: true });
  function buildMenu(items, cls) {
    var m = h('div', 'wf-menu' + (cls ? ' ' + cls : ''));
    m.setAttribute('role', 'menu'); m.tabIndex = -1;
    items.forEach(function (it) {
      if (it.sep) { m.appendChild(h('div', 'sep')); return; }
      if (it.tags) {
        var row = h('div', 'tagrow');
        TAG_ORDER.forEach(function (t) {
          var b = h('button', 'tg'); b.type = 'button'; b.setAttribute('role', 'menuitemcheckbox'); b.setAttribute('aria-label', S[t]); b.title = S[t];
          b.style.background = TAGS[t]; b.dataset.tag = t;
          if (it.on && it.on.indexOf(t) >= 0) b.classList.add('on');
          b.onclick = function () { closeMenu(); it.fn && it.fn(t); };
          row.appendChild(b);
        });
        m.appendChild(row); return;
      }
      if (it.head) { m.appendChild(h('div', 'mh', esc(it.head))); return; }
      var b = h('button', 'mi' + (it.check ? ' chk' : '') + (it.sub ? ' has-sub' : ''));
      b.type = 'button'; b.setAttribute('role', 'menuitem'); if (it.disabled) b.disabled = true;
      b.innerHTML = (it.icon ? ico(it.icon) : '<span class="ck">' + (it.check ? '✓' : '') + '</span>') + '<span class="ml">' + esc(it.label) + '</span>' + (it.kbd ? '<kbd>' + esc(it.kbd) + '</kbd>' : '') + (it.sub ? '<span class="sa">' + ico('r') + '</span>' : '');
      if (it.sub) {
        var open = function () { m.querySelectorAll('.wf-menu.sub').forEach(function (s) { s.remove(); }); var s = buildMenu(it.sub, 'sub' + (cls ? ' ' + cls : '')); m.appendChild(s); var r = b.getBoundingClientRect(), mr = m.getBoundingClientRect(); s.style.left = (r.right - mr.left - 4) + 'px'; s.style.top = (r.top - mr.top - 5) + 'px'; var sr = s.getBoundingClientRect(); if (sr.right > innerWidth - 8) s.style.left = (r.left - mr.left - sr.width + 4) + 'px'; if (sr.bottom > innerHeight - 8) s.style.top = (r.top - mr.top - (sr.bottom - innerHeight) - 12) + 'px'; return s; };
        b.addEventListener('pointerenter', function (e) { if (e.pointerType === 'mouse') open(); });
        b.onclick = function (e) { e.stopPropagation(); var s = open(); var f = s.querySelector('button'); if (f) f.focus(); };
        b._open = open;
      } else b.onclick = function () { closeMenu(); it.fn && it.fn(); };
      if (!it.sub) b.addEventListener('pointerenter', function () { m.querySelectorAll(':scope > .wf-menu.sub').forEach(function (s) { s.remove(); }); });
      m.appendChild(b);
    });
    m.addEventListener('keydown', function (e) {
      var own = Array.prototype.filter.call(m.children, function (c) { return c.tagName === 'BUTTON' && !c.disabled; });
      var tags = m.querySelectorAll(':scope > .tagrow button');
      var all = [];
      Array.prototype.forEach.call(m.children, function (c) { if (c.classList.contains('tagrow')) Array.prototype.forEach.call(c.children, function (x) { all.push(x); }); else if (own.indexOf(c) >= 0) all.push(c); });
      var i = all.indexOf(document.activeElement);
      if (i < 0 && e.key === 'ArrowUp') i = 0;
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); e.stopPropagation(); var n = all[(i + (e.key === 'ArrowDown' ? 1 : -1) + all.length) % all.length]; if (n) n.focus(); }
      else if (e.key === 'ArrowRight' && document.activeElement._open) { e.preventDefault(); e.stopPropagation(); var s = document.activeElement._open(); var f = s.querySelector('button'); if (f) f.focus(); }
      else if (e.key === 'ArrowLeft' && m.classList.contains('sub')) { e.preventDefault(); e.stopPropagation(); var p = m.parentElement; m.remove(); var pb = p.querySelector('.has-sub'); if (pb) pb.focus(); }
      else if ((e.key === 'Enter' || e.key === ' ') && document.activeElement && document.activeElement.tagName === 'BUTTON' && m.contains(document.activeElement)) { e.preventDefault(); e.stopPropagation(); document.activeElement.click(); }
      else if (e.key === 'Escape' || e.key === 'Tab') { e.preventDefault(); e.stopPropagation(); closeMenu(); }
      void tags;
    });
    return m;
  }
  F.menu = function (items, x, y, cls) {
    closeMenu();
    var m = buildMenu(items, cls), ret = document.activeElement;
    document.body.appendChild(m);
    var r = m.getBoundingClientRect();
    m.style.left = Math.max(8, Math.min(x, innerWidth - r.width - 8)) + 'px';
    m.style.top = Math.max(8, Math.min(y, innerHeight - r.height - 8)) + 'px';
    MENU = { root: m, ret: ret && ret.closest && ret.closest('.fw') ? ret : this.el, t: Date.now() };
    try { m.focus({ preventScroll: true }); } catch (x) { }
  };
  F.menuOpen = function () { return !!MENU; };
  F.ctxAt = function (it, x, y) {
    var n = this.byId(+it.dataset.id);
    if (this.st.sel.indexOf(n) < 0) this.select([n], { noScroll: true, col: it.dataset.col != null ? +it.dataset.col : null });
    this.ctxMenu(x, y);
  };
  F.ctxMenu = function (x, y) {
    var self = this, sel = this.st.sel.slice(), n = sel[0], one = sel.length === 1, fs = this.fs;
    if (!n) return;
    if (isIn(n, fs.trash) && n !== fs.trash) return this.menu([
      { label: S.putBack, icon: 'l', fn: function () { self.putBack(sel); } }, { sep: 1 },
      { label: S.ql, kbd: T('Space', 'Espace'), fn: function () { self.openQL(); } }, { label: S.info, kbd: 'Ctrl I', fn: function () { self.toggleInfo(true); } }
    ], x, y);
    var items = [
      { label: S.open, kbd: 'Ctrl O', fn: function () { self.open(n); } },
      { label: S.ql, kbd: T('Space', 'Espace'), fn: function () { self.openQL(); } },
      { sep: 1 },
      { label: S.info, kbd: 'Ctrl I', fn: function () { self.toggleInfo(true); } }
    ];
    if (one && n.parent !== fs.apps) items.push({ label: S.rename, kbd: T('Enter', 'Entrée'), fn: function () { self.rename(n); } });
    if (sel.some(function (k) { return !k.dir && !k.app; })) items.push({ label: S.duplicate, fn: function () { self.duplicate(sel); } });
    if (one) items.push({ label: S.copyPath, fn: function () { self.copyPath(n); } });
    items.push({ sep: 1 }, { tags: true, on: n.tags || [], fn: function (t) { self.setTag(sel, t); } }, { sep: 1 });
    if (n.parent !== fs.apps) items.push({ label: S.toTrash, kbd: T('Delete', 'Suppr'), fn: function () { self.trash(sel); } }, { sep: 1 });
    if (n.dir) items.push({ label: S.openTerm, fn: function () { self.toast(T('Opens Windows Terminal in ', 'Ouvre Windows Terminal dans ') + self.pathStr(n)); } });
    items.push({ label: S.moreOpts, fn: function () { self.winMenu(x, y); } });
    this.menu(items, x, y);
  };
  F.bgMenu = function (x, y) {
    var self = this, st = this.st, d = st.loc.n, fs = this.fs, items = [];
    if (d === fs.trash) items.push({ label: S.emptyTrash, disabled: !d.children.length, fn: function () { self.emptyTrash(); } }, { sep: 1 });
    else if (d && d !== fs.apps && d !== fs.root) items.push({ label: S.newFolder, fn: function () { self.newFolder(); } }, { sep: 1 });
    if (d) items.push({ label: S.info, kbd: 'Ctrl I', fn: function () { self.st.sel = []; self.toggleInfo(true); } });
    items.push({ label: S.viewAs, sub: VIEWS.map(function (v, i) { return { label: VLABEL[v], kbd: String(i + 1), check: st.view === v, fn: function () { self.setView(v); } }; }) });
    items.push({ label: S.sortBy, sub: this.sortItems() });
    this.menu(items, x, y);
  };
  F.winMenu = function (x, y) {
    var self = this, note = function () { self.toast(T('In the app, this is the real Windows menu, with all your shell extensions.', 'Dans l’app, c’est le vrai menu de Windows, avec toutes vos extensions.')); };
    this.menu([
      { head: T('Windows menu', 'Menu Windows') },
      { label: S.win.openWith, icon: 'apps', sub: [{ label: T('Photos', 'Photos'), fn: note }, { label: T('Notepad', 'Bloc-notes'), fn: note }, { label: T('Choose another app', 'Choisir une autre application'), fn: note }] },
      { label: S.win.share, icon: 'share', fn: note },
      { label: S.win.compress, icon: 'documents', fn: note },
      { label: S.win.asPath, icon: 'list', fn: note },
      { label: S.win.send, icon: 'r', sub: [{ label: T('Desktop (create shortcut)', 'Bureau (créer un raccourci)'), fn: note }, { label: T('Mail recipient', 'Destinataire'), fn: note }] },
      { sep: 1 },
      { label: S.win.props, icon: 'more', kbd: 'Alt+Enter', fn: note }
    ], x, y, 'win11');
  };

  /* ----- Get Info ----- */
  F.toggleInfo = function (force) {
    if (this.info && !force) return this.closeInfo();
    if (!this.info) { this.info = h('div', 'fw-info'); this.info.setAttribute('role', 'dialog'); this.el.appendChild(this.info); }
    this.renderInfo();
    var self = this; requestAnimationFrame(function () { if (self.info) self.info.classList.add('in'); });
  };
  F.closeInfo = function () { if (this.info) { this.info.remove(); this.info = null; } };
  F.renderInfo = function () {
    var n = this.st.sel[0] || this.st.loc.n; if (!n || !this.info) return;
    this.info.setAttribute('aria-label', S.info + ': ' + n.name);
    var where = n.parent && n.parent !== this.fs.root ? this.pathStr(n.parent) : '—';
    var row = function (a, b) { return '<dt>' + a + '</dt><dd>' + b + '</dd>'; };
    this.info.innerHTML = '<div class="fi-bar"><button type="button" class="ql-x" data-act="info-x" aria-label="' + S.close + '">' + ico('x') + '</button><b>' + esc(n.name) + '</b></div>' +
      '<div class="fi-top"><div class="fi-i">' + iconHTML(n, true) + '</div><div><b>' + esc(n.name) + '</b><span>' + (n.dir ? S.nItems(n.children.length) : fmtSize(sizeOf(n))) + '</span><span>' + S.modifiedS + (LANG === 'fr' ? ' : ' : ': ') + fmtDate(n.date) + '</span></div></div>' +
      '<dl>' + row(S.kind, kindOf(n)) + row(S.size, n.dir ? fmtSize(sizeOf(n)) + ' · ' + S.nItems(n.children.length) : fmtSize(sizeOf(n))) + row(S.where, esc(where)) + row(S.created, fmtDate(n.cdate || n.date - 864e5 * 3)) + row(S.modifiedS, fmtDate(n.date)) + '</dl>' +
      '<h6>' + S.tagsLbl + '</h6><div class="fi-tags">' + ((n.tags || []).map(function (t) { return '<span><i style="background:' + TAGS[t] + '"></i>' + S[t] + '</span>'; }).join('') || '<em>—</em>') + '</div>';
  };

  /* ----- Quick Look (inside the window) ----- */
  F.openQL = function () {
    var n = this.st.sel[this.st.sel.length - 1]; if (!n) return;
    if (this.renaming) this.commitRename();
    var wrap = h('div', 'ql-wrap'), self = this;
    this.$('.fw-toast').classList.remove('show');
    wrap.innerHTML = '<div class="ql" role="dialog" aria-modal="false"><div class="ql-bar"><button type="button" class="ql-x" data-act="ql-x" aria-label="' + S.close + '">' + ico('x') + '</button><b class="ql-name"></b><span class="ql-st"></span><button type="button" class="ql-open" data-act="ql-open"></button></div><div class="ql-body"></div></div>';
    wrap.addEventListener('pointerdown', function (e) { if (e.target === wrap) { self.closeQL(); } });
    this.el.appendChild(wrap);
    this.ql = { wrap: wrap, n: null, c: null };
    var e = this.itemEl(n), wr = this.el.getBoundingClientRect();
    if (e) { var r = e.getBoundingClientRect(); wrap.querySelector('.ql').style.transformOrigin = (r.left + r.width / 2 - wr.left) + 'px ' + (r.top + r.height / 2 - wr.top) + 'px'; }
    this.refreshQL();
    requestAnimationFrame(function () { wrap.classList.add('in'); });
    this.emit('ql', n);
  };
  F.refreshQL = function () {
    var q = this.ql, n = this.st.sel[this.st.sel.length - 1], self = this;
    if (!q) return;
    if (!n) return this.closeQL();
    if (q.n === n) return;
    if (q.c) q.c.destroy();
    q.n = n;
    q.c = qlContent(n, { onEdit: function () { q.dirty = true; var s = q.wrap.querySelector('.ql-st'); s.textContent = S.edited + ' · ' + S.saveHint; s.className = 'ql-st edit'; } });
    var panel = q.wrap.querySelector('.ql');
    panel.className = 'ql t-' + q.c.type;
    panel.setAttribute('aria-label', S.ql + ': ' + n.name);
    q.wrap.querySelector('.ql-name').textContent = n.name;
    var st = q.wrap.querySelector('.ql-st'); st.className = 'ql-st'; st.textContent = n.dir ? S.nItems(n.children.length) : fmtSize(sizeOf(n));
    q.wrap.querySelector('.ql-open').textContent = n.dir ? S.open : openWithLabel(q.c.type);
    var body = q.wrap.querySelector('.ql-body'); body.innerHTML = ''; body.appendChild(q.c.el);
  };
  F.closeQL = function () {
    var q = this.ql; if (!q) return;
    this.ql = null;
    if (q.c) q.c.destroy();
    q.wrap.classList.remove('in'); q.wrap.classList.add('out');
    setTimeout(function () { q.wrap.remove(); }, 200);
    if (q.dirty) { var keep = this.st.sel.slice(); this.render(); if (this.st.view !== 'columns') this.select(keep, { noScroll: true }); }
    try { this.el.focus({ preventScroll: true }); } catch (e) { }
  };
  F.saved = function () {
    if (!this.ql) return;
    var s = this.ql.wrap.querySelector('.ql-st'); s.textContent = S.toastSaved + ' ✓'; s.className = 'ql-st ok';
    this.emit('edit');
  };

  /* ----- scripted states (used by the page) ----- */
  F.show = function (c) {
    var st = this.st;
    this.closeQL(); this.closeInfo(); closeMenu();
    if (c.loc && this.locKey(st.loc) !== c.loc) this.go(this.resolve(c.loc), { quiet: true });
    st.view = c.view || st.view; st.trail = []; st.exp = {}; st.sel = [];
    var self = this;
    (c.expand || []).forEach(function (p) { var n = self.find(p); if (n) st.exp[n.id] = true; });
    if (c.trail) { var acc = []; st.trail = c.trail.map(function (nm) { acc.push(nm); return self.find(acc.slice()); }).filter(Boolean); }
    this.render(true);
    if (c.select) this.select(c.select.map(function (p) { return self.find(p); }), { noScroll: true });
    if (c.trail && st.trail.length) { st.sel = [st.trail[st.trail.length - 1]]; this.updateSel(); }
  };

  window.WFFinder = Finder;
  window.WFCloseMenu = closeMenu;
})();
