import { marked } from 'marked';

function slugifyHeading(text: string): string {
    const slug = text
        .toLowerCase()
        .replace(/[^\w]+/g, '-')
        .replace(/^-+|-+$/g, '');
    return slug || 'section';
}

function createRenderer() {
    const renderer = new marked.Renderer();
    const headingCounts = new Map<string, number>();

    // marked's heading signature is broader than the typed package currently exposes.
    // @ts-ignore typed package lags the runtime API here
    renderer.heading = (text: string, depth: number) => {
        const baseSlug = slugifyHeading(text);
        const count = headingCounts.get(baseSlug) ?? 0;
        headingCounts.set(baseSlug, count + 1);
        const headingId = count === 0 ? baseSlug : `${baseSlug}-${count}`;
        return `<h${depth} id="${headingId}">${text}</h${depth}>`;
    };

    return renderer;
}

export async function renderMarkdown(markdown: string): Promise<string> {
    return marked.parse(markdown, { renderer: createRenderer() });
}

export function sanitizeMarkdownHtml(html: string): string {
    return html;
}

export function rewriteInternalDocLinks(html: string, currentPath: string): string {
    const baseDocPath = `${currentPath}.md`;
    const isExternalHref = (href: string) => /^(?:[a-z][a-z0-9+.-]*:|\/\/)/i.test(href);
    const baseUrl = new URL(baseDocPath, 'https://arden.local');

    return html.replace(/<a\b([^>]*)href="([^"]+)"([^>]*)>/g, (full, before, rawHref, after) => {
        if (!rawHref || rawHref.startsWith('#') || isExternalHref(rawHref)) {
            return full;
        }

        const resolved = new URL(rawHref, baseUrl);
        let path = resolved.pathname;

        if (path.endsWith('.md')) {
            path = path.slice(0, -3);
        }

        if (path.startsWith('/docs/docs/')) {
            path = path.replace('/docs/docs/', '/docs/');
        }

        const finalHref = `${path}${resolved.hash}`;
        return `<a${before}href="${finalHref}"${after}>`;
    });
}
