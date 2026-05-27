import fs from "node:fs/promises";
import path from "node:path";
import ogConfig from "./og.config.mjs";
import { renderOgPng } from "./scripts/og.mjs";

export default function (eleventyConfig) {
  eleventyConfig.addPassthroughCopy({ "src/assets": "assets" });

  eleventyConfig.addFilter("isoDate", (value) => {
    const d = value instanceof Date ? value : new Date(value);
    return d.toISOString().slice(0, 10);
  });

  eleventyConfig.on("eleventy.after", async ({ dir }) => {
    const outRoot = path.join(dir.output, "assets/og");
    await fs.mkdir(outRoot, { recursive: true });
    for (const entry of ogConfig) {
      const png = await renderOgPng(entry);
      await fs.writeFile(path.join(outRoot, `${entry.slug}.png`), png);
    }
  });

  return {
    dir: {
      input: "src",
      includes: "_includes",
      data: "_data",
      output: "_site",
    },
    markdownTemplateEngine: "njk",
    htmlTemplateEngine: "njk",
    templateFormats: ["njk", "md", "html"],
  };
}
