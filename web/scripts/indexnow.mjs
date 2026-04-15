import { promises as fs } from 'node:fs';
import path from 'node:path';

const siteUrl = 'https://www.arden-lang.dev';
const host = 'www.arden-lang.dev';
const key = '5f16d52efed72638de1a80329fd512fb';
const keyLocation = `${siteUrl}/${key}.txt`;
const endpoint = process.env.INDEXNOW_ENDPOINT ?? 'https://api.indexnow.org/indexnow';

function printUsage() {
  console.log(`Usage:
  npm run indexnow -- --url /docs/overview --url /changelog
  npm run indexnow -- --file urls.txt
  npm run indexnow -- --sitemap

Options:
  --url <url>     Relative or absolute URL to submit. Can be repeated.
  --file <path>   Text file with one URL per line.
  --sitemap       Submit every URL currently listed in public/sitemap.xml.
`);
}

function toAbsoluteUrl(value) {
  const trimmed = value.trim();
  if (!trimmed) return null;
  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
    return trimmed;
  }
  if (!trimmed.startsWith('/')) {
    return `${siteUrl}/${trimmed}`;
  }
  return `${siteUrl}${trimmed}`;
}

function isAllowedUrl(value) {
  try {
    const parsed = new URL(value);
    return parsed.protocol === 'https:' && parsed.hostname === host;
  } catch {
    return false;
  }
}

async function collectUrlsFromFile(filePath) {
  const contents = await fs.readFile(filePath, 'utf8');
  return contents
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map(toAbsoluteUrl)
    .filter(Boolean);
}

async function collectUrlsFromSitemap() {
  const sitemapPath = path.resolve(process.cwd(), 'public', 'sitemap.xml');
  const sitemap = await fs.readFile(sitemapPath, 'utf8');
  return Array.from(sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)).map((match) => match[1]);
}

function parseArgs(argv) {
  const urls = [];
  let filePath = null;
  let useSitemap = false;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--url') {
      const value = argv[index + 1];
      if (!value) {
        throw new Error('--url requires a value');
      }
      const normalized = toAbsoluteUrl(value);
      if (normalized) urls.push(normalized);
      index += 1;
      continue;
    }
    if (arg === '--file') {
      filePath = argv[index + 1];
      if (!filePath) {
        throw new Error('--file requires a path');
      }
      index += 1;
      continue;
    }
    if (arg === '--sitemap') {
      useSitemap = true;
      continue;
    }
    if (arg === '--help' || arg === '-h') {
      printUsage();
      process.exit(0);
    }
    throw new Error(`Unknown argument: ${arg}`);
  }

  return { urls, filePath, useSitemap };
}

async function resolveUrlList(argv) {
  const { urls, filePath, useSitemap } = parseArgs(argv);
  const collected = [...urls];

  if (filePath) {
    collected.push(...await collectUrlsFromFile(path.resolve(process.cwd(), filePath)));
  }

  if (useSitemap) {
    collected.push(...await collectUrlsFromSitemap());
  }

  return Array.from(new Set(collected)).filter((value) => isAllowedUrl(value));
}

async function submitIndexNow(urlList) {
  if (urlList.length === 0) {
    throw new Error('No URLs to submit. Use --url, --file, or --sitemap.');
  }

  const response = await fetch(endpoint, {
    method: 'POST',
    headers: {
      'content-type': 'application/json; charset=utf-8',
    },
    body: JSON.stringify({
      host,
      key,
      keyLocation,
      urlList,
    }),
  });

  const body = await response.text();
  if (!response.ok) {
    throw new Error(`IndexNow request failed (${response.status}): ${body}`);
  }

  console.log(`Submitted ${urlList.length} URL(s) to IndexNow.`);
  if (body.trim()) {
    console.log(body.trim());
  }
}

async function main() {
  const urlList = await resolveUrlList(process.argv.slice(2));
  await submitIndexNow(urlList);
}

await main();
