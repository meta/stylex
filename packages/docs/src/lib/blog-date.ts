/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

// Blog posts encode their publication date as a `YYYY-MM-DD` prefix on the
// filename (e.g. `2026-06-23-my-post.mdx`); there is no `date` frontmatter field.
// These helpers are the single source of truth for reading and formatting that
// date, shared by the blog post page and the RSS feed.

/**
 * Parse the `YYYY-MM-DD` prefix from a blog page path into a Date.
 * Returns `null` when the path has no date prefix.
 */
export function parseBlogDate(path: string): Date | null {
  const dateStr = path.match(/^(\d{4}-\d{2}-\d{2})/)?.[1];
  return dateStr ? new Date(dateStr) : null;
}

/**
 * Format a blog date as e.g. "June 23, 2026".
 *
 * The date is interpreted and formatted in UTC: `new Date('2026-06-23')` is UTC
 * midnight, so formatting in a behind-UTC zone would roll the displayed day back
 * by one. UTC keeps the date exactly as written in the filename.
 */
export function formatBlogDate(date: Date): string {
  return new Intl.DateTimeFormat('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    timeZone: 'UTC',
  }).format(date);
}
