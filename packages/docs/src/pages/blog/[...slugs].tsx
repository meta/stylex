/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
import * as stylex from '@stylexjs/stylex';
import { blogSource } from '@/lib/source';
import { PageProps } from 'waku/router';
import {
  DocsBody,
  DocsDescription,
  DocsPage,
  DocsTitle,
} from '@/components/layout/page';
import { baseOptions } from '@/lib/layout.shared';
import { parseBlogDate, formatBlogDate } from '@/lib/blog-date';
import { mdxComponents } from '@/components/mdx';
import nmnImage from '@/static/img/nmn.jpg';
import necolasImage from '@/static/img/necolas.jpg';
import mellyeliuImage from '@/static/img/mellyeliu.jpg';
import vincentriemerImage from '@/static/img/vincentriemer.png';
import { vars } from '@/theming/vars.stylex';

export default function BlogPage({ slugs }: PageProps<'/blog/[...slugs]'>) {
  const pages = blogSource.getPages();

  const slug = slugs[0];

  const page = pages.find((page) => page.data.slug === slug);

  if (!page) {
    return (
      <div {...stylex.props(styles.fallbackContainer)}>
        <h1 {...stylex.props(styles.fallbackTitle)}>Page Not Found {slug}</h1>
        <p {...stylex.props(styles.fallbackDescription)}>
          The page you are looking for does not exist.
        </p>
      </div>
    );
  }

  const MDX = page.data.body;
  const authors = page.data.authors.map(
    (author) => AUTHORS[author as keyof typeof AUTHORS],
  );
  const publishedDate = parseBlogDate(page.path);

  return (
    <DocsPage
      {...baseOptions()}
      toc={page.data.toc}
      // tableOfContent={{ style: 'clerk' }}
      // breadcrumb={{ enabled: true }}
    >
      <title>{`${page.data.title} | StyleX`}</title>
      <DocsTitle>
        {page.data.title}{' '}
        {/* <code>
          '{slugs.join('/')}' vs '{page.data.slug}' is{' '}
          {slugs.length === 1 && slugs[0] === page.data.slug ? 'true' : 'false'}
        </code> */}
      </DocsTitle>
      {publishedDate && (
        <time
          dateTime={publishedDate.toISOString().slice(0, 10)}
          {...stylex.props(styles.date)}
        >
          {formatBlogDate(publishedDate)}
        </time>
      )}
      <div {...stylex.props(styles.authors)}>
        {authors.map((author) => (
          <a href={author.url} target="_blank" {...stylex.props(styles.author)}>
            <img {...stylex.props(styles.authorImage)} src={author.image_url} />
            <div {...stylex.props(styles.authorInfo)}>
              <div {...stylex.props(styles.authorName)}>{author.name}</div>
              <div {...stylex.props(styles.authorTitle)}>{author.title}</div>
            </div>
          </a>
        ))}
      </div>
      <DocsDescription>{page.data.description}</DocsDescription>
      <DocsBody>
        <MDX
          components={{
            ...mdxComponents,
          }}
        />
      </DocsBody>
    </DocsPage>
  );
}

export async function getConfig() {
  const pages = blogSource.getPages();
  const slugs = pages.map((page) => [page.data.slug]);

  return {
    render: 'static' as const,
    staticPaths: slugs,
  } as const;
}

const styles = stylex.create({
  fallbackContainer: {
    paddingBlock: '3rem',
    paddingInline: '1rem',
    textAlign: 'center',
  },
  fallbackTitle: {
    marginBlockEnd: '1rem',
    fontSize: '1.875rem',
    fontWeight: 700,
    lineHeight: '2.25rem',
    color: `${vars['--color-fd-foreground']}`,
  },
  fallbackDescription: {
    color: `${vars['--color-fd-muted-foreground']}`,
  },
  date: {
    // The parent <article> is a flex column with `gap: 24`. The big title's
    // line-height makes that 24px gap read as too much above the date, so pull the
    // date in with an equal negative block margin — leaving a tighter, uniform
    // ~12px of space between title, date, and authors.
    marginBlock: -12,
    fontSize: 14,
    color: `${vars['--color-fd-muted-foreground']}`,
  },
  authors: {
    display: 'flex',
    gap: 32,
  },
  author: {
    display: 'flex',
    gap: 16,
    textDecoration: 'none',
  },
  authorImage: {
    width: 48,
    height: 48,
    borderRadius: '50%',
  },
  authorInfo: {
    display: 'flex',
    flexDirection: 'column',
  },
  authorName: {
    fontSize: 16,
    fontWeight: 600,
    color: `${vars['--color-fd-primary']}`,
  },
  authorTitle: {
    fontSize: 14,
    color: `${vars['--color-fd-muted-foreground']}`,
  },
});

const AUTHORS = {
  mellyeliu: {
    name: 'Melissa Liu',
    title: 'StyleX Core',
    url: 'https://github.com/mellyeliu',
    image_url: mellyeliuImage,
  },
  necolas: {
    name: 'Nicolas Gallagher',
    title: 'StyleX Core',
    url: 'https://github.com/necolas',
    image_url: necolasImage,
  },
  nmn: {
    name: 'Naman Goel',
    title: 'StyleX Core',
    url: 'https://github.com/nmn',
    image_url: nmnImage,
  },
  vincentriemer: {
    name: 'Vincent Riemer',
    title: 'StyleX Core',
    url: 'https://github.com/vincentriemer',
    image_url: vincentriemerImage,
  },
};
