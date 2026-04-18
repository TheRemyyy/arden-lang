import { describe, expect, it } from 'vitest';
import { renderMarkdown, rewriteInternalDocLinks } from './markdown';

describe('markdown helpers', () => {
    it('deduplicates repeated heading ids', async () => {
        const html = await renderMarkdown('## Repeat\n\n## Repeat');

        expect(html).toContain('<h2 id="repeat">Repeat</h2>');
        expect(html).toContain('<h2 id="repeat-1">Repeat</h2>');
    });

    it('renders emphasized heading text without crashing marked token headings', async () => {
        const html = await renderMarkdown('## Hello *World*');

        expect(html).toContain('<h2 id="hello-world">Hello <em>World</em></h2>');
    });

    it('does not rewrite custom-scheme links as router links', () => {
        const html = rewriteInternalDocLinks(
            '<p><a href="ftp://example.com/archive.md">Archive</a></p>',
            '/docs/overview',
        );

        expect(html).toContain('href="ftp://example.com/archive.md"');
        expect(html).not.toContain('data-router-link="true"');
    });
});
