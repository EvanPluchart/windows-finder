/* Windows Finder landing: demo data (strings, virtual disk, generated artwork). */
(function () {
  'use strict';
  var root = document.documentElement;
  var LANG = (root.lang || 'en').slice(0, 2) === 'fr' ? 'fr' : 'en';
  var me = document.currentScript;
  var ASSETS = me ? new URL('./', me.src).href : 'assets/';
  var T = function (en, fr) { return LANG === 'fr' ? fr : en; };

  var S = {
    favorites: T('Favorites', 'Favoris'), locations: T('Locations', 'Emplacements'), tagsH: T('Tags', 'Étiquettes'),
    recents: T('Recents', 'Récents'), applications: 'Applications', desktop: T('Desktop', 'Bureau'), documents: 'Documents',
    downloads: T('Downloads', 'Téléchargements'), pictures: T('Pictures', 'Images'), music: T('Music', 'Musique'), videos: T('Videos', 'Vidéos'),
    onedrive: 'OneDrive', disk: T('Local Disk (C:)', 'Disque local (C:)'), wsl: 'Ubuntu (WSL)', network: T('Network', 'Réseau'), trash: T('Recycle Bin', 'Corbeille'),
    red: T('Red', 'Rouge'), orange: 'Orange', yellow: T('Yellow', 'Jaune'), green: T('Green', 'Vert'), blue: T('Blue', 'Bleu'), purple: T('Purple', 'Violet'), gray: T('Gray', 'Gris'),
    name: T('Name', 'Nom'), modified: T('Date Modified', 'Date de modification'), size: T('Size', 'Taille'), kind: T('Kind', 'Type'),
    today: T('Today at', "Aujourd'hui à"), yesterday: T('Yesterday at', 'Hier à'), at: T('at', 'à'),
    items: function (n) { return n + ' ' + (n === 1 ? T('item', 'élément') : T('items', 'éléments')); },
    selOf: function (a, b) { return T(a + ' of ' + b + ' selected', a + ' sur ' + b + ' sélectionné' + (a > 1 ? 's' : '')); },
    free: T('295 GB available', '295 Go disponibles'),
    empty: T('This folder is empty', 'Ce dossier est vide'),
    noResults: T('No results', 'Aucun résultat'),
    searchPh: T('Search', 'Rechercher'), searching: T('Searching', 'Recherche dans'), thisPC: T('This PC', 'Ce PC'),
    all: T('All', 'Tout'), kDocs: 'Documents', kImages: 'Images', kFolders: T('Folders', 'Dossiers'), kMedia: T('Audio / Video', 'Audio / Vidéo'),
    back: T('Back', 'Précédent'), fwd: T('Forward', 'Suivant'), views: T('View', 'Présentation'),
    vIcons: T('Icons', 'Icônes'), vList: T('List', 'Liste'), vColumns: T('Columns', 'Colonnes'), vGallery: T('Gallery', 'Galerie'),
    share: T('Share', 'Partager'), tagBtn: T('Edit Tags', 'Étiquettes'), more: T('More', 'Plus'), search: T('Search', 'Rechercher'),
    group: T('Group', 'Grouper'), sidebar: T('Sidebar', 'Barre latérale'),
    open: T('Open', 'Ouvrir'), ql: T('Quick Look', 'Aperçu rapide'), info: T('Get Info', 'Lire les informations'), rename: T('Rename', 'Renommer'),
    duplicate: T('Duplicate', 'Dupliquer'), toTrash: T('Move to Recycle Bin', 'Placer dans la corbeille'), putBack: T('Put Back', 'Remettre à sa place'),
    emptyTrash: T('Empty Recycle Bin', 'Vider la corbeille'), copyPath: T('Copy Path', 'Copier le chemin'), newFolder: T('New Folder', 'Nouveau dossier'),
    untitled: T('untitled folder', 'dossier sans titre'), copySuffix: T(' copy', ' copie'),
    moreOpts: T('More Options…', "Plus d'options…"), viewAs: T('View as', 'Présentation'), sortBy: T('Sort By', 'Trier par'),
    openTerm: T('Open in Terminal', 'Ouvrir dans le Terminal'),
    openWith: T('Open with', 'Ouvrir avec'), openWithApp: function (a) { return T('Open with ' + a, 'Ouvrir avec ' + a); },
    win: { openWith: T('Open with', 'Ouvrir avec'), share: T('Share', 'Partager'), asPath: T('Copy as path', 'Copier en tant que chemin d’accès'), compress: T('Compress to ZIP file', 'Compresser dans un fichier ZIP'), send: T('Send to', 'Envoyer vers'), props: T('Properties', 'Propriétés') },
    toastTrash: function (n) { return T('Moved ' + n + ' to the Recycle Bin', 'Placé ' + n + ' dans la corbeille'); },
    toastMove: function (n, d) { return T('Moved ' + n + ' to “' + d + '”', 'Déplacé ' + n + ' vers « ' + d + ' »'); },
    toastPut: function (n) { return T('Put back ' + n, 'Remis à sa place : ' + n); },
    toastCopy: T('Path copied', 'Chemin copié'), toastSaved: T('Saved', 'Enregistré'), toastEmpty: T('Recycle Bin emptied', 'Corbeille vidée'),
    toastUndone: T('Undone', 'Annulé'), undo: T('Undo', 'Annuler'),
    nItems: function (n) { return n === 1 ? T('1 item', '1 élément') : T(n + ' items', n + ' éléments'); },
    q: function (s) { return T('“' + s + '”', '« ' + s + ' »'); },
    edited: T('Edited', 'Modifié'), saveHint: T('Ctrl+S to save', 'Ctrl+S pour enregistrer'),
    page: function (a, b) { return T('Page ' + a + ' of ' + b, 'Page ' + a + ' sur ' + b); },
    infoH: T('Info', 'Informations'), where: T('Where', 'Emplacement'), created: T('Created', 'Création'), modifiedS: T('Modified', 'Modification'),
    tagsLbl: T('Tags', 'Étiquettes'), close: T('Close', 'Fermer'),
    hintOpen: T('Double-click to open', 'Double-clic pour ouvrir'), hintTap: T('Tap again to open', 'Touchez encore pour ouvrir'),
    kinds: {
      folder: T('Folder', 'Dossier'), jpg: T('JPEG image', 'Image JPEG'), png: T('PNG image', 'Image PNG'), pdf: T('PDF document', 'Document PDF'),
      csv: T('CSV document', 'Document CSV'), txt: T('Plain text', 'Texte brut'), mp4: T('MPEG-4 movie', 'Vidéo MPEG-4'), zip: T('ZIP archive', 'Archive ZIP'),
      exe: 'Application', mp3: T('MP3 audio', 'Audio MP3'), app: 'Application', drive: T('Local disk', 'Disque local'), sh: T('Shell script', 'Script shell')
    }
  };

  /* ---------- generated artwork (SVG) ---------- */
  function svg(body, w, h) { w = w || 600; h = h || 400; return '<svg xmlns="http://www.w3.org/2000/svg" width="' + w + '" height="' + h + '" viewBox="0 0 ' + w + ' ' + h + '" preserveAspectRatio="xMidYMid slice">' + body + '</svg>'; }
  function lg(id, stops, x2, y2) {
    return '<linearGradient id="' + id + '" x1="0" y1="0" x2="' + (x2 || 0) + '" y2="' + (y2 == null ? 1 : y2) + '">' +
      stops.map(function (s) { return '<stop offset="' + s[0] + '" stop-color="' + s[1] + '"/>'; }).join('') + '</linearGradient>';
  }
  function rnd(seed) { return function () { seed = (seed * 16807) % 2147483647; return (seed - 1) / 2147483646; }; }
  function stars(n, seed, h) {
    var r = rnd(seed), o = '';
    for (var i = 0; i < n; i++) o += '<circle cx="' + (r() * 600).toFixed(1) + '" cy="' + (r() * h).toFixed(1) + '" r="' + (r() * 1.3 + .3).toFixed(2) + '" fill="#fff" opacity="' + (r() * .7 + .3).toFixed(2) + '"/>';
    return o;
  }
  var ART = {
    sunset: function () {
      return svg('<defs>' + lg('a', [[0, '#2b1b5a'], [.45, '#c2417a'], [.75, '#ff8a4c'], [1, '#ffc46b']]) + lg('b', [[0, '#ff9a5a'], [1, '#3a1d4f']]) + '</defs>' +
        '<rect width="600" height="400" fill="url(#a)"/><circle cx="300" cy="250" r="62" fill="#ffe3a1" opacity=".95"/>' +
        '<rect y="262" width="600" height="138" fill="url(#b)"/>' +
        '<g fill="#ffd9a0" opacity=".55"><rect x="250" y="272" width="100" height="4" rx="2"/><rect x="265" y="286" width="70" height="3" rx="1.5"/><rect x="278" y="300" width="44" height="3" rx="1.5"/><rect x="288" y="314" width="24" height="2" rx="1"/></g>' +
        '<path d="M0 262 L70 228 L130 250 L200 214 L250 262Z" fill="#2a1240" opacity=".85"/><path d="M380 262 L450 222 L520 240 L600 208 L600 262Z" fill="#2a1240" opacity=".9"/>');
    },
    mountains: function () {
      return svg('<defs>' + lg('a', [[0, '#8fd0ff'], [1, '#e8f4ff']]) + lg('m', [[0, '#5b7ea8'], [1, '#2b3f5f']]) + lg('n', [[0, '#7d9cc4'], [1, '#41597d']]) + '</defs>' +
        '<rect width="600" height="400" fill="url(#a)"/><circle cx="470" cy="90" r="34" fill="#fff" opacity=".9"/>' +
        '<path d="M0 300 L120 150 L190 220 L290 90 L400 230 L470 170 L600 280 L600 400 L0 400Z" fill="url(#n)"/>' +
        '<path d="M290 90 L262 126 L282 120 L300 136 L316 118 L330 134Z M120 150 L100 175 L118 170 L132 180 L144 166Z" fill="#fff"/>' +
        '<path d="M0 340 L90 260 L170 320 L260 240 L360 330 L460 250 L600 330 L600 400 L0 400Z" fill="url(#m)"/>' +
        '<path d="M0 380 Q150 340 300 372 T600 360 L600 400 L0 400Z" fill="#1f2d44"/>');
    },
    coast: function () {
      return svg('<defs>' + lg('s', [[0, '#7fd3ff'], [1, '#d9f3ff']]) + lg('w', [[0, '#16b3c9'], [1, '#0a6f96']]) + lg('d', [[0, '#f6dfb0'], [1, '#e7c084']]) + '</defs>' +
        '<rect width="600" height="400" fill="url(#s)"/><rect y="170" width="600" height="230" fill="url(#w)"/>' +
        '<path d="M0 170 L80 120 L150 150 L180 170Z" fill="#6a8f5a"/><path d="M600 400 L600 230 Q470 250 400 320 Q360 370 250 400Z" fill="url(#d)"/>' +
        '<path d="M600 236 Q480 258 412 322 Q372 366 280 400" stroke="#fff" stroke-width="7" fill="none" opacity=".75"/>' +
        '<g stroke="#fff" stroke-width="3" opacity=".35" fill="none"><path d="M40 220 q20 -6 40 0 t40 0"/><path d="M180 260 q20 -6 40 0 t40 0"/><path d="M90 320 q20 -6 40 0 t40 0"/></g>');
    },
    aurora: function () {
      return svg('<defs>' + lg('a', [[0, '#050816'], [1, '#14244a']]) + lg('g', [[0, '#5dffb0'], [.6, '#3ad6c6'], [1, '#7a5cff']], 1, 0) +
        '<filter id="f"><feGaussianBlur stdDeviation="18"/></filter></defs><rect width="600" height="400" fill="url(#a)"/>' + stars(70, 7, 260) +
        '<g filter="url(#f)" opacity=".85"><path d="M-20 200 C120 60 260 220 380 110 S560 80 640 150" stroke="url(#g)" stroke-width="46" fill="none"/>' +
        '<path d="M-20 250 C140 150 280 260 420 170 S580 160 640 210" stroke="#b05cff" stroke-width="26" fill="none" opacity=".6"/></g>' +
        '<path d="M0 330 L80 280 L150 310 L230 260 L320 320 L400 270 L500 300 L600 260 L600 400 L0 400Z" fill="#05070f"/>');
    },
    dunes: function () {
      return svg('<defs>' + lg('a', [[0, '#ffb36b'], [1, '#ffe2b0']]) + lg('d1', [[0, '#f29a4d'], [1, '#c4632b']], 1, 1) + lg('d2', [[0, '#d97a3a'], [1, '#8f3f1c']], 1, 1) + '</defs>' +
        '<rect width="600" height="400" fill="url(#a)"/><circle cx="160" cy="120" r="46" fill="#fff4dc"/>' +
        '<path d="M0 260 Q160 170 330 240 T600 220 L600 400 L0 400Z" fill="url(#d1)"/><path d="M0 330 Q200 250 380 320 T600 300 L600 400 L0 400Z" fill="url(#d2)"/>' +
        '<path d="M330 240 Q420 280 600 262" stroke="#ffd0a0" stroke-width="3" fill="none" opacity=".6"/>');
    },
    city: function () {
      var r = rnd(11), b = '', w = '';
      for (var x = 0; x < 600; x += 34 + Math.floor(r() * 20)) {
        var h = 90 + r() * 180, bw = 28 + r() * 26;
        b += '<rect x="' + x + '" y="' + (400 - h).toFixed(0) + '" width="' + bw.toFixed(0) + '" height="' + h.toFixed(0) + '" fill="#0d1226"/>';
        for (var y = 400 - h + 10; y < 390; y += 14) for (var c = x + 6; c < x + bw - 6; c += 9) if (r() > .55) w += '<rect x="' + c.toFixed(0) + '" y="' + y.toFixed(0) + '" width="4" height="6" fill="' + (r() > .3 ? '#ffd27a' : '#8ad8ff') + '" opacity="' + (r() * .6 + .4).toFixed(2) + '"/>';
      }
      return svg('<defs>' + lg('a', [[0, '#0b1030'], [.7, '#3b2a6b'], [1, '#c4588a']]) + '</defs><rect width="600" height="400" fill="url(#a)"/>' + stars(40, 3, 200) +
        '<circle cx="480" cy="80" r="26" fill="#f4f1ff"/><circle cx="470" cy="74" r="24" fill="#2a2457" opacity=".2"/>' + b + w);
    },
    tiles: function () {
      var o = '';
      for (var y = 0; y < 400; y += 100) for (var x = 0; x < 600; x += 100) {
        o += '<g transform="translate(' + x + ' ' + y + ')"><rect width="100" height="100" fill="#f4f7ff"/><path d="M50 8 L92 50 L50 92 L8 50Z" fill="none" stroke="#1d4fa8" stroke-width="5"/>' +
          '<circle cx="50" cy="50" r="15" fill="#1d4fa8"/><circle cx="50" cy="50" r="6" fill="#f4f7ff"/><path d="M0 0h18M0 0v18M100 0h-18M100 0v18M0 100h18M0 100v-18M100 100h-18M100 100v-18" stroke="#3b78d8" stroke-width="7"/></g>';
      }
      return svg(o + '<g stroke="#c9d6f2" stroke-width="2">' + [100, 200, 300, 400, 500].map(function (x) { return '<path d="M' + x + ' 0V400"/>'; }).join('') + [100, 200, 300].map(function (y) { return '<path d="M0 ' + y + 'H600"/>'; }).join('') + '</g>');
    },
    rooftops: function () {
      var r = rnd(5), o = '';
      for (var row = 0; row < 5; row++) for (var x = -30; x < 620; x += 70 + r() * 30) {
        var y = 120 + row * 60 + r() * 16, w = 70 + r() * 40;
        o += '<rect x="' + x.toFixed(0) + '" y="' + (y + 20).toFixed(0) + '" width="' + w.toFixed(0) + '" height="140" fill="' + ['#f3e6cf', '#f7d9b8', '#fbf3e6', '#e9cfa8'][Math.floor(r() * 4)] + '"/>' +
          '<path d="M' + (x - 6).toFixed(0) + ' ' + (y + 22).toFixed(0) + ' L' + (x + w / 2).toFixed(0) + ' ' + y.toFixed(0) + ' L' + (x + w + 6).toFixed(0) + ' ' + (y + 22).toFixed(0) + 'Z" fill="' + ['#c8562d', '#d9693a', '#b44a26'][Math.floor(r() * 3)] + '"/>' +
          '<rect x="' + (x + 14).toFixed(0) + '" y="' + (y + 40).toFixed(0) + '" width="10" height="16" fill="#5a7da8" opacity=".7"/><rect x="' + (x + 36).toFixed(0) + '" y="' + (y + 40).toFixed(0) + '" width="10" height="16" fill="#5a7da8" opacity=".7"/>';
      }
      return svg('<defs>' + lg('a', [[0, '#8ccfff'], [1, '#e8f6ff']]) + '</defs><rect width="600" height="400" fill="url(#a)"/><rect y="150" width="600" height="20" fill="#5fb0e0" opacity=".6"/>' + o);
    },
    tram: function () {
      return svg('<defs>' + lg('a', [[0, '#ffe6b8'], [1, '#ffd08a']]) + '</defs><rect width="600" height="400" fill="url(#a)"/>' +
        '<rect x="0" y="40" width="200" height="300" fill="#f2c9a0"/><rect x="420" y="20" width="180" height="320" fill="#e9b88f"/>' +
        '<g fill="#6b8fb8" opacity=".6"><rect x="30" y="80" width="30" height="44"/><rect x="110" y="80" width="30" height="44"/><rect x="450" y="70" width="30" height="44"/><rect x="530" y="70" width="30" height="44"/></g>' +
        '<rect y="330" width="600" height="70" fill="#8a7f72"/><path d="M0 360H600M0 376H600" stroke="#4d463e" stroke-width="3"/>' +
        '<path d="M220 120 L210 96 M380 120 L392 96" stroke="#222" stroke-width="3"/><path d="M150 96 H460" stroke="#222" stroke-width="2"/>' +
        '<rect x="170" y="120" width="260" height="210" rx="22" fill="#ffc21a"/><rect x="170" y="250" width="260" height="56" fill="#f0f0f0"/>' +
        '<g fill="#2b3a4a"><rect x="188" y="140" width="50" height="70" rx="6"/><rect x="250" y="140" width="50" height="70" rx="6"/><rect x="312" y="140" width="50" height="70" rx="6"/><rect x="374" y="140" width="40" height="70" rx="6"/></g>' +
        '<text x="300" y="290" font-family="Arial" font-weight="700" font-size="26" text-anchor="middle" fill="#2b3a4a">28</text>' +
        '<circle cx="215" cy="330" r="14" fill="#333"/><circle cx="385" cy="330" r="14" fill="#333"/>');
    },
    wire: function () {
      return svg('<rect width="600" height="400" fill="#f5f6f8"/><rect x="30" y="24" width="540" height="34" rx="8" fill="#dfe3ea"/><rect x="44" y="36" width="80" height="10" rx="5" fill="#b8c0cc"/>' +
        '<rect x="30" y="78" width="330" height="170" rx="12" fill="#e4e8ef"/><path d="M30 78 L360 248 M360 78 L30 248" stroke="#c9cfda" stroke-width="2"/>' +
        '<rect x="380" y="78" width="190" height="16" rx="8" fill="#c4cbd6"/><rect x="380" y="106" width="150" height="10" rx="5" fill="#d3d8e0"/><rect x="380" y="124" width="170" height="10" rx="5" fill="#d3d8e0"/>' +
        '<rect x="380" y="200" width="110" height="34" rx="17" fill="#2f7df4"/><g fill="#e4e8ef"><rect x="30" y="268" width="170" height="104" rx="12"/><rect x="215" y="268" width="170" height="104" rx="12"/><rect x="400" y="268" width="170" height="104" rx="12"/></g>');
    },
    shot: function () {
      return svg('<defs>' + lg('a', [[0, '#3b6cff'], [1, '#9b5cff']], 1, 1) + '</defs><rect width="600" height="400" fill="url(#a)"/>' +
        '<rect x="70" y="60" width="460" height="290" rx="16" fill="#fff" opacity=".95"/><rect x="70" y="60" width="120" height="290" rx="16" fill="#ececee"/>' +
        '<g fill="#ff5f57"><circle cx="92" cy="82" r="6"/></g><circle cx="110" cy="82" r="6" fill="#febc2e"/><circle cx="128" cy="82" r="6" fill="#28c840"/>' +
        '<g fill="#cfd3da"><rect x="88" y="110" width="80" height="9" rx="4"/><rect x="88" y="132" width="64" height="9" rx="4"/><rect x="88" y="154" width="74" height="9" rx="4"/></g>' +
        '<g fill="#6cbcff"><rect x="214" y="110" width="64" height="52" rx="8"/><rect x="298" y="110" width="64" height="52" rx="8"/><rect x="382" y="110" width="64" height="52" rx="8"/><rect x="214" y="200" width="64" height="52" rx="8"/></g>');
    },
    mood: function () {
      return svg('<rect width="600" height="400" fill="#f3efe9"/><rect x="30" y="30" width="250" height="200" rx="10" fill="#e06b4a"/><rect x="300" y="30" width="270" height="120" rx="10" fill="#2b4d6b"/>' +
        '<rect x="300" y="170" width="130" height="200" rx="10" fill="#f2c14e"/><rect x="450" y="170" width="120" height="200" rx="10" fill="#8fb9a8"/><rect x="30" y="250" width="250" height="120" rx="10" fill="#1f1f24"/>' +
        '<text x="50" y="330" font-family="Georgia" font-style="italic" font-size="44" fill="#f3efe9">Aa</text>');
    },
    waves: function () {
      return svg('<defs>' + lg('a', [[0, '#0b2a4a'], [1, '#0e6aa0']]) + '</defs><rect width="600" height="400" fill="url(#a)"/>' +
        '<g fill="none" stroke-linecap="round"><path class="wv1" d="M-60 230 Q0 200 60 230 T180 230 T300 230 T420 230 T540 230 T660 230" stroke="#5ed0ff" stroke-width="14" opacity=".7"/>' +
        '<path class="wv2" d="M-60 280 Q0 250 60 280 T180 280 T300 280 T420 280 T540 280 T660 280" stroke="#2fa6e8" stroke-width="18" opacity=".8"/>' +
        '<path class="wv3" d="M-60 330 Q0 300 60 330 T180 330 T300 330 T420 330 T540 330 T660 330" stroke="#0b87c9" stroke-width="22"/></g>' +
        '<circle cx="470" cy="110" r="40" fill="#ffe7a8" opacity=".9"/>');
    },
    timelapse: function () { return ART.city(); }
  };
  var artCache = {};
  function artURL(k) {
    if (!artCache[k]) artCache[k] = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent((ART[k] || ART.sunset)());
    return artCache[k];
  }

  /* ---------- virtual disk ---------- */
  var DAY = 864e5, now = Date.now(), uid = 0;
  function ago(days, h, m) { var d = new Date(now - days * DAY); d.setHours(h == null ? 10 : h, m || 0, 0, 0); return Math.min(d.getTime(), now - 60000); }
  function D(name, kids, o) { var n = { id: ++uid, name: name, dir: true, children: kids || [], date: ago(3, 14, 20) }; for (var k in o) n[k] = o[k]; return n; }
  function F(name, o) { var n = { id: ++uid, name: name, dir: false, size: 24000, date: ago(2, 11, 5) }; for (var k in o) n[k] = o[k]; return n; }

  var TX = {
    notes: T('Groceries\n- Coffee beans\n- Oranges\n- Oat milk\n\nCall the landlord about the heating.\nBook a table for Friday.', 'Courses\n- Café en grains\n- Oranges\n- Lait d’avoine\n\nAppeler le propriétaire pour le chauffage.\nRéserver une table pour vendredi.'),
    lisbon: T('Trip to Lisbon, 3 days\n\nDay 1  Alfama, Tram 28, sunset at the Miradouro\nDay 2  Belém, pastéis de nata, LX Factory\nDay 3  Sintra (train from Rossio, 40 min)\n\nPack: charger, sunglasses, comfy shoes.', 'Voyage à Lisbonne, 3 jours\n\nJour 1  Alfama, tram 28, coucher de soleil au Miradouro\nJour 2  Belém, pastéis de nata, LX Factory\nJour 3  Sintra (train depuis Rossio, 40 min)\n\nÀ emporter : chargeur, lunettes de soleil, bonnes chaussures.'),
    ideas: T('App ideas\n\n1. A plant watering reminder that reads the weather\n2. Shared shopping list with zero sign-up\n3. Photo dedupe for the family NAS', 'Idées d’apps\n\n1. Un rappel d’arrosage qui lit la météo\n2. Une liste de courses partagée, sans inscription\n3. Dédoublonner les photos du NAS familial'),
    roadmap: T('Windows Finder roadmap\n\n[x] Four views\n[x] Quick Look with editable text and CSV\n[x] Tags and smart folders\n[ ] Code signing\n[ ] More languages', 'Feuille de route Windows Finder\n\n[x] Quatre présentations\n[x] Aperçu rapide avec texte et CSV modifiables\n[x] Étiquettes et dossiers intelligents\n[ ] Signature du code\n[ ] Plus de langues'),
    release: T('v0.1.0\n\nFirst public build.\nIcons, list, columns and gallery views, tabs, Quick Look, tags, Get Info and a command palette.', 'v0.1.0\n\nPremière version publique.\nIcônes, liste, colonnes et galerie, onglets, Aperçu rapide, étiquettes, Lire les informations et palette de commandes.'),
    copy: T('Headline: Make every page feel calm.\nSubhead: Fewer clicks, clearer type, faster pages.\nCTA: Start the tour', 'Titre : Des pages qui respirent.\nSous-titre : Moins de clics, un texte plus clair, des pages plus rapides.\nBouton : Commencer la visite'),
    draft: T('Old draft. Probably safe to delete.', 'Vieux brouillon. Sans doute à supprimer.'),
    readme: T('Shared with the family.\nPut new photos in the Photos folder.', 'Partagé avec la famille.\nMettez les nouvelles photos dans le dossier Photos.'),
    bashrc: '# ~/.bashrc\nexport EDITOR=nano\nalias ll="ls -la"\nalias gs="git status"'
  };
  var CSV = {
    budget: [[T('Month', 'Mois'), T('Income', 'Revenus'), T('Rent', 'Loyer'), T('Groceries', 'Courses'), T('Transport', 'Transport'), T('Savings', 'Épargne')],
      [T('July', 'Juillet'), '3200', '1150', '420', '90', '600'], [T('August', 'Août'), '3200', '1150', '465', '120', '520'],
      [T('September', 'Septembre'), '3350', '1150', '398', '85', '700'], [T('October', 'Octobre'), '3350', '1150', '410', '95', '680']],
    receipts: [[T('Date', 'Date'), T('Store', 'Magasin'), T('Category', 'Catégorie'), T('Amount', 'Montant')],
      ['2025-01-14', T('Hardware store', 'Quincaillerie'), T('Home office', 'Bureau'), '86.40'], ['2025-03-02', T('Bookshop', 'Librairie'), T('Training', 'Formation'), '42.00'],
      ['2025-06-21', T('Electronics', 'Électronique'), T('Equipment', 'Matériel'), '329.99'], ['2025-09-09', T('Train', 'Train'), T('Travel', 'Déplacements'), '64.50']]
  };
  var PDF = {
    invoice: function (n, amt) { return { kind: 'invoice', no: n, amount: amt }; },
    lease: { kind: 'text', title: T('Residential lease', 'Bail d’habitation'), pages: 3 },
    tax: { kind: 'form', title: T('Tax return 2025', 'Déclaration de revenus 2025'), pages: 2 },
    brief: { kind: 'text', title: T('Website redesign: brief', 'Refonte du site : brief'), pages: 2 },
    report: { kind: 'report', title: T('Q3 report', 'Rapport T3'), pages: 2 }
  };

  function buildFS() {
    uid = 0;
    var pictures = D(S.pictures, [
      D(T('Lisbon', 'Lisbonne'), [
        F('Alfama.jpg', { art: 'rooftops', size: 3.1e6, date: ago(40, 18, 2) }), F('Tram 28.jpg', { art: 'tram', size: 2.7e6, date: ago(40, 18, 30), tags: ['yellow'] }),
        F(T('Tiles.jpg', 'Azulejos.jpg'), { art: 'tiles', size: 2.2e6, date: ago(39, 11, 12) }), F(T('Sunset at Belém.jpg', 'Coucher de soleil à Belém.jpg'), { art: 'sunset', size: 3.6e6, date: ago(39, 19, 48), tags: ['orange'] })
      ], { date: ago(39, 20, 1) }),
      F('Aurora.jpg', { art: 'aurora', size: 4.2e6, date: ago(12, 23, 14), tags: ['purple'] }), F(T('Mountains.jpg', 'Montagnes.jpg'), { art: 'mountains', size: 3.8e6, date: ago(9, 9, 40) }),
      F(T('Coast.jpg', 'Côte.jpg'), { art: 'coast', size: 2.9e6, date: ago(6, 16, 2), tags: ['blue'] }), F(T('Dunes.jpg', 'Dunes.jpg'), { art: 'dunes', size: 3.3e6, date: ago(5, 8, 15) }),
      F(T('City at night.jpg', 'Ville la nuit.jpg'), { art: 'city', size: 4.6e6, date: ago(1, 22, 31) })
    ], { icon: 'PicturesFolderIcon', special: 'pictures', date: ago(1, 22, 31) });

    var docs = D('Documents', [
      D(T('Projects', 'Projets'), [
        D(T('Website redesign', 'Refonte du site'), [
          F(T('Brief.pdf', 'Brief.pdf'), { pdf: PDF.brief, size: 412e3, date: ago(8, 10, 2), tags: ['red'] }),
          F(T('Wireframes.png', 'Maquettes.png'), { art: 'wire', size: 820e3, date: ago(7, 15, 44) }),
          F(T('Copy.txt', 'Textes.txt'), { text: TX.copy, size: 1.2e3, date: ago(2, 17, 12) })
        ], { date: ago(2, 17, 12), tags: ['red'] }),
        D('Windows Finder', [
          F(T('App icon.png', 'Icône.png'), { img: ASSETS + 'logo-512.webp', size: 1.3e6, date: ago(0, 10, 5), tags: ['blue'] }),
          F(T('Roadmap.txt', 'Feuille de route.txt'), { text: TX.roadmap, size: 2.1e3, date: ago(0, 9, 12) }),
          F(T('Release notes.txt', 'Notes de version.txt'), { text: TX.release, size: 1.6e3, date: ago(0, 9, 40) })
        ], { date: ago(0, 10, 5), tags: ['blue'] }),
        D(T('Moodboard', 'Planche d’ambiance'), [F('Palette.png', { art: 'mood', size: 640e3, date: ago(15, 14, 30) })], { date: ago(15, 14, 30) })
      ], { date: ago(0, 10, 5) }),
      D(T('Invoices', 'Factures'), [
        F(T('Invoice 2026-08.pdf', 'Facture 2026-08.pdf'), { pdf: PDF.invoice('2026-08', 1840), size: 96e3, date: ago(60, 9, 2) }),
        F(T('Invoice 2026-09.pdf', 'Facture 2026-09.pdf'), { pdf: PDF.invoice('2026-09', 2160), size: 98e3, date: ago(30, 9, 4) }),
        F(T('Invoice 2026-10.pdf', 'Facture 2026-10.pdf'), { pdf: PDF.invoice('2026-10', 2410), size: 101e3, date: ago(1, 9, 1), tags: ['orange'] })
      ], { date: ago(1, 9, 1) }),
      D(T('Taxes 2025', 'Impôts 2025'), [
        F(T('Tax return.pdf', 'Déclaration.pdf'), { pdf: PDF.tax, size: 512e3, date: ago(150, 11, 30) }),
        F(T('Receipts.csv', 'Reçus.csv'), { csv: CSV.receipts, size: 2.4e3, date: ago(140, 18, 6) })
      ], { date: ago(140, 18, 6) }),
      F(T('Budget.csv', 'Budget.csv'), { csv: CSV.budget, size: 3.1e3, date: ago(0, 8, 47), tags: ['green'] }),
      F(T('Lease.pdf', 'Bail.pdf'), { pdf: PDF.lease, size: 1.1e6, date: ago(210, 16, 20) }),
      F('Notes.txt', { text: TX.notes, size: 1.5e3, date: ago(1, 18, 48), tags: ['yellow'] }),
      F(T('Trip to Lisbon.txt', 'Voyage à Lisbonne.txt'), { text: TX.lisbon, size: 2.2e3, date: ago(42, 21, 10), tags: ['orange'] })
    ], { icon: 'DocumentsFolderIcon', special: 'documents', date: ago(0, 10, 5) });

    var downloads = D(S.downloads, [
      F('WindowsFinder-Setup.exe', { size: 9.8e6, date: ago(0, 9, 58), ext: 'exe' }),
      F(T('Photos export.zip', 'Export photos.zip'), { size: 148e6, date: ago(4, 20, 2) }),
      F(T('Waves.mp4', 'Vagues.mp4'), { video: 'waves', size: 24.6e6, date: ago(3, 12, 30), dur: 18 }),
      F(T('Report Q3.pdf', 'Rapport T3.pdf'), { pdf: PDF.report, size: 2.4e6, date: ago(11, 15, 2) })
    ], { icon: 'DownloadsFolder', special: 'downloads', date: ago(0, 9, 58) });

    var desktop = D(S.desktop, [
      F(T('Screenshot 2026-10-12.png', 'Capture 2026-10-12.png'), { art: 'shot', size: 1.4e6, date: ago(4, 14, 22) }),
      F(T('Ideas.txt', 'Idées.txt'), { text: TX.ideas, size: 900, date: ago(2, 23, 2), tags: ['purple'] })
    ], { icon: 'DesktopFolderIcon', special: 'desktop' });

    var music = D(S.music, [
      D(T('Road trip', 'Road trip'), [F('Track 01.mp3', { size: 8.1e6 }), F('Track 02.mp3', { size: 7.4e6 }), F('Track 03.mp3', { size: 9.2e6 })]),
      F(T('Voice memo.mp3', 'Mémo vocal.mp3'), { size: 1.2e6, date: ago(6, 8, 30) })
    ], { icon: 'MusicFolderIcon', special: 'music' });

    var videos = D(S.videos, [
      F(T('Waves.mp4', 'Vagues.mp4'), { video: 'waves', size: 24.6e6, date: ago(3, 12, 30), dur: 18 }),
      F(T('Timelapse.mp4', 'Timelapse.mp4'), { video: 'timelapse', size: 61.2e6, date: ago(20, 22, 10), dur: 32 })
    ], { icon: 'MovieFolderIcon', special: 'videos' });

    var apps = D('Applications', [
      F(T('Calculator', 'Calculatrice'), { app: 'calc' }), F('Notepad', { app: 'note' }), F('Paint', { app: 'paint' }),
      F('Terminal', { app: 'term' }), F(T('Settings', 'Paramètres'), { app: 'gear' }), F('Windows Finder', { app: 'wf' })
    ], { special: 'apps' });

    var onedrive = D('OneDrive', [
      D(T('Photos', 'Photos'), [F(T('Coast.jpg', 'Côte.jpg'), { art: 'coast', size: 2.9e6 }), F(T('Mountains.jpg', 'Montagnes.jpg'), { art: 'mountains', size: 3.8e6 })]),
      D(T('Shared', 'Partagé'), [F('README.txt', { text: TX.readme, size: 300 })])
    ], { special: 'onedrive' });

    var home = D('alex', [desktop, docs, downloads, music, pictures, videos, onedrive], { special: 'home' });
    var users = D(T('Users', 'Utilisateurs'), [home]);
    var c = D(S.disk, [D(T('Program Files', 'Programmes'), [D('Windows Finder', [F('WindowsFinder.exe', { size: 22.4e6, ext: 'exe' })])]), users, D('Windows', [])], { special: 'disk' });
    var wsl = D('Ubuntu (WSL)', [D('home', [D('alex', [F('.bashrc', { text: TX.bashrc, size: 3.7e3 }), F('deploy.sh', { text: '#!/bin/sh\nset -e\nnpm run build\nrsync -av dist/ server:/var/www', size: 210 })])]), D('etc', []), D('usr', [])], { special: 'wsl' });
    var trash = D(S.trash, [F(T('Old draft.txt', 'Vieux brouillon.txt'), { text: TX.draft, size: 400, origin: null })], { special: 'trash' });
    var top = D('', [c, wsl, trash, apps], { special: 'root' });

    (function link(n, p) { n.parent = p; if (n.dir) n.children.forEach(function (k) { link(k, n); }); })(top, null);
    trash.children[0].origin = docs;
    return { root: top, docs: docs, desktop: desktop, downloads: downloads, pictures: pictures, music: music, videos: videos, apps: apps, onedrive: onedrive, home: home, c: c, wsl: wsl, trash: trash };
  }

  window.WFData = { LANG: LANG, ASSETS: ASSETS, S: S, T: T, buildFS: buildFS, artURL: artURL };
})();
