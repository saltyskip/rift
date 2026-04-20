import fs from "node:fs";
import path from "node:path";
import matter from "gray-matter";

export interface Author {
  name: string;
  url?: string;
  github?: string;
  twitter?: string;
  bio?: string;
}

export interface PostFrontmatter {
  title: string;
  description: string;
  slug: string;
  publishedAt: string;
  updatedAt?: string;
  author: Author;
  targetKeyword: string;
  secondaryKeywords?: string[];
  category?: string;
  ogImage?: string;
}

export interface Post {
  frontmatter: PostFrontmatter;
  slug: string;
  filePath: string;
}

const BLOG_DIR = path.join(process.cwd(), "content", "blog");

function readPost(filename: string): Post {
  const filePath = path.join(BLOG_DIR, filename);
  const raw = fs.readFileSync(filePath, "utf8");
  const { data } = matter(raw);
  const slug = data.slug ?? filename.replace(/\.mdx?$/, "");
  return {
    frontmatter: { ...data, slug } as PostFrontmatter,
    slug,
    filePath,
  };
}

export function getAllPosts(): Post[] {
  if (!fs.existsSync(BLOG_DIR)) return [];
  return fs
    .readdirSync(BLOG_DIR)
    .filter((f) => /\.mdx?$/.test(f))
    .map(readPost)
    .sort(
      (a, b) =>
        new Date(b.frontmatter.publishedAt).getTime() -
        new Date(a.frontmatter.publishedAt).getTime(),
    );
}

export function getPostBySlug(slug: string): Post | null {
  if (!fs.existsSync(BLOG_DIR)) return null;
  const candidates = [`${slug}.mdx`, `${slug}.md`];
  for (const file of candidates) {
    const p = path.join(BLOG_DIR, file);
    if (fs.existsSync(p)) return readPost(file);
  }
  return null;
}

export function getAllSlugs(): string[] {
  return getAllPosts().map((p) => p.slug);
}
