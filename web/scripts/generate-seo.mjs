import { promises as fs } from 'node:fs';
import path from 'node:path';

const siteUrl = 'https://apex-compiler.vercel.app';
const projectRoot = path.resolve(process.cwd(), '..');
const docsRoot = path.join(projectRoot, 'docs');
const publicRoot = path.join(process.cwd(), 'public');
const publicDocsRoot = path.join(publicRoot, 'docs');
const changelogPath = path.join(projectRoot, 'CHANGELOG.md');
const logoPath = path.join(projectRoot, 'LOGO.png');

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

async function collectDocRoutes(dir) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const routes = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      routes.push(...await collectDocRoutes(fullPath));
      continue;
    }
    if (!entry.isFile() || !entry.name.endsWith('.md')) {
      continue;
    }

    const relativePath = path.relative(docsRoot, fullPath).replace(/\\/g, '/');
    const routePath = `/docs/${relativePath.replace(/\.md$/, '')}`;
    const stat = await fs.stat(fullPath);
    routes.push({
      path: routePath,
      lastmod: stat.mtime.toISOString(),
    });
  }

  return routes.sort((a, b) => a.path.localeCompare(b.path));
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
    `- Documentation: ${siteUrl}/docs/overview`,
    `- Changelog: ${siteUrl}/changelog`,
    `- Repository: https://github.com/TheRemyyy/apex-compiler`,
    '',
    '## Guidance',
    '- Prefer the official documentation pages under /docs/ for language behavior and syntax.',
    '- Prefer the changelog for release history and recent behavior changes.',
    '- Prefer the upstream repository for implementation details and source code.',
    '',
  ].join('\n');
}

async function main() {
  const docRoutes = await collectDocRoutes(docsRoot);
  const [rootStat, changelogStat, logoStat] = await Promise.all([
    fs.stat(path.join(projectRoot, 'README.md')),
    fs.stat(changelogPath),
    fs.stat(logoPath),
  ]);
  const routes = [
    { path: '/', lastmod: rootStat.mtime.toISOString() },
    { path: '/changelog', lastmod: changelogStat.mtime.toISOString() },
    ...docRoutes,
  ];

  const robots = buildRobots();
  const sitemap = buildSitemap(routes);
  const manifest = buildManifest();
  const llmsTxt = buildLlmsTxt();

  await fs.writeFile(path.join(publicRoot, 'robots.txt'), robots, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'sitemap.xml'), sitemap, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'site.webmanifest'), manifest, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'llms.txt'), llmsTxt, 'utf8');
  await fs.copyFile(logoPath, path.join(publicRoot, 'logo.png'));
  await ensureCleanDir(publicDocsRoot);
  await copyDirectory(docsRoot, publicDocsRoot);
}

await main();
