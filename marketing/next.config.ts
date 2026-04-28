import type { NextConfig } from "next";
import createMDX from "@next/mdx";

const withMDX = createMDX({
  extension: /\.mdx?$/,
  options: {
    remarkPlugins: [["remark-frontmatter", ["yaml"]], ["remark-gfm"]],
    rehypePlugins: [],
  },
});

const nextConfig: NextConfig = {
  pageExtensions: ["ts", "tsx", "md", "mdx"],
  async rewrites() {
    const apiOrigin =
      process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

    return [
      {
        source: "/openapi.json",
        destination: `${apiOrigin}/openapi.json`,
      },
      {
        source: "/.well-known/openapi.json",
        destination: `${apiOrigin}/openapi.json`,
      },
    ];
  },
};

export default withMDX(nextConfig);
