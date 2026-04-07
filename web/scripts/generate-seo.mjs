import { promises as fs } from 'node:fs';
import path from 'node:path';

const siteUrl = 'https://apex-compiler.vercel.app';
const projectRoot = path.resolve(process.cwd(), '..');
const docsRoot = path.join(projectRoot, 'docs');
const publicRoot = path.join(process.cwd(), 'public');

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
    routes.push(routePath);
  }

  return routes.sort();
}

function buildSitemap(routes) {
  const entries = routes.map((route) => {
    const priority = route === '/' ? '1.0' : route === '/docs/overview' ? '0.9' : route === '/changelog' ? '0.8' : '0.7';
    const changefreq = route === '/' ? 'weekly' : route.startsWith('/docs/') ? 'monthly' : 'weekly';
    return `  <url>
    <loc>${siteUrl}${route}</loc>
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

async function main() {
  const docRoutes = await collectDocRoutes(docsRoot);
  const routes = ['/', '/changelog', ...docRoutes];

  const robots = `User-agent: *\nAllow: /\nSitemap: ${siteUrl}/sitemap.xml\n`;
  const sitemap = buildSitemap(routes);

  await fs.writeFile(path.join(publicRoot, 'robots.txt'), robots, 'utf8');
  await fs.writeFile(path.join(publicRoot, 'sitemap.xml'), sitemap, 'utf8');
}

await main();
