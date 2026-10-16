/* Windows Finder landing: page interactions. Needs fs.js and finder.js. */
(function () {
  'use strict';
  var W = window.WFData, U = window.WFUtil, Finder = window.WFFinder;
  var T = W ? W.T : function (a) { return a; };
  var root = document.documentElement;
  var $ = function (s, c) { return (c || document).querySelector(s); };
  var $$ = function (s, c) { return Array.prototype.slice.call((c || document).querySelectorAll(s)); };
  var reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  var store = { get: function (k) { try { return localStorage.getItem(k); } catch (e) { return null; } }, set: function (k, v) { try { localStorage.setItem(k, v); } catch (e) { } } };
  var sleep = function (ms) { return new Promise(function (r) { setTimeout(r, ms); }); };

  /* ---------- theme + accent ---------- */
  function isDark() { var t = root.getAttribute('data-theme'); return t ? t === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches; }
  function setTheme(t) {
    if (t === 'auto') { root.removeAttribute('data-theme'); store.set('wf-theme', ''); }
    else { root.setAttribute('data-theme', t); store.set('wf-theme', t); }
    $$('[data-theme-set]').forEach(function (b) { b.setAttribute('aria-pressed', b.dataset.themeSet === (root.getAttribute('data-theme') || 'auto')); });
  }
  var themeBtn = $('#theme');
  if (themeBtn) themeBtn.addEventListener('click', function () { setTheme(isDark() ? 'light' : 'dark'); });
  $$('[data-theme-set]').forEach(function (b) { b.addEventListener('click', function () { setTheme(b.dataset.themeSet); }); });
  setTheme(root.getAttribute('data-theme') || 'auto');

  // (code, selection light, selection dark, accent light, accent dark), from src/theme.rs
  var ACCENTS = { blue: ['#0064e1', '#0458d0', '#2f7df4', '#4d94ff'], purple: ['#8944ab', '#7d3f9e', '#a550a7', '#bf5af2'], pink: ['#d92b72', '#c42a67', '#f74f9e', '#ff6bb3'], red: ['#d3302f', '#bf2c2b', '#f5524f', '#ff6b66'],
    orange: ['#e0640a', '#c95a09', '#f7822b', '#ff9a47'], yellow: ['#d9a400', '#c29200', '#f0b82a', '#ffd23d'], green: ['#2ea043', '#278a39', '#3fbf5a', '#4fd36d'], graphite: ['#6b6b70', '#5a5a5e', '#8e8e93', '#a0a0a5'] };
  function setAccent(k, save) {
    var a = ACCENTS[k] || ACCENTS.blue;
    root.style.setProperty('--sel-l', a[0]); root.style.setProperty('--sel-d', a[1]);
    root.style.setProperty('--acc-l', a[2]); root.style.setProperty('--acc-d', a[3]);
    $$('.swatches button').forEach(function (b) { b.setAttribute('aria-pressed', b.dataset.accent === k); });
    if (save) store.set('wf-accent', k);
  }
  $$('.swatches button').forEach(function (b) { b.style.setProperty('--c', ACCENTS[b.dataset.accent][2]); b.addEventListener('click', function () { setAccent(b.dataset.accent, true); }); });
  setAccent(store.get('wf-accent') || 'blue');

  /* ---------- header, reveal, counters, spotlight ---------- */
  var hd = $('.hd');
  var onScroll = function () { if (hd) hd.classList.toggle('scrolled', scrollY > 8); };
  addEventListener('scroll', onScroll, { passive: true }); onScroll();

  var io = 'IntersectionObserver' in window ? new IntersectionObserver(function (es) {
    es.forEach(function (e) {
      if (!e.isIntersecting) return;
      e.target.classList.add('in', 'in-view');
      if (e.target.dataset.count) count(e.target);
      io.unobserve(e.target);
    });
  }, { rootMargin: '0px 0px -8% 0px', threshold: .12 }) : null;
  $$('.rv, .tile, [data-count]').forEach(function (el) { if (io) io.observe(el); else el.classList.add('in', 'in-view'); });
  function count(el) {
    var end = +el.dataset.count, pre = el.dataset.prefix || '', t0 = performance.now(), d = reduce ? 1 : 1200;
    (function step(t) { var p = Math.min(1, (t - t0) / d), v = Math.round(end * (1 - Math.pow(1 - p, 3))); el.textContent = pre + v + (el.dataset.suffix || ''); if (p < 1) requestAnimationFrame(step); })(t0);
  }
  $$('.tile').forEach(function (t) {
    t.addEventListener('pointermove', function (e) { var r = t.getBoundingClientRect(); t.style.setProperty('--mx', (e.clientX - r.left) + 'px'); t.style.setProperty('--my', (e.clientY - r.top) + 'px'); });
  });

  /* hero logo follows the pointer a little; the desk straightens as it scrolls into view */
  var logo = $('.hero-logo'), desk = $('#desk'), fine = matchMedia('(pointer: fine)').matches;
  if (logo && fine && !reduce) {
    addEventListener('pointermove', function (e) {
      var x = e.clientX / innerWidth - .5, y = e.clientY / innerHeight - .5;
      logo.style.transform = 'perspective(600px) rotateY(' + (x * 18).toFixed(2) + 'deg) rotateX(' + (-y * 14).toFixed(2) + 'deg)';
    }, { passive: true });
  }
  if (desk && !reduce) {
    var tick = false;
    var tilt = function () {
      tick = false;
      if (innerWidth < 760) { desk.style.transform = ''; return; }
      var r = desk.getBoundingClientRect(), p = Math.max(0, Math.min(1, (innerHeight - r.top) / (innerHeight * .75)));
      desk.style.transform = p >= 1 ? '' : 'perspective(1800px) rotateX(' + ((1 - p) * 16).toFixed(2) + 'deg) scale(' + (.93 + p * .07).toFixed(4) + ')';
    };
    addEventListener('scroll', function () { if (!tick) { tick = true; requestAnimationFrame(tilt); } }, { passive: true });
    addEventListener('resize', tilt); tilt();
  }

  if (!W || !Finder) return;
  $$('.fic[data-file]').forEach(function (e) { e.innerHTML = U.iconHTML({ name: e.dataset.file, dir: false }); });

  /* ---------- hero demo + quests ---------- */
  var mount = $('#hero-finder'), hero = null, touring = false;
  if (mount) {
    hero = new Finder(mount, { start: 'docs', view: 'icons', label: T('Windows Finder demo. Double-click folders to open them, press Space for Quick Look, right-click for more.', 'Démo de Windows Finder. Double-cliquez sur un dossier pour l’ouvrir, Espace pour l’Aperçu rapide, clic droit pour plus.') });
    window.wfDemo = hero;
    var quests = $('#quests'), ring = quests && $('.q-ring .fg', quests), done = {};
    var total = quests ? $$('[data-q]', quests).length : 0;
    var tickQ = function (k) {
      if (touring || done[k] || !quests) return;
      var li = $('[data-q="' + k + '"]', quests); if (!li) return;
      done[k] = 1; li.classList.add('done', 'pop');
      var n = Object.keys(done).length;
      $('.q-ring b', quests).textContent = n + '/' + total;
      if (ring) ring.style.strokeDashoffset = String(150.8 * (1 - n / total));
      $('.q-head span', quests).textContent = n === total ? T('All done. Nice.', 'Tout est fait. Bravo.') : T(total - n + ' left', 'Encore ' + (total - n));
      if (n === total) setTimeout(function () { quests.classList.add('all'); }, 700);
    };
    hero.on(function (e, d) {
      if (e === 'interact') stopTour();
      if (e === 'open-folder') tickQ('open');
      if (e === 'ql') tickQ('ql');
      if (e === 'view' && d === 'columns') tickQ('columns');
      if (e === 'tag') tickQ('tag');
      if (e === 'rename') tickQ('rename');
      if (e === 'search') tickQ('search');
      if (e === 'move' || e === 'trash') tickQ('move');
    });
    $$('[data-q]').forEach(function (li) { li.addEventListener('click', function () { hint(li.dataset.q); }); });
  }
  function hint(k) {
    if (!hero) return;
    var msg = {
      open: T('Double-click a folder, or select it and press Ctrl+↓.', 'Double-cliquez sur un dossier, ou sélectionnez-le puis Ctrl+↓.'),
      ql: T('Select a file and press Space.', 'Sélectionnez un fichier et appuyez sur Espace.'),
      columns: T('Click the third view button, or press 3.', 'Cliquez sur le troisième bouton de présentation, ou appuyez sur 3.'),
      tag: T('Right-click a file and pick a colour.', 'Clic droit sur un fichier, puis choisissez une couleur.'),
      rename: T('Select a file and press Enter.', 'Sélectionnez un fichier et appuyez sur Entrée.'),
      search: T('Click the magnifier and type “invoice”.', 'Cliquez sur la loupe et tapez « facture ».'),
      move: T('Drag a file onto a folder, or press Delete.', 'Glissez un fichier sur un dossier, ou appuyez sur Suppr.')
    }[k];
    if (msg) hero.toast(msg);
    hero.el.focus({ preventScroll: true });
  }

  /* ---------- guided tour with a ghost cursor ---------- */
  var ghost = null, tour = null;
  function stopTour() { if (tour) { tour.stop = true; tour = null; } touring = false; if (ghost) ghost.classList.remove('on'); }
  function ghostTo(el, token) {
    if (token.stop) throw 0;
    var dr = desk.getBoundingClientRect(), r = el.getBoundingClientRect();
    ghost.style.transform = 'translate(' + (r.left - dr.left + Math.min(r.width / 2, 40)) + 'px,' + (r.top - dr.top + r.height / 2) + 'px)';
    return sleep(780).then(function () { if (token.stop) throw 0; ghost.classList.remove('tap'); void ghost.offsetWidth; ghost.classList.add('tap'); return sleep(220); });
  }
  function runTour() {
    if (!hero || !desk) return;
    stopTour();
    var tk = tour = { stop: false }, f = hero, fs = f.fs;
    if (!ghost) { ghost = document.createElement('div'); ghost.className = 'ghost'; ghost.setAttribute('aria-hidden', 'true'); ghost.innerHTML = '<svg viewBox="0 0 24 24"><path d="M5 3l14 8-6.2 1.6L10 19z" fill="#111" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>'; desk.appendChild(ghost); }
    touring = true;
    var dr = desk.getBoundingClientRect();
    ghost.style.transition = 'none'; ghost.style.transform = 'translate(' + (dr.width * .55) + 'px,' + (dr.height * .9) + 'px)'; void ghost.offsetWidth; ghost.style.transition = '';
    ghost.classList.add('on');
    var item = function (n) { return f.itemEl(n); };
    var step = Promise.resolve()
      .then(function () { f.show({ loc: 'docs', view: 'icons' }); return sleep(500); })
      .then(function () { return ghostTo($('[data-loc="pictures"]', f.el), tk); })
      .then(function () { f.go(f.resolve('pictures')); return sleep(700); })
      .then(function () { var a = fs.pictures.children.filter(function (n) { return n.art === 'aurora'; })[0]; return ghostTo(item(a), tk).then(function () { f.select([a]); return sleep(500); }); })
      .then(function () { f.openQL(); return sleep(1700); })
      .then(function () { if (tk.stop) throw 0; f.arrow('ArrowRight'); return sleep(1300); })
      .then(function () { if (tk.stop) throw 0; f.arrow('ArrowRight'); return sleep(1300); })
      .then(function () { if (tk.stop) throw 0; f.closeQL(); return ghostTo($('.fw-seg [data-view="columns"]', f.el), tk); })
      .then(function () { f.setView('columns', true); return sleep(700); })
      .then(function () { var l = fs.pictures.children.filter(function (n) { return n.dir; })[0]; return ghostTo(item(l), tk).then(function () { f.select([l], { col: 0 }); return sleep(800); }).then(function () { return l; }); })
      .then(function (l) { var tr = l.children.filter(function (n) { return n.art === 'tram'; })[0]; return ghostTo(item(tr), tk).then(function () { f.select([tr], { col: 1 }); return sleep(1800); }); })
      .then(function () { return ghostTo($('.fw-seg [data-view="icons"]', f.el), tk); })
      .then(function () { f.setView('icons', true); return ghostTo($('[data-loc="docs"]', f.el), tk); })
      .then(function () { f.show({ loc: 'docs', view: 'icons' }); f.toast(T('Your turn. Double-click a folder, press Space on a file.', 'À vous. Double-cliquez sur un dossier, Espace sur un fichier.')); return sleep(600); })
      .catch(function () { })
      .then(function () { if (tour === tk) { tour = null; touring = false; ghost.classList.remove('on'); } });
    return step;
  }
  var tourBtn = $('#tour');
  if (tourBtn) tourBtn.addEventListener('click', function () { desk.scrollIntoView({ behavior: reduce ? 'auto' : 'smooth', block: 'center' }); setTimeout(runTour, reduce ? 0 : 450); });
  if (desk && hero && !reduce && 'IntersectionObserver' in window) {
    var auto = true, t0 = null;
    hero.on(function (e) { if (e === 'interact') auto = false; });
    new IntersectionObserver(function (es) {
      es.forEach(function (e) {
        clearTimeout(t0);
        if (e.isIntersecting && auto) t0 = setTimeout(function () { if (auto) { auto = false; runTour(); } }, 1400);
      });
    }, { threshold: .6 }).observe(desk);
  }

  /* ---------- the four views, driven by scroll ---------- */
  var vMount = $('#views-finder');
  if (vMount) {
    var vf = new Finder(vMount, { start: 'docs', view: 'icons', compact: true, noDrag: true, label: T('Windows Finder views', 'Présentations de Windows Finder') });
    vf.el.classList.add('vivid');
    var P = T('Projects', 'Projets');
    var CFG = {
      icons: { loc: 'docs', view: 'icons', select: [[T('Budget.csv', 'Budget.csv')]] },
      list: { loc: 'docs', view: 'list', expand: [[P], [P, 'Windows Finder']], select: [[P, 'Windows Finder', T('Roadmap.txt', 'Feuille de route.txt')]] },
      columns: { loc: 'docs', view: 'columns', trail: [P, 'Windows Finder', T('App icon.png', 'Icône.png')] },
      gallery: { loc: 'pictures', view: 'gallery', select: [['Aurora.jpg']] }
    };
    var cur = '';
    var activate = function (k) {
      if (k === cur) return; cur = k;
      $$('.vs-step').forEach(function (s) { var on = s.dataset.v === k; s.classList.toggle('on', on); var b = $('.vs-tab', s); if (b) b.setAttribute('aria-pressed', on); });
      vf.show(CFG[k]);
    };
    activate('icons');
    var wide = matchMedia('(min-width: 961px)');
    $$('.vs-step').forEach(function (s) {
      $('.vs-tab', s).addEventListener('click', function () {
        if (wide.matches) s.scrollIntoView({ behavior: reduce ? 'auto' : 'smooth', block: 'center' });
        activate(s.dataset.v);
      });
    });
    if ('IntersectionObserver' in window) {
      var vio = new IntersectionObserver(function (es) { if (!wide.matches) return; es.forEach(function (e) { if (e.isIntersecting) activate(e.target.dataset.v); }); }, { rootMargin: '-45% 0px -45% 0px' });
      $$('.vs-step').forEach(function (s) { vio.observe(s); });
    }
  }

  /* ---------- Quick Look showcase ---------- */
  var qp = $('#qls-panel');
  if (qp) {
    var qfs = W.buildFS(), cur2 = null, items = {};
    U.walk(qfs.root, function (n) {
      if (!items.image && n.art === 'tram') items.image = n;
      if (!items.video && n.video === 'waves') items.video = n;
      if (n.pdf && n.pdf.kind === 'invoice') items.pdf = n;
      if (!items.csv && n.csv && n.parent === qfs.docs) items.csv = n;
      if (!items.text && n.text != null && /lisbon/i.test(n.name)) items.text = n;
    });
    $$('.qls-tabs [data-f]').forEach(function (b) { var n = items[b.dataset.f]; if (n) $('.ti', b).innerHTML = U.iconHTML(n); });
    var showQ = function (k) {
      var n = items[k]; if (!n) return;
      if (cur2) cur2.destroy();
      cur2 = U.qlContent(n, { onEdit: function () { var s = $('.ql-st', qp); s.textContent = W.S.edited; s.className = 'ql-st edit'; } });
      qp.className = 'ql t-' + cur2.type + (reduce ? '' : ' swap');
      $('.ql-name', qp).textContent = n.name;
      var st = $('.ql-st', qp); st.className = 'ql-st'; st.textContent = U.fmtSize(U.sizeOf(n));
      $('.ql-open', qp).textContent = U.openWithLabel(cur2.type);
      var body = $('.ql-body', qp); body.innerHTML = ''; body.appendChild(cur2.el);
      $$('.qls-tabs [data-f]').forEach(function (b) { b.setAttribute('aria-selected', b.dataset.f === k); b.tabIndex = b.dataset.f === k ? 0 : -1; });
    };
    var tabs = $$('.qls-tabs [data-f]');
    tabs.forEach(function (b, i) {
      b.addEventListener('click', function () { showQ(b.dataset.f); });
      b.addEventListener('keydown', function (e) {
        var d = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[e.key]; if (!d) return;
        e.preventDefault(); var nb = tabs[(i + d + tabs.length) % tabs.length]; nb.focus(); showQ(nb.dataset.f);
      });
    });
    showQ('image');
    var kb = $('.qls .sh kbd');
    if (kb && !reduce) setInterval(function () { kb.classList.add('press'); setTimeout(function () { kb.classList.remove('press'); }, 160); }, 2600);
  }

  /* ---------- command palette tile ---------- */
  var palIn = $('#pal-in'), palL = $('#pal-l');
  if (palIn && palL) {
    var go = function (fn) { return function () { fn(); }; };
    var CMDS = [
      [T('Switch to Column view', 'Présentation en colonnes'), '3', 'columns', go(function () { hero && hero.setView('columns'); })],
      [T('Switch to Gallery view', 'Présentation en galerie'), '4', 'gallery', go(function () { hero && hero.setView('gallery'); })],
      [T('Switch to List view', 'Présentation en liste'), '2', 'list', go(function () { hero && hero.setView('list'); })],
      [T('Switch to Icon view', 'Présentation en icônes'), '1', 'grid', go(function () { hero && hero.setView('icons'); })],
      [T('Toggle dark mode', 'Basculer le mode sombre'), '', 'moon', go(function () { setTheme(isDark() ? 'light' : 'dark'); })],
      [T('Accent colour: Purple', 'Couleur d’accent : violet'), '', 'pal', go(function () { setAccent('purple', true); })],
      [T('Accent colour: Green', 'Couleur d’accent : vert'), '', 'pal', go(function () { setAccent('green', true); })],
      [T('Accent colour: Blue', 'Couleur d’accent : bleu'), '', 'pal', go(function () { setAccent('blue', true); })],
      [T('Go to Folder…', 'Aller au dossier…'), 'Ctrl ⇧ G', 'folder', null],
      [T('New Tab', 'Nouvel onglet'), 'Ctrl T', 'plus', null],
      [T('Reopen Closed Tab', 'Rouvrir l’onglet fermé'), 'Ctrl ⇧ T', 'undo', null],
      [T('Get Info', 'Lire les informations'), 'Ctrl I', 'info', null],
      [T('Show Hidden Files', 'Afficher les fichiers cachés'), 'Ctrl ⇧ .', 'eye', null],
      [T('Copy Path', 'Copier le chemin'), 'Ctrl ⇧ C', 'copy', null],
      [T('Connect to Server…', 'Se connecter au serveur…'), 'Ctrl K', 'net', null],
      [T('Rename', 'Renommer'), 'F2', 'edit', null],
      [T('Open in Terminal', 'Ouvrir dans le Terminal'), '', 'term', null],
      [T('View Options', 'Options de présentation'), 'Ctrl J', 'sliders', null]
    ];
    var sel = 0, shown = [];
    var hl = function (s, q) { if (!q) return U.esc(s); var i = U.norm(s).indexOf(U.norm(q)); return i < 0 ? U.esc(s) : U.esc(s.slice(0, i)) + '<mark>' + U.esc(s.slice(i, i + q.length)) + '</mark>' + U.esc(s.slice(i + q.length)); };
    var draw = function () {
      var q = palIn.value.trim();
      shown = CMDS.filter(function (c) { return !q || U.norm(c[0]).indexOf(U.norm(q)) >= 0; });
      sel = Math.min(sel, Math.max(0, shown.length - 1));
      palL.innerHTML = shown.length ? shown.map(function (c, i) {
        return '<li role="option" id="pal-o' + i + '" aria-selected="' + (i === sel) + '"' + (i === sel ? ' class="on"' : '') + ' data-i="' + i + '"><svg class="i"><use href="#i-' + c[2] + '"/></svg><span>' + hl(c[0], q) + '</span>' + (c[1] ? '<kbd>' + c[1] + '</kbd>' : '') + '</li>';
      }).join('') : '<li class="none">' + T('No command found', 'Aucune commande') + '</li>';
      palIn.setAttribute('aria-activedescendant', shown.length ? 'pal-o' + sel : '');
    };
    var run = function (i) {
      var c = shown[i]; if (!c) return;
      var li = palL.querySelector('[data-i="' + i + '"]'); if (li) { li.classList.remove('flash'); void li.offsetWidth; li.classList.add('flash'); }
      if (c[3]) c[3]();
      var note = $('#pal-note'); if (note) note.textContent = c[3] ? T('Done. ', 'Fait. ') + (c[2] === 'moon' || c[2] === 'pal' ? T('Look at the page.', 'Regardez la page.') : T('Look at the demo above.', 'Regardez la démo plus haut.')) : T('In the app, this runs the command.', 'Dans l’app, cela lance la commande.');
    };
    palIn.addEventListener('input', function () { sel = 0; draw(); });
    palIn.addEventListener('keydown', function (e) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); sel = (sel + (e.key === 'ArrowDown' ? 1 : -1) + shown.length) % Math.max(1, shown.length); draw(); var o = $('#pal-o' + sel); if (o) o.parentNode.scrollTop = o.offsetTop - 60; }
      if (e.key === 'Enter') { e.preventDefault(); run(sel); }
    });
    palL.addEventListener('click', function (e) { var li = e.target.closest('[data-i]'); if (li) { sel = +li.dataset.i; draw(); run(sel); } });
    draw();
  }

  /* ---------- tags + search tiles ---------- */
  var tagTile = $('#tag-tile');
  if (tagTile) {
    var on = null;
    $$('.chips button', tagTile).forEach(function (b) {
      b.addEventListener('click', function () {
        on = on === b.dataset.tag ? null : b.dataset.tag;
        $$('.chips button', tagTile).forEach(function (x) { x.setAttribute('aria-pressed', x.dataset.tag === on); });
        $$('.flist li', tagTile).forEach(function (li) { li.classList.toggle('off', !!on && (li.dataset.tags || '').split(' ').indexOf(on) < 0); });
      });
    });
  }
  var sIn = $('#s-in');
  if (sIn) {
    var lis = $$('#s-list li'), cnt = $('#s-count');
    var names = lis.map(function (li) { return $('.nm', li).textContent; });
    var filter = function () {
      var q = sIn.value.trim(), n = 0;
      lis.forEach(function (li, i) {
        var m = !q || U.norm(names[i]).indexOf(U.norm(q)) >= 0; li.classList.toggle('off', !m); if (m) n++;
        var nm = $('.nm', li), j = U.norm(names[i]).indexOf(U.norm(q));
        nm.innerHTML = q && j >= 0 ? U.esc(names[i].slice(0, j)) + '<mark>' + U.esc(names[i].slice(j, j + q.length)) + '</mark>' + U.esc(names[i].slice(j + q.length)) : U.esc(names[i]);
      });
      if (cnt) cnt.textContent = T(n + (n === 1 ? ' result' : ' results') + ' in This PC', n + ' résultat' + (n > 1 ? 's' : '') + ' dans Ce PC');
    };
    sIn.addEventListener('input', filter); filter();
  }

  /* ---------- tabs tile ---------- */
  var tb = $('#tabbar');
  if (tb) {
    var pool = [T('Pictures', 'Images'), 'OneDrive', T('Projects', 'Projets'), 'Ubuntu (WSL)', T('Desktop', 'Bureau'), T('Music', 'Musique'), T('Downloads', 'Téléchargements')], k = 0;
    var closed = [];
    var mk = function (name) {
      var b = document.createElement('button'); b.type = 'button'; b.className = 'tab'; b.setAttribute('role', 'tab');
      b.innerHTML = '<span>' + U.esc(name) + '</span><i class="x" role="button" aria-label="' + T('Close tab', 'Fermer l’onglet') + '"><svg class="i"><use href="#i-x"/></svg></i>';
      return b;
    };
    var pick = function (b) { $$('.tab', tb).forEach(function (x) { x.setAttribute('aria-selected', x === b); }); };
    tb.addEventListener('click', function (e) {
      var x = e.target.closest('.x'), t = e.target.closest('.tab');
      if (e.target.closest('.add')) {
        if ($$('.tab', tb).length >= 5) $$('.tab', tb)[0].remove();
        var nb = mk(pool[k++ % pool.length]); tb.insertBefore(nb, $('.add', tb)); pick(nb); return;
      }
      if (x && t) { var was = t.getAttribute('aria-selected') === 'true'; closed.push($('span', t).textContent); t.remove(); var rest = $$('.tab', tb); if (!rest.length) { var nb2 = mk(T('Documents', 'Documents')); tb.insertBefore(nb2, $('.add', tb)); rest = [nb2]; } if (was) pick(rest[rest.length - 1]); return; }
      if (t) pick(t);
    });
    var reo = $('#tab-reopen');
    if (reo) reo.addEventListener('click', function () { var n = closed.pop(); if (!n) return; var nb = mk(n); tb.insertBefore(nb, $('.add', tb)); pick(nb); });
  }

  /* ---------- shortcuts light up ---------- */
  var keys = $('#keys');
  if (keys) {
    var keysVis = false;
    if ('IntersectionObserver' in window) new IntersectionObserver(function (es) { keysVis = es[0].isIntersecting; }).observe(keys);
    document.addEventListener('keydown', function (e) {
      if (!keysVis || e.target.closest && e.target.closest('input, textarea, [contenteditable], .fw')) return;
      var k = e.key.length === 1 ? e.key.toLowerCase() : e.key;
      var id = (e.ctrlKey || e.metaKey ? 'ctrl+' : '') + (e.shiftKey && k.length === 1 && /[a-z.]/.test(k) ? 'shift+' : '') + (k === ' ' ? 'space' : k);
      var hit = keys.querySelector('[data-k~="' + id.toLowerCase() + '"]');
      if (!hit) return;
      hit.classList.remove('hit'); void hit.offsetWidth; hit.classList.add('hit');
      setTimeout(function () { hit.classList.remove('hit'); }, 650);
    });
  }

  /* ---------- copy build commands ---------- */
  $$('.copy').forEach(function (b) {
    b.addEventListener('click', function () {
      var pre = b.parentNode.querySelector('pre'), txt = pre.innerText.split('\n').filter(function (l) { return l.trim() && l.trim()[0] !== '#'; }).join('\n');
      var ok = function () { var o = b.textContent; b.textContent = T('Copied', 'Copié'); setTimeout(function () { b.textContent = o; }, 1400); };
      try { navigator.clipboard.writeText(txt).then(ok, function () { }); } catch (e) { }
    });
  });
})();
