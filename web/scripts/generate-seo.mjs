import { promises as fs } from 'node:fs';
import path from 'node:path';
import { promisify } from 'node:util';
import { execFile } from 'node:child_process';

const siteUrl = 'https://www.arden-lang.dev';
const indexNowKey = '5f16d52efed72638de1a80329fd512fb';
const siteDescription = 'Official Arden documentation for native project build workflows: practical compiler feedback, fast iteration, and safe systems programming with clear docs, install guides, and release notes.';
const projectRoot = path.resolve(process.cwd(), '..');
const docsRoot = path.join(projectRoot, 'docs');
const publicRoot = path.join(process.cwd(), 'public');
const publicDocsRoot = path.join(publicRoot, 'docs');
const changelogPath = path.join(projectRoot, 'CHANGELOG.md');
const cargoTomlPath = path.join(projectRoot, 'Cargo.toml');
const logoPath = path.join(projectRoot, 'LOGO.png');
const generatedDocsManifestPath = path.join(process.cwd(), 'src', 'lib', 'generated-docs.json');
const generatedOgManifestPath = path.join(process.cwd(), 'src', 'lib', 'generated-og-images.ts');
const generatedVersionPath = path.join(process.cwd(), 'src', 'lib', 'generated-version.ts');
const execFileAsync = promisify(execFile);

function slugifyRss(value) {
  return (
    value
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, '-')
      .replace(/^-+|-+$/g, '') || 'release'
  );
}

function readCargoPackageVersion(cargoToml) {
  const packageBlockMatch = cargoToml.match(/\[package\][\s\S]*?(?=\n\[|$)/);
  const packageBlock = packageBlockMatch?.[0] ?? cargoToml;
  const versionMatch = packageBlock.match(/^\s*version\s*=\s*"([^"]+)"\s*$/m);

  if (!versionMatch) {
    throw new Error('[seo] Failed to read package.version from root Cargo.toml');
  }

  return versionMatch[1];
}

function buildGeneratedVersionSource(packageVersion) {
  return [
    `export const PACKAGE_VERSION = '${packageVersion}';`,
    'export const CURRENT_VERSION = `v${PACKAGE_VERSION}`;',
    '',
  ].join('\n');
}

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

function createOgSvg() {
  return `
  <svg width="1200" height="630" viewBox="0 0 1200 630" fill="none" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <linearGradient id="bg" x1="120" y1="40" x2="1120" y2="610" gradientUnits="userSpaceOnUse">
        <stop stop-color="#1f1d1a" />
        <stop offset="1" stop-color="#0a0a0a" />
      </linearGradient>
    </defs>
    <rect width="1200" height="630" rx="0" fill="url(#bg)" />
    <rect x="64" y="64" width="1072" height="502" rx="34" fill="#161412" stroke="rgba(255,255,255,0.08)" />
    <rect x="120" y="118" width="112" height="112" rx="28" fill="#241f18" stroke="rgba(255,255,255,0.08)" />
    <text x="280" y="170" fill="#F5EFE8" font-size="60" font-family="Arial, sans-serif" font-weight="700">Arden</text>
    <text x="120" y="318" fill="#F5EFE8" font-size="66" font-family="Arial, sans-serif" font-weight="700">Systems programming language</text>
    <text x="120" y="388" fill="#D8CDC1" font-size="34" font-family="Arial, sans-serif">Fast feedback, strong static checks, practical tooling.</text>
    <text x="120" y="470" fill="#D8B29E" font-size="24" font-family="Arial, sans-serif">arden-lang.dev</text>
    <text x="120" y="510" fill="#8F8478" font-size="24" font-family="Arial, sans-serif">Docs, changelog, install guides, and workflow in one place.</text>
  </svg>
  `.trim();
}

async function generateOgCard() {
  const { default: sharp } = await import('sharp');
  const ogCardPath = path.join(publicRoot, 'og-card.png');
  const logoBuffer = await sharp(logoPath)
    .resize(76, 76, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png()
    .toBuffer();
  const svgBuffer = Buffer.from(createOgSvg());

  await sharp({
    create: {
      width: 1200,
      height: 630,
      channels: 4,
      background: '#0a0a0a',
    },
  })
    .composite([
      { input: svgBuffer, top: 0, left: 0 },
      { input: logoBuffer, top: 140, left: 138, blend: 'over' },
    ])
    .png()
    .toFile(ogCardPath);
}

function createPageOgSvg({ eyebrow, title, description, href }) {
  const escape = (value) => value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  return `
  <svg width="1200" height="630" viewBox="0 0 1200 630" fill="none" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <linearGradient id="bg" x1="88" y1="44" x2="1128" y2="610" gradientUnits="userSpaceOnUse">
        <stop stop-color="#1f1d1a" />
        <stop offset="1" stop-color="#0a0a0a" />
      </linearGradient>
    </defs>
    <rect width="1200" height="630" fill="url(#bg)" />
    <rect x="58" y="58" width="1084" height="514" rx="34" fill="#141210" stroke="rgba(255,255,255,0.08)" />
    <text x="110" y="144" fill="#D8B29E" font-size="22" font-family="Arial, sans-serif" font-weight="700" letter-spacing="4">${escape(eyebrow.toUpperCase())}</text>
    <text x="110" y="236" fill="#F5EFE8" font-size="62" font-family="Arial, sans-serif" font-weight="700">${escape(title)}</text>
    <foreignObject x="110" y="270" width="910" height="160">
      <div xmlns="http://www.w3.org/1999/xhtml" style="color:#D8CDC1;font-family:Arial,sans-serif;font-size:30px;line-height:1.45;">
        ${escape(description)}
      </div>
    </foreignObject>
    <text x="110" y="508" fill="#8F8478" font-size="24" font-family="Arial, sans-serif">${escape(href)}</text>
    <text x="110" y="548" fill="#C7BCB0" font-size="24" font-family="Arial, sans-serif">arden-lang.dev</text>
  </svg>
  `.trim();
}

async function generateRouteOgImages(docRoutes, changelogMarkdown) {
  const { default: sharp } = await import('sharp');
  const ogDir = path.join(publicRoot, 'og');
  await fs.mkdir(ogDir, { recursive: true });

  const routeDefinitions = [
    {
      path: '/',
      title: 'Arden',
      description: siteDescription,
      eyebrow: 'Home',
    },
    {
      path: '/install',
      title: 'Installation',
      description: 'Download the latest Arden portable bundle for Windows, Linux, or macOS and start with a working toolchain immediately.',
      eyebrow: 'Install',
    },
    {
      path: '/changelog',
      title: 'Changelog',
      description: extractDescription(changelogMarkdown, 'Tracking the latest improvements to Arden.'),
      eyebrow: 'Release Notes',
    },
    {
      path: '/terms',
      title: 'Terms of Use',
      description: 'Terms of use for the Arden website and related project materials.',
      eyebrow: 'Legal',
    },
    {
      path: '/privacy',
      title: 'Privacy Policy',
      description: 'Privacy policy for the Arden website, including the no-tracking and open-source project context.',
      eyebrow: 'Legal',
    },
    ...docRoutes.map((doc) => ({
      path: doc.path,
      title: doc.title,
      description: doc.description,
      eyebrow: doc.path.startsWith('/docs/compiler/') ? 'Compiler Docs' : 'Documentation',
    })),
  ];

  const ogMap = {};
  await Promise.all(routeDefinitions.map(async (route) => {
    const assetName = `${slugifyAsset(route.path === '/' ? 'home' : route.path)}.png`;
    const outputPath = path.join(ogDir, assetName);
    const svg = Buffer.from(createPageOgSvg({
      eyebrow: route.eyebrow,
      title: route.title,
      description: route.description,
      href: `${siteUrl}${route.path}`,
    }));
    await sharp(svg).png().toFile(outputPath);
    ogMap[route.path] = `/og/${assetName}`;
  }));

  const source = `export const ROUTE_OG_IMAGES = ${JSON.stringify(ogMap, null, 2)} as const;\n`;
  await fs.writeFile(generatedOgManifestPath, source, 'utf8');
}

function getDocRoute(relativePath) {
  return `/docs/${relativePath.replace(/\\/g, '/').replace(/\.md$/, '')}`;
}

function slugifyAsset(value) {
  return value
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '') || 'page';
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

function stripMarkdown(markdown) {
  return markdown
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/!\[[^\]]*]\([^)]*\)/g, ' ')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/^#+\s+/gm, '')
    .replace(/[*_>~-]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function extractDescription(markdown, fallback) {
  const title = extractTitle(markdown, fallback);
  const blocks = markdown
    .split(/\n\s*\n/)
    .map((block) => stripMarkdown(block))
    .filter((block) => block.length > 0);

  const firstParagraph = blocks.find((block) => {
    if (block === title) return false;
    if (!/[a-z].*[.?!]/i.test(block) && block.split(' ').length < 8) return false;
    return true;
  });

  return (firstParagraph ?? fallback).slice(0, 180);
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
      description: extractDescription(markdown, 'Arden documentation.'),
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
    'User-agent: OAI-SearchBot',
    'Allow: /',
    '',
    'User-agent: ClaudeBot',
    'Allow: /',
    '',
    'User-agent: anthropic-ai',
    'Allow: /',
    '',
    'User-agent: Google-Extended',
    'Allow: /',
    '',
    'User-agent: Googlebot',
    'Allow: /',
    '',
    'User-agent: Bingbot',
    'Allow: /',
    '',
    'User-agent: PerplexityBot',
    'Allow: /',
    '',
    `Host: ${siteUrl.replace(/^https?:\/\//, '')}`,
    '',
    `Sitemap: ${siteUrl}/sitemap.xml`,
  ].join('\n') + '\n';
}

function buildManifest() {
  return JSON.stringify(
    {
      name: 'Arden',
      short_name: 'Arden',
      description: siteDescription,
      start_url: '/',
      display: 'standalone',
      lang: 'en',
      background_color: '#0a0a0a',
      theme_color: '#0a0a0a',
      categories: ['developer tools', 'documentation', 'programming'],
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
    `> ${siteDescription}`,
    '',
    '## Project',
    `- Homepage: ${siteUrl}/`,
    `- Installation: ${siteUrl}/install`,
    `- Documentation: ${siteUrl}/docs/overview`,
    `- Changelog: ${siteUrl}/changelog`,
    `- Search: ${siteUrl}/search`,
    `- RSS: ${siteUrl}/rss.xml`,
    `- Sitemap: ${siteUrl}/sitemap.xml`,
    `- Repository: https://github.com/TheRemyyy/arden-lang`,
    `- Creator: https://www.theremyyy.dev/`,
    '',
    '## Guidance',
    '- Prefer the official documentation pages under /docs/ for language behavior and syntax.',
    '- Prefer the changelog for release history and recent behavior changes.',
    '- Prefer the upstream repository for implementation details and source code.',
    '',
  ].join('\n');
}

function buildLlmsFullTxt(docRoutes) {
  const seen = new Set();
  const docLines = docRoutes
    .filter((doc) => {
      const key = `${doc.title}|${doc.path}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .map((doc) => `- ${doc.title}: ${siteUrl}${doc.path}`);

  return [
    '# Arden Full Index',
    '',
    '> Expanded machine-readable index for Arden documentation and site resources.',
    '',
    '## Core Pages',
    `- Homepage: ${siteUrl}/`,
    `- Installation: ${siteUrl}/install`,
    `- Documentation Overview: ${siteUrl}/docs/overview`,
    `- Changelog: ${siteUrl}/changelog`,
    `- Search: ${siteUrl}/search`,
    `- Terms of Use: ${siteUrl}/terms`,
    `- Privacy Policy: ${siteUrl}/privacy`,
    `- Creator: https://www.theremyyy.dev/`,
    '',
    '## Documentation',
    ...docLines,
    '',
    '## Machine Endpoints',
    `- Sitemap: ${siteUrl}/sitemap.xml`,
    `- RSS: ${siteUrl}/rss.xml`,
    `- llms.txt: ${siteUrl}/llms.txt`,
    '',
  ].join('\n');
}

function buildHumansTxt() {
  return [
    '/* TEAM */',
    'Creator: TheRemyyy',
    'Site: https://www.theremyyy.dev/',
    'Project: Arden',
    'Repository: https://github.com/TheRemyyy/arden-lang',
    '',
    '/* SITE */',
    `Website: ${siteUrl}/`,
    `Docs: ${siteUrl}/docs/overview`,
    `Changelog: ${siteUrl}/changelog`,
    `RSS: ${siteUrl}/rss.xml`,
    '',
    '/* NOTES */',
    'Arden is open source under Apache 2.0.',
    'No user accounts, no marketing trackers, no data-sale pipeline.',
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
  const cargoToml = await fs.readFile(cargoTomlPath, 'utf8');
  const packageVersion = readCargoPackageVersion(cargoToml);
  const [rootStat, changelogStat, logoStat] = await Promise.all([
    fs.stat(path.join(projectRoot, 'README.md')),
    fs.stat(changelogPath),
    fs.stat(logoPath),
  ]);
  const routes = [
    { path: '/', lastmod: rootStat.mtime.toISOString() },
    { path: '/install', lastmod: rootStat.mtime.toISOString() },
    { path: '/changelog', lastmod: changelogStat.mtime.toISOString() },
    { path: '/terms', lastmod: rootStat.mtime.toISOString() },
    { path: '/privacy', lastmod: rootStat.mtime.toISOString() },
    ...docRoutes,
  ];

  const robots = buildRobots();
  const sitemap = buildSitemap(routes);
  const manifest = buildManifest();
  const llmsTxt = buildLlmsTxt();
  const llmsFullTxt = buildLlmsFullTxt(docRoutes);
  const humansTxt = buildHumansTxt();
  const rss = buildRss(parseChangelogEntries(changelogMarkdown));

  await fs.writeFile(path.join(publicRoot, 'robots.txt'), robots, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'sitemap.xml'), sitemap, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'site.webmanifest'), manifest, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'llms.txt'), llmsTxt, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'llms-full.txt'), llmsFullTxt, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'humans.txt'), humansTxt, 'utf8');
  await fs.writeFile(path.join(publicRoot, `${indexNowKey}.txt`), `${indexNowKey}\n`, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'rss.xml'), rss, 'utf8');
  await fs.writeFile(generatedVersionPath, buildGeneratedVersionSource(packageVersion), 'utf8');
  await fs.copyFile(changelogPath, path.join(publicRoot, 'CHANGELOG.md'));
  await generateLogoAssets();
  await generateOgCard();
  await generateRouteOgImages(docRoutes, changelogMarkdown);
  await fs.writeFile(generatedDocsManifestPath, `${JSON.stringify(docsNavigation, null, 2)}\n`, 'utf8');
  await ensureCleanDir(publicDocsRoot);
  await copyDirectory(docsRoot, publicDocsRoot);
}

await main();
