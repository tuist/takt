// Exposes Open Graph entries to templates keyed by URL, so a page can resolve
// its OG metadata with `og[page.url]`. The same source of truth is used by
// the eleventy.after hook to generate one PNG per entry.
import entries from "../../og.config.mjs";

const byUrl = Object.fromEntries(entries.map((e) => [e.url, e]));

export default byUrl;
