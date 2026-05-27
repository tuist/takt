// Resolves a per-page `seo` object that base.njk renders into the head.
// Pages may override pieces with `seoTitle`, `seoDescription`, or `ogType`;
// otherwise we fall back to page-level title/description and site defaults.
export default {
  seo: (data) => {
    const site = data.site || {};
    const og = data.og || {};
    const url = data.page?.url ?? "/";
    const entry = og[url] || og["/"] || {};
    const slug = entry.slug || "index";

    return {
      title: data.seoTitle || data.title || `${site.name} — ${site.tagline}`,
      description: data.seoDescription || data.description || site.description,
      canonical: `${site.url}${url}`,
      image: `${site.url}/assets/og/${slug}.png`,
      imageAlt: entry.imageAlt || `${site.name} — ${site.tagline}`,
      ogType: data.ogType || "website",
    };
  },
};
