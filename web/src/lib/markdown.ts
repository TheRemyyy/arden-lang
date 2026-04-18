import { marked } from 'marked';
import createDOMPurify from 'dompurify';
import { JSDOM } from 'jsdom';

function slugifyHeading(text: string): string {
    const slug = text
        .toLowerCase()
        .replace(/[^\w]+/g, '-')
        .replace(/^-+|-+$/g, '');
    return slug || 'section';
}

function stripHtmlTags(text: string): string {
    return text.replace(/<[^>]+>/g, ' ');
}

type HeadingTokenLike = {
    text?: string;
    depth?: number;
    tokens?: unknown[];
};

function isHeadingTokenLike(value: unknown): value is HeadingTokenLike {
    return typeof value === 'object' && value !== null;
}

function resolveHeadingText(value: string | HeadingTokenLike): string {
    if (typeof value === 'string') {
        return value;
    }
    return typeof value.text === 'string' ? stripHtmlTags(value.text) : '';
}

function createRenderer() {
    const renderer = new marked.Renderer();
    const headingCounts = new Map<string, number>();

    // `marked` heading runtime args differ across versions; support both text and token forms.
    // @ts-expect-error runtime API is broader than the bundled type signature here
    renderer.heading = function (value: string | HeadingTokenLike, depth?: number) {
        const headingText = resolveHeadingText(value);
        const headingDepth =
            typeof depth === 'number'
                ? depth
                : isHeadingTokenLike(value) && typeof value.depth === 'number'
                    ? value.depth
                    : 1;
        const headingHtml =
            isHeadingTokenLike(value) && Array.isArray(value.tokens)
                ? marked.Parser.parseInline(value.tokens)
                : headingText;
        const baseSlug = slugifyHeading(stripHtmlTags(headingHtml));
        const count = headingCounts.get(baseSlug) ?? 0;
        headingCounts.set(baseSlug, count + 1);
        const headingId = count === 0 ? baseSlug : `${baseSlug}-${count}`;
        return `<h${headingDepth} id="${headingId}">${headingHtml}</h${headingDepth}>`;
    };

    return renderer;
}

export async function renderMarkdown(markdown: string): Promise<string> {
    const renderedHtml = await marked.parse(markdown, { renderer: createRenderer() });
    return sanitizeMarkdownHtml(renderedHtml);
}

function createSanitizer() {
    const htmlWindow =
        typeof window !== 'undefined'
            ? window
            : new JSDOM('<!doctype html><html><body></body></html>').window;
    const sanitizer = createDOMPurify(htmlWindow);

    sanitizer.addHook('uponSanitizeAttribute', (_node, data) => {
        const attributeName = data.attrName.toLowerCase();
        const attributeValue = data.attrValue.trim();

        if (attributeName.startsWith('on')) {
            data.keepAttr = false;
            return;
        }

        if (attributeName === 'href' && /^javascript:/i.test(attributeValue)) {
            data.attrValue = '#';
        }
    });

    return sanitizer;
}

const markdownSanitizer = createSanitizer();

export function sanitizeMarkdownHtml(html: string): string {
    return markdownSanitizer.sanitize(html, {
        USE_PROFILES: { html: true },
        FORBID_TAGS: ['script', 'style'],
        ALLOW_UNKNOWN_PROTOCOLS: false,
    });
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
