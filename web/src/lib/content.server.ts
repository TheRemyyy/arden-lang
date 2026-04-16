import { promises as fs } from 'node:fs';
import path from 'node:path';
import { FLATTENED_DOCS, getDocNeighbors, normalizeDocsPath } from './docs';
import { parseChangelogMarkdown, type ChangelogRelease } from './changelog';
import { renderMarkdown, rewriteInternalDocLinks } from './markdown';

export type PageHeading = {
    id: string;
    text: string;
    level: 2 | 3;
};

export type DocsPageData = {
    title: string;
    description: string;
    normalizedPath: string;
    content: string;
    headings: PageHeading[];
    lastUpdated: string;
    prevDoc: { title: string; path: string } | null;
    nextDoc: { title: string; path: string } | null;
};

export type ChangelogPageData = {
    title: string;
    description: string;
    content: string;
    lastUpdated: string;
    releases: ChangelogRelease[];
};

const REPO_ROOT = path.resolve(process.cwd(), '..');
const DOCS_ROOT = path.join(REPO_ROOT, 'docs');
const CHANGELOG_PATH = path.join(REPO_ROOT, 'CHANGELOG.md');

function stripMarkdown(markdown: string): string {
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

function extractDescription(markdown: string): string {
    const title = extractTitle(markdown, 'Arden documentation');
    const blocks = markdown
        .split(/\n\s*\n/)
        .map((block) => stripMarkdown(block))
        .filter((block) => block.length > 0);

    const firstParagraph = blocks.find((block) => {
        if (block === title) return false;
        if (block.startsWith('#')) return false;
        if (!/[a-z].*[.?!]/i.test(block) && block.split(' ').length < 8) return false;
        return true;
    });

    return firstParagraph?.slice(0, 180) ?? 'Arden documentation.';
}

function extractTitle(markdown: string, fallback: string): string {
    const match = markdown.match(/^#\s+(.+)$/m);
    return match?.[1]?.trim() ?? fallback;
}

function extractHeadingsFromHtml(html: string): PageHeading[] {
    return Array.from(html.matchAll(/<(h[23]) id="([^"]+)">([\s\S]*?)<\/h[23]>/g)).map((match) => ({
        level: match[1] === 'h2' ? 2 : 3,
        id: match[2],
        text: match[3].replace(/[<>]/g, '').trim(),
    }));
}

async function loadMarkdownFile(filePath: string): Promise<string> {
    return fs.readFile(filePath, 'utf8');
}

export async function loadDocPage(urlPathname: string): Promise<DocsPageData> {
    const normalizedPath = normalizeDocsPath(urlPathname);
    const docPath = normalizedPath.replace(/^\/docs\/?/, '');
    const markdownPath = path.join(DOCS_ROOT, `${docPath}.md`);
    const stat = await fs.stat(markdownPath);
    const markdown = await loadMarkdownFile(markdownPath);
    const rendered = await renderMarkdown(markdown);
    const content = rewriteInternalDocLinks(rendered, normalizedPath);
    const fallbackTitle =
        FLATTENED_DOCS.find((item) => item.path === normalizedPath)?.title ?? 'Documentation';

    return {
        title: extractTitle(markdown, fallbackTitle),
        description: extractDescription(markdown),
        normalizedPath,
        content,
        headings: extractHeadingsFromHtml(content),
        lastUpdated: stat.mtime.toISOString(),
        ...getDocNeighbors(normalizedPath),
    };
}

export async function loadChangelogPage(): Promise<ChangelogPageData> {
    const markdown = await loadMarkdownFile(CHANGELOG_PATH);
    const stat = await fs.stat(CHANGELOG_PATH);
    return {
        title: 'Changelog',
        description: 'Tracking the latest improvements to Arden.',
        content: await renderMarkdown(markdown),
        lastUpdated: stat.mtime.toISOString(),
        releases: await parseChangelogMarkdown(markdown),
    };
}
