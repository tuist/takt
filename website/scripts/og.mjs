import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import satori from "satori";
import { html } from "satori-html";
import { Resvg } from "@resvg/resvg-js";

const here = path.dirname(fileURLToPath(import.meta.url));

const fontRegular = await fs.readFile(path.join(here, "fonts/JetBrainsMono-Regular.ttf"));
const fontBold = await fs.readFile(path.join(here, "fonts/JetBrainsMono-Bold.ttf"));

const FONTS = [
  { name: "JetBrains Mono", data: fontRegular, weight: 400, style: "normal" },
  { name: "JetBrains Mono", data: fontBold, weight: 700, style: "normal" },
];

const WIDTH = 1200;
const HEIGHT = 630;

const PAPER = "#f6f3ec";
const INK = "#1c1c1c";
const INK_SOFT = "#4a4a4a";
const INK_MUTED = "#7d7a72";
const MARK = "#b6502a";

function template({ title, subtitle, description }) {
  return html`
    <div style="display: flex; flex-direction: column; width: 1200px; height: 630px; background: ${PAPER}; color: ${INK}; font-family: 'JetBrains Mono'; padding: 56px 64px;">
      <div style="display: flex; justify-content: space-between; align-items: center; font-size: 22px; letter-spacing: 4px; text-transform: uppercase; color: ${INK_MUTED}; border-top: 1px solid ${INK}; border-bottom: 1px solid ${INK}; padding: 10px 0;">
        <span style="display: flex;">takt(1)</span>
        <span style="display: flex;">${subtitle || "an agent substrate"}</span>
        <span style="display: flex;">tuist/takt</span>
      </div>
      <div style="display: flex; flex-direction: column; flex-grow: 1; justify-content: center;">
        <div style="display: flex; align-items: baseline; gap: 28px;">
          <span style="color: ${MARK}; font-size: 112px; font-weight: 700; line-height: 1;">$</span>
          <span style="color: ${INK}; font-size: 112px; font-weight: 700; line-height: 1; letter-spacing: -2px;">${title}</span>
        </div>
        <div style="display: flex; margin-top: 36px; font-size: 40px; color: ${INK_SOFT}; line-height: 1.35; max-width: 1000px;">${description}</div>
      </div>
      <div style="display: flex; justify-content: flex-end; align-items: center; font-size: 22px; color: ${INK_MUTED}; border-top: 1px solid ${INK}; padding: 10px 0;">
        <span style="display: flex;">github.com/tuist/takt</span>
      </div>
    </div>
  `;
}

export async function renderOgPng(entry) {
  const tree = template(entry);
  const svg = await satori(tree, { width: WIDTH, height: HEIGHT, fonts: FONTS });
  const resvg = new Resvg(svg, {
    fitTo: { mode: "width", value: WIDTH },
    font: { loadSystemFonts: false },
  });
  return resvg.render().asPng();
}

export const OG_WIDTH = WIDTH;
export const OG_HEIGHT = HEIGHT;
