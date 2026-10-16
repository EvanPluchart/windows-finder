// Generates the open icon set into assets/open/{sf,macicons}.
//   - symbols: Lucide glyphs (ISC licence), rendered black on transparent, trimmed to their ink box
//     (the app tints them at runtime; file names match the Apple set so the two are interchangeable)
//   - folders: hand-drawn in this file (MIT, same as the repo)
// Usage: npm install && node gen.mjs
import { Resvg } from "@resvg/resvg-js";
import { PNG } from "pngjs";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "..", "..", "assets", "open");
mkdirSync(join(out, "sf"), { recursive: true });
mkdirSync(join(out, "macicons"), { recursive: true });

const lucide = (name) => {
  const svg = readFileSync(join(here, "node_modules", "lucide-static", "icons", `${name}.svg`), "utf8");
  return svg.replace(/^[\s\S]*?<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "");
};

function renderPng(svg, width) {
  const r = new Resvg(svg, { fitTo: { mode: "width", value: width }, background: "rgba(0,0,0,0)" });
  return r.render();
}

/** Renders, then crops to the non-transparent bounding box. */
function trimmedPng(svg, width) {
  const img = renderPng(svg, width);
  const { width: w, height: h } = img;
  const px = img.pixels;
  let x0 = w, y0 = h, x1 = -1, y1 = -1;
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++)
      if (px[(y * w + x) * 4 + 3] > 8) {
        if (x < x0) x0 = x;
        if (x > x1) x1 = x;
        if (y < y0) y0 = y;
        if (y > y1) y1 = y;
      }
  const cw = x1 - x0 + 1, ch = y1 - y0 + 1;
  const png = new PNG({ width: cw, height: ch });
  for (let y = 0; y < ch; y++)
    for (let x = 0; x < cw; x++) {
      const s = ((y + y0) * w + (x + x0)) * 4, d = (y * cw + x) * 4;
      png.data[d] = 0; png.data[d + 1] = 0; png.data[d + 2] = 0; png.data[d + 3] = px[s + 3];
    }
  return PNG.sync.write(png);
}

const wrap = (inner, { stroke = 1.75, fill = "none" } = {}) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="${fill}" stroke="#000" stroke-width="${stroke}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`;

// name -> [svg inner markup or lucide name, options]
const L = (name, extra = {}) => ({ inner: lucide(name), ...extra });
const custom = (inner, extra = {}) => ({ inner, ...extra });

const regular = 1.6, medium = 2.2;
const SYMBOLS = {
  "clock": L("clock"),
  "house": L("house"),
  "menubar.dock.rectangle": L("panel-bottom"),
  "doc": L("file"),
  "arrow.down.circle": L("circle-arrow-down"),
  "photo": L("image"),
  "music.note": L("music"),
  "video": L("video"),
  "icloud": L("cloud"),
  "internaldrive": L("hard-drive"),
  "externaldrive": custom(`<rect x="2.5" y="7" width="19" height="10.5" rx="3"/><path d="M6.5 12.2 H12"/><circle cx="17" cy="12.2" r="1" fill="#000"/>`),
  "eject.fill": custom(`<path d="M12 4 L21 15 H3 Z" fill="#000"/><rect x="4" y="18" width="16" height="2.6" rx="1.3" fill="#000" stroke="none"/>`, { stroke: 1.2 }),
  "applications": custom(`<path d="M4.5 20 L12 3.5 L19.5 20"/><path d="M8 14.5 H16"/>`, { stroke: 2.1 }),
  "app.dashed": L("square-dashed"),
  "globe": L("globe"),
  "trash": L("trash-2"),
  "laptopcomputer": L("laptop"),
  "folder": L("folder"),
  "list.bullet": L("list"),
  "rectangle.split.3x1": L("columns-3"),
  "square.grid.3x1.below.line.grid.1x2": L("gallery-thumbnails"),
  "square.grid.3x2": L("grid-3x3"),
  "square.grid.2x2": L("grid-2x2"),
  "square.and.arrow.up": L("share"),
  "tag": L("tag"),
  "ellipsis": custom(`<circle cx="5.5" cy="12" r="2.2" fill="#000" stroke="none"/><circle cx="12" cy="12" r="2.2" fill="#000" stroke="none"/><circle cx="18.5" cy="12" r="2.2" fill="#000" stroke="none"/>`),
  "magnifyingglass": L("search"),
  "chevron.left": L("chevron-left", { scale: 1.35 }),
  "chevron.right": L("chevron-right", { scale: 1.35 }),
  "chevron.down": L("chevron-down", { scale: 1.35 }),
  "xmark": L("x"),
};
// Names that exist in two weights in the Apple set (suffix -medium); the plain ones exist only as regular.
const WITH_MEDIUM = ["list.bullet", "rectangle.split.3x1", "square.grid.3x1.below.line.grid.1x2", "square.grid.3x2", "square.and.arrow.up", "tag", "ellipsis", "magnifyingglass"];
const ONLY_MEDIUM = ["chevron.left", "chevron.right", "chevron.down", "square.grid.2x2", "xmark"];

const BASE = 160; // px for 24 units, close to the Apple glyph bodies
function emit(file, def, strokeWidth) {
  const stroke = def.stroke ?? strokeWidth;
  const svg = wrap(def.inner, { stroke });
  writeFileSync(join(out, "sf", `${file}.png`), trimmedPng(svg, Math.round(BASE * (def.scale ?? 1) * 1)));
}
for (const [name, def] of Object.entries(SYMBOLS)) {
  if (ONLY_MEDIUM.includes(name)) emit(`${name}-medium`, def, medium);
  else {
    emit(name, def, regular);
    if (WITH_MEDIUM.includes(name)) emit(`${name}-medium`, def, medium);
  }
}

// ---- Folders -------------------------------------------------------------------------------

const folder = (emblem) => `<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <defs>
    <linearGradient id="back" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#62b6f2"/><stop offset="1" stop-color="#3e92dc"/></linearGradient>
    <linearGradient id="front" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#a8e4ff"/><stop offset="0.55" stop-color="#62c4fb"/><stop offset="1" stop-color="#44a9f4"/></linearGradient>
    <filter id="soft" x="-20%" y="-20%" width="140%" height="160%"><feGaussianBlur stdDeviation="6"/></filter>
  </defs>
  <ellipse cx="128" cy="218" rx="104" ry="11" fill="#0a3a66" opacity="0.28" filter="url(#soft)"/>
  <path d="M22 66 a16 16 0 0 1 16-16 h54 c8 0 12.5 3.6 17 8.6 l6 6.4 h103 a16 16 0 0 1 16 16 V190 a16 16 0 0 1 -16 16 H38 a16 16 0 0 1 -16 -16 Z" fill="url(#back)"/>
  <rect x="22" y="84" width="212" height="126" rx="16" fill="url(#front)"/>
  <path d="M38 85.5 H218" stroke="#ffffff" stroke-opacity="0.55" stroke-width="2" stroke-linecap="round"/>
  ${emblem ? `<g transform="translate(128 150)">
    <g transform="translate(-36 -35) scale(3)" stroke="#ffffff" stroke-opacity="0.5" stroke-width="1.7" fill="none" stroke-linecap="round" stroke-linejoin="round">${emblem}</g>
    <g transform="translate(-36 -36.5) scale(3)" stroke="#1d6fc0" stroke-opacity="0.78" stroke-width="1.7" fill="none" stroke-linecap="round" stroke-linejoin="round">${emblem}</g>
  </g>` : ""}
</svg>`;

const FOLDERS = {
  GenericFolderIcon: null,
  DesktopFolderIcon: lucide("monitor"),
  DocumentsFolderIcon: lucide("file-text"),
  DownloadsFolder: lucide("circle-arrow-down"),
  MovieFolderIcon: lucide("film"),
  MusicFolderIcon: lucide("music"),
  PicturesFolderIcon: lucide("image"),
};
for (const [file, emblem] of Object.entries(FOLDERS)) {
  writeFileSync(join(out, "macicons", `${file}.png`), Buffer.from(renderPng(folder(emblem), 256).asPng()));
}
console.log("done:", Object.keys(SYMBOLS).length, "symbols,", Object.keys(FOLDERS).length, "folders ->", out);
