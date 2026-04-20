import type { MDXComponents } from "mdx/types";
import { blogMdxComponents } from "@/components/blog/mdx-components";

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    ...components,
    ...blogMdxComponents,
  };
}
