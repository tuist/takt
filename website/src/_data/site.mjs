// Site-level metadata used by every page. Override the canonical URL by
// setting SITE_URL at build time so deploys to preview/staging environments
// resolve OG and canonical links correctly.
const url = (process.env.SITE_URL || "https://takt.tuist.dev").replace(/\/+$/, "");

export default {
  url,
  name: "Takt",
  tagline: "an agent substrate",
  description:
    "Takt is a durable substrate for AI agent harnesses: reusable packages, configurable actions, composable workflows, and inspectable artifacts. The harness stays the interface; Takt lives underneath.",
  locale: "en_US",
  author: {
    name: "Tuist",
    url: "https://tuist.dev",
  },
  repo: "https://github.com/tuist/takt",
  twitter: "@tuistio",
};
