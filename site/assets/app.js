// Windows Finder landing page: interactive Finder-style window + theme toggle. No dependencies.
(function () {
  var L = window.WF_LABELS || { name: "Name", date: "Date Modified", size: "Size", kind: "Kind", folder: "Folder", items: "1 of 8 selected, 295 GB available", preview: "Preview" };

  var folderSVG = '<svg viewBox="0 0 64 56" aria-hidden="true"><defs><linearGradient id="fg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--folder)"/><stop offset="1" stop-color="var(--folder-2)"/></linearGradient></defs><path d="M4 12a6 6 0 0 1 6-6h14l6 6h24a6 6 0 0 1 6 6v28a6 6 0 0 1-6 6H10a6 6 0 0 1-6-6z" fill="url(#fg)"/><path d="M4 20h56v26a6 6 0 0 1-6 6H10a6 6 0 0 1-6-6z" fill="var(--folder)" opacity=".55"/></svg>';
  function fileSVG(c) { return '<svg viewBox="0 0 56 64" aria-hidden="true"><path d="M8 4h28l14 14v38a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4V8a4 4 0 0 1 4-4z" fill="#fff" stroke="#c9ccd6"/><path d="M36 4v14h14z" fill="#e3e6ee"/><rect x="12" y="30" width="30" height="4" rx="2" fill="' + c + '"/><rect x="12" y="40" width="22" height="4" rx="2" fill="#d5d8e2"/></svg>'; }

  var files = [
    { n: "Documents", d: "Today at 14:03", s: "--", k: "Folder", dir: true },
    { n: "Downloads", d: "Today at 09:41", s: "--", k: "Folder", dir: true },
    { n: "Pictures", d: "Yesterday at 21:13", s: "--", k: "Folder", dir: true },
    { n: "Projects", d: "Oct 12, 2026", s: "--", k: "Folder", dir: true },
    { n: "Budget.xlsx", d: "Today at 11:20", s: "48 KB", k: "XLSX file", c: "#1f9d55" },
    { n: "Notes.md", d: "Yesterday at 18:48", s: "2.7 KB", k: "MD file", c: "#0a64f0" },
    { n: "Trip.pdf", d: "Oct 9, 2026", s: "1.2 MB", k: "PDF file", c: "#e5484d" },
    { n: "Demo.mp4", d: "Oct 3, 2026", s: "86 MB", k: "MP4 file", c: "#8e4ec6" }
  ];
  function icon(f) { return f.dir ? folderSVG : fileSVG(f.c); }

  function icons() {
    return '<div class="view v-icons">' + files.map(function (f, i) {
      return '<div class="ic' + (i === 0 ? " sel" : "") + '">' + icon(f) + "<b>" + f.n + "</b></div>";
    }).join("") + "</div>";
  }
  function list() {
    return '<div class="view v-list"><table><thead><tr><th>' + L.name + "</th><th>" + L.date + "</th><th>" + L.size + "</th><th>" + L.kind + "</th></tr></thead><tbody>" +
      files.map(function (f, i) {
        return '<tr class="' + (i === 0 ? "sel" : "") + '"><td>' + icon(f) + f.n + "</td><td>" + f.d + "</td><td>" + f.s + "</td><td>" + (f.dir ? L.folder : f.k) + "</td></tr>";
      }).join("") + "</tbody></table></div>";
  }
  function columns() {
    var c1 = files.slice(0, 5).map(function (f, i) { return '<div class="' + (i === 0 ? "sel" : "") + (f.dir ? "" : " leaf") + '">' + icon(f) + f.n + "</div>"; }).join("");
    var c2 = ["Invoices", "Taxes", "Receipts", "Budget.xlsx"].map(function (n, i) {
      var leaf = i === 3; return '<div class="' + (i === 3 ? "sel" : "") + (leaf ? " leaf" : "") + '">' + (leaf ? fileSVG("#1f9d55") : folderSVG) + n + "</div>";
    }).join("");
    return '<div class="view v-columns"><div class="col">' + c1 + '</div><div class="col">' + c2 + '</div><div class="col pv">' + fileSVG("#1f9d55") + "<b>Budget.xlsx</b><small>XLSX file · 48 KB</small><small>" + L.preview + "</small></div></div>";
  }
  function gallery() {
    return '<div class="view v-gallery"><div class="gl-main">Trip.pdf</div><div class="gl-strip">' + files.map(function () { return "<i></i>"; }).join("") + "</div></div>";
  }

  var host = document.getElementById("mock-content");
  if (host) {
    host.innerHTML = icons() + list() + columns() + gallery();
    var win = document.getElementById("mock");
    var btns = document.querySelectorAll("#mock .seg button");
    function set(v) {
      win.setAttribute("data-view", v);
      btns.forEach(function (b) { b.setAttribute("aria-pressed", String(b.dataset.v === v)); });
    }
    btns.forEach(function (b) { b.addEventListener("click", function () { stop(); set(b.dataset.v); }); });
    var order = ["icons", "list", "columns", "gallery"], i = 0, timer = null;
    function stop() { if (timer) { clearInterval(timer); timer = null; } }
    set("icons");
    if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      timer = setInterval(function () { i = (i + 1) % order.length; set(order[i]); }, 3200);
    }
    var st = document.getElementById("mock-status");
    if (st) st.textContent = L.items;
  }

  // Theme toggle (remembers the choice; falls back to the system setting).
  var root = document.documentElement, key = "wf-theme", tbtn = document.getElementById("theme");
  try { var saved = localStorage.getItem(key); if (saved) root.setAttribute("data-theme", saved); } catch (e) {}
  function dark() { return root.getAttribute("data-theme") === "dark" || (!root.getAttribute("data-theme") && window.matchMedia("(prefers-color-scheme: dark)").matches); }
  function paint() { if (tbtn) tbtn.textContent = dark() ? "☀" : "☾"; }
  if (tbtn) tbtn.addEventListener("click", function () {
    var next = dark() ? "light" : "dark"; root.setAttribute("data-theme", next);
    try { localStorage.setItem(key, next); } catch (e) {}
    paint();
  });
  paint();
})();
