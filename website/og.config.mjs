// Per-page Open Graph configuration.
//
// Add an entry per URL the site exposes. The build hook generates one PNG per
// entry at _site/assets/og/<slug>.png, and the same data is exposed to
// templates via the `og` global data so meta tags resolve to the right image.
export default [
  {
    slug: "index",
    url: "/",
    title: "Takt",
    subtitle: "an agent substrate",
    description: "The harness stays the interface. Takt lives underneath.",
    imageAlt: "Takt — an agent substrate. The harness stays the interface; Takt lives underneath.",
  },
];
