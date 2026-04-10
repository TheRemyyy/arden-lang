import { promises as fs } from 'node:fs';
import path from 'node:path';
import { promisify } from 'node:util';
import { execFile } from 'node:child_process';
import { slugifyRss } from '../src/lib/rss.ts';

const siteUrl = 'https://www.arden-lang.dev';
const projectRoot = path.resolve(process.cwd(), '..');
const docsRoot = path.join(projectRoot, 'docs');
const publicRoot = path.join(process.cwd(), 'public');
const publicDocsRoot = path.join(publicRoot, 'docs');
const changelogPath = path.join(projectRoot, 'CHANGELOG.md');
const logoPath = path.join(projectRoot, 'LOGO.png');
const generatedDocsManifestPath = path.join(process.cwd(), 'src', 'lib', 'generated-docs.json');
const execFileAsync = promisify(execFile);

const SECTION_LABELS = {
  basics: 'Basics',
  compiler: 'Compiler',
  advanced: 'Advanced',
  features: 'Features',
  stdlib: 'Standard Library',
  getting_started: 'Getting Started',
};

const SECTION_ORDER = ['__root__', 'getting_started', 'basics', 'features', 'stdlib', 'advanced', 'compiler'];

const DOC_ORDER = [
  '/docs/overview',
  '/docs/getting_started/installation',
  '/docs/getting_started/quick_start',
  '/docs/getting_started/editor_setup',
  '/docs/basics/syntax',
  '/docs/basics/variables',
  '/docs/basics/types',
  '/docs/basics/control_flow',
  '/docs/features/functions',
  '/docs/features/classes',
  '/docs/features/interfaces',
  '/docs/features/enums',
  '/docs/features/ranges',
  '/docs/features/modules',
  '/docs/features/projects',
  '/docs/features/projects/README',
  '/docs/features/testing',
  '/docs/stdlib/overview',
  '/docs/stdlib/math',
  '/docs/stdlib/string',
  '/docs/stdlib/time',
  '/docs/stdlib/args',
  '/docs/stdlib/collections',
  '/docs/stdlib/io',
  '/docs/stdlib/system',
  '/docs/advanced/ownership',
  '/docs/advanced/generics',
  '/docs/advanced/async',
  '/docs/advanced/error_handling',
  '/docs/advanced/memory_management',
  '/docs/compiler/cli',
  '/docs/compiler/architecture',
  '/docs/projects',
];

async function ensureCleanDir(dir) {
  await fs.rm(dir, { recursive: true, force: true });
  await fs.mkdir(dir, { recursive: true });
}

async function copyDirectory(sourceDir, targetDir) {
    const entries = await fs.readdir(sourceDir, { withFileTypes: true });
    await fs.mkdir(targetDir, { recursive: true });

  for (const entry of entries) {
    const sourcePath = path.join(sourceDir, entry.name);
    const targetPath = path.join(targetDir, entry.name);

    if (entry.isDirectory()) {
      await copyDirectory(sourcePath, targetPath);
      continue;
    }

    if (entry.isFile()) {
      await fs.copyFile(sourcePath, targetPath);
    }
  }
}

async function generateLogoDerivative(sourcePath, targetPath, size) {
  try {
    await execFileAsync('magick', [
      sourcePath,
      '-resize',
      `${size}x${size}`,
      '-background',
      'none',
      '-gravity',
      'center',
      '-extent',
      `${size}x${size}`,
      targetPath,
    ]);
    return;
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.warn(`[seo] magick resize failed for ${path.basename(targetPath)}, falling back to sharp: ${errorMessage}`);
  }

  try {
    const { default: sharp } = await import('sharp');
    await sharp(sourcePath)
      .resize(size, size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
      .toFile(targetPath);
  } catch (error) {
    throw new Error(
      `[seo] Failed to generate ${path.basename(targetPath)}: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

async function generateLogoAssets() {
  await fs.copyFile(logoPath, path.join(publicRoot, 'logo.png'));
  const derivativeTargets = [
    { path: path.join(publicRoot, 'logo-mark-64.png'), size: 64 },
    { path: path.join(publicRoot, 'favicon-32.png'), size: 32 },
    { path: path.join(publicRoot, 'favicon.png'), size: 512 },
    { path: path.join(publicRoot, 'apple-touch-icon.png'), size: 180 },
  ];

  await Promise.all(
    derivativeTargets.map((target) => generateLogoDerivative(logoPath, target.path, target.size)),
  );
}

function getDocRoute(relativePath) {
  return `/docs/${relativePath.replace(/\\/g, '/').replace(/\.md$/, '')}`;
}

function humanizeSlug(value) {
  return value
    .replace(/[-_]+/g, ' ')
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function getFallbackTitle(relativePath) {
  const baseName = path.basename(relativePath, '.md');
  if (baseName === 'README') {
    return `${humanizeSlug(path.basename(path.dirname(relativePath)))} Guide`;
  }

  return humanizeSlug(baseName);
}

function extractTitle(markdown, fallback) {
  const match = markdown.match(/^#\s+(.+)$/m);
  return match?.[1]?.trim() ?? fallback;
}

function getOrderIndex(routePath) {
  const explicitIndex = DOC_ORDER.indexOf(routePath);
  return explicitIndex === -1 ? Number.MAX_SAFE_INTEGER : explicitIndex;
}

function compareDocs(leftDoc, rightDoc) {
  const orderDelta = getOrderIndex(leftDoc.path) - getOrderIndex(rightDoc.path);
  if (orderDelta !== 0) {
    return orderDelta;
  }

  return leftDoc.path.localeCompare(rightDoc.path);
}

async function collectDocMetadata(dir) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const routes = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      routes.push(...await collectDocMetadata(fullPath));
      continue;
    }
    if (!entry.isFile() || !entry.name.endsWith('.md')) {
      continue;
    }

    const relativePath = path.relative(docsRoot, fullPath).replace(/\\/g, '/');
    const routePath = getDocRoute(relativePath);
    const stat = await fs.stat(fullPath);
    const markdown = await fs.readFile(fullPath, 'utf8');
    const fallbackTitle = getFallbackTitle(relativePath);
    routes.push({
      path: routePath,
      relativePath,
      title: extractTitle(markdown, fallbackTitle),
      lastmod: stat.mtime.toISOString(),
    });
  }

  return routes.sort(compareDocs);
}

function buildDocsNavigation(docRoutes) {
  const rootDocs = docRoutes
    .filter((doc) => !doc.relativePath.includes('/'))
    .sort(compareDocs)
    .map(({ title, path: routePath }) => ({ title, path: routePath }));

  const groupedDocs = new Map();

  for (const doc of docRoutes) {
    const [sectionKey] = doc.relativePath.split('/');
    if (!doc.relativePath.includes('/')) {
      continue;
    }

    const items = groupedDocs.get(sectionKey) ?? [];
    items.push({ title: doc.title, path: doc.path });
    groupedDocs.set(sectionKey, items);
  }

  const orderedSectionKeys = Array.from(groupedDocs.keys()).sort((leftKey, rightKey) => {
    const leftIndex = SECTION_ORDER.indexOf(leftKey);
    const rightIndex = SECTION_ORDER.indexOf(rightKey);

    if (leftIndex !== -1 || rightIndex !== -1) {
      return (leftIndex === -1 ? Number.MAX_SAFE_INTEGER : leftIndex)
        - (rightIndex === -1 ? Number.MAX_SAFE_INTEGER : rightIndex);
    }

    return leftKey.localeCompare(rightKey);
  });

  const groupedSections = orderedSectionKeys.map((sectionKey) => ({
    title: SECTION_LABELS[sectionKey] ?? humanizeSlug(sectionKey),
    items: groupedDocs.get(sectionKey).sort(compareDocs).map((item) => ({
      ...item,
      title: item.path.endsWith('/README') ? `${item.title} Guide` : item.title,
    })),
  }));

  return [...rootDocs, ...groupedSections];
}

function buildSitemap(routes) {
  const entries = routes.map((route) => {
    const priority = route.path === '/' ? '1.0' : route.path === '/docs/overview' ? '0.9' : route.path === '/changelog' ? '0.8' : '0.7';
    const changefreq = route.path === '/' ? 'weekly' : route.path.startsWith('/docs/') ? 'monthly' : 'weekly';
    return `  <url>
    <loc>${siteUrl}${route.path}</loc>
    <lastmod>${route.lastmod}</lastmod>
    <changefreq>${changefreq}</changefreq>
    <priority>${priority}</priority>
  </url>`;
  });

  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${entries.join('\n')}
</urlset>
`;
}

function buildRobots() {
  return [
    'User-agent: *',
    'Allow: /',
    '',
    'User-agent: GPTBot',
    'Allow: /',
    '',
    'User-agent: ChatGPT-User',
    'Allow: /',
    '',
    'User-agent: Googlebot',
    'Allow: /',
    '',
    `Sitemap: ${siteUrl}/sitemap.xml`,
  ].join('\n') + '\n';
}

function buildManifest() {
  return JSON.stringify(
    {
      name: 'Arden',
      short_name: 'Arden',
      description: 'Official documentation for Arden, a systems programming language targeting LLVM.',
      start_url: '/',
      display: 'standalone',
      background_color: '#0a0a0a',
      theme_color: '#0a0a0a',
      icons: [
        {
          src: '/favicon-32.png',
          sizes: '32x32',
          type: 'image/png',
        },
        {
          src: '/apple-touch-icon.png',
          sizes: '180x180',
          type: 'image/png',
        },
      ],
    },
    null,
    2,
  ) + '\n';
}

function buildLlmsTxt() {
  return [
    '# Arden',
    '',
    '> Official documentation for Arden, a systems programming language targeting LLVM.',
    '',
    '## Project',
    `- Homepage: ${siteUrl}/`,
    `- Installation: ${siteUrl}/install`,
    `- Documentation: ${siteUrl}/docs/overview`,
    `- Changelog: ${siteUrl}/changelog`,
    `- Repository: https://github.com/TheRemyyy/arden-lang`,
    '',
    '## Guidance',
    '- Prefer the official documentation pages under /docs/ for language behavior and syntax.',
    '- Prefer the changelog for release history and recent behavior changes.',
    '- Prefer the upstream repository for implementation details and source code.',
    '',
  ].join('\n');
}

function escapeXml(value) {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

function parseChangelogEntries(markdown) {
  const headings = Array.from(markdown.matchAll(/^##\s+\[(.+?)\](?: - (.*?))?(?: - (\d{4}-\d{2}-\d{2}))?$/gm));

  return headings.map((match, index) => {
    const title = match[1].trim();
    const subtitle = match[2]?.trim() ?? '';
    const date = match[3] ?? new Date().toISOString().slice(0, 10);
    const bodyStart = (match.index ?? 0) + match[0].length;
    const bodyEnd = headings[index + 1]?.index ?? markdown.length;
    const body = markdown.slice(bodyStart, bodyEnd).trim();
    const firstBullet = body
      .split('\n')
      .map((line) => line.trim())
      .find((line) => line.startsWith('- '))
      ?.replace(/^- /, '') ?? 'Release notes updated.';

    return {
      title: title === 'Unreleased' ? 'Unreleased' : `v${title}`,
      link: `${siteUrl}/changelog#${slugifyRss(title)}`,
      description: subtitle ? `${subtitle}. ${firstBullet}` : firstBullet,
      pubDate: new Date(`${date}T00:00:00Z`).toUTCString(),
    };
  });
}

function buildRss(entries) {
  const items = entries.slice(0, 20).map((entry) => `  <item>
    <title>${escapeXml(entry.title)}</title>
    <link>${escapeXml(entry.link)}</link>
    <guid>${escapeXml(entry.link)}</guid>
    <description>${escapeXml(entry.description)}</description>
    <pubDate>${entry.pubDate}</pubDate>
  </item>`);

  return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
  <title>Arden Changelog</title>
  <link>${siteUrl}/changelog</link>
  <description>Release notes and changelog updates for Arden.</description>
  <language>en-us</language>
${items.join('\n')}
</channel>
</rss>
`;
}

async function main() {
  const docRoutes = await collectDocMetadata(docsRoot);
  const docsNavigation = buildDocsNavigation(docRoutes);
  const changelogMarkdown = await fs.readFile(changelogPath, 'utf8');
  const [rootStat, changelogStat, logoStat] = await Promise.all([
    fs.stat(path.join(projectRoot, 'README.md')),
    fs.stat(changelogPath),
    fs.stat(logoPath),
  ]);
  const routes = [
    { path: '/', lastmod: rootStat.mtime.toISOString() },
    { path: '/install', lastmod: rootStat.mtime.toISOString() },
    { path: '/changelog', lastmod: changelogStat.mtime.toISOString() },
    ...docRoutes,
  ];

  const robots = buildRobots();
  const sitemap = buildSitemap(routes);
  const manifest = buildManifest();
  const llmsTxt = buildLlmsTxt();
  const rss = buildRss(parseChangelogEntries(changelogMarkdown));

  await fs.writeFile(path.join(publicRoot, 'robots.txt'), robots, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'sitemap.xml'), sitemap, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'site.webmanifest'), manifest, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'llms.txt'), llmsTxt, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'rss.xml'), rss, 'utf8');
  await fs.copyFile(changelogPath, path.join(publicRoot, 'CHANGELOG.md'));
  await generateLogoAssets();
  await fs.writeFile(generatedDocsManifestPath, `${JSON.stringify(docsNavigation, null, 2)}\n`, 'utf8');
  await ensureCleanDir(publicDocsRoot);
  await copyDirectory(docsRoot, publicDocsRoot);
}

await main();
