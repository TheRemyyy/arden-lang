import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { Footer } from './Footer';

describe('Footer and docs metadata', () => {
    it('uses extensionless documentation routes', () => {
        render(
            <MemoryRouter>
                <Footer />
            </MemoryRouter>,
        );

        expect(screen.getByRole('link', { name: 'Docs Hub' })).toHaveAttribute(
            'href',
            '/docs/overview',
        );
        expect(screen.getByRole('link', { name: 'Stdlib Reference' })).toHaveAttribute(
            'href',
            '/docs/stdlib/overview',
        );
    });

    it('keeps sitemap entries extensionless', () => {
        const sitemapPath = path.resolve(process.cwd(), 'public', 'sitemap.xml');
        const sitemap = fs.readFileSync(sitemapPath, 'utf8');

        expect(sitemap).not.toContain('.md</loc>');
    });
});
