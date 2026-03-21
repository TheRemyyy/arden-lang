import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useNavigate } from 'react-router-dom';
import { Docs } from './Docs';

function resolveFetchResponse(markdown: string) {
    return {
        ok: true,
        text: async () => markdown,
    };
}

function NavigateButton({ to }: { to: string }) {
    const navigate = useNavigate();
    return <button onClick={() => navigate(to)}>Go</button>;
}

describe('Docs page', () => {
    afterEach(() => {
        vi.restoreAllMocks();
        cleanup();
    });

    it('rewrites markdown links to router paths and sanitizes raw html', async () => {
        vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
        vi.stubGlobal(
            'fetch',
            vi.fn().mockResolvedValue(
                resolveFetchResponse(
                    '# Page\n\n[Stdlib](../stdlib/overview.md)\n\n<script>window.__xss = true;</script>',
                ),
            ),
        );

        const { container } = render(
            <MemoryRouter initialEntries={['/docs/features/projects']}>
                <Routes>
                    <Route path="/docs/*" element={<Docs />} />
                </Routes>
            </MemoryRouter>,
        );

        const link = await screen.findByRole('link', { name: 'Stdlib' });
        expect(link).toHaveAttribute('href', '/docs/stdlib/overview');

        await waitFor(() => {
            expect(container.querySelector('script')).toBeNull();
        });
    });

    it('normalizes nested trailing-slash doc routes before fetching', async () => {
        vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
        const fetchMock = vi.fn().mockResolvedValue(resolveFetchResponse('# Projects'));
        vi.stubGlobal('fetch', fetchMock);

        render(
            <MemoryRouter initialEntries={['/docs/features/projects/']}>
                <Routes>
                    <Route path="/docs/*" element={<Docs />} />
                </Routes>
            </MemoryRouter>,
        );

        await screen.findByRole('heading', { name: 'Projects' });
        expect(fetchMock).toHaveBeenCalledWith(
            '/docs/features/projects.md',
            expect.objectContaining({ signal: expect.any(AbortSignal) }),
        );
    });

    it('shows loading state instead of stale content while navigating between docs', async () => {
        vi.spyOn(window, 'scrollTo').mockImplementation(() => {});
        let resolveSecondFetch: ((value: unknown) => void) | undefined;
        const secondFetch = new Promise((resolve) => {
            resolveSecondFetch = resolve;
        });
        const fetchMock = vi
            .fn()
            .mockResolvedValueOnce(resolveFetchResponse('# Old Page'))
            .mockReturnValueOnce(secondFetch);
        vi.stubGlobal('fetch', fetchMock);

        const { container } = render(
            <MemoryRouter initialEntries={['/docs/features/projects']}>
                <Routes>
                    <Route
                        path="/docs/*"
                        element={
                            <>
                                <NavigateButton to="/docs/overview" />
                                <Docs />
                            </>
                        }
                    />
                </Routes>
            </MemoryRouter>,
        );

        await screen.findByRole('heading', { name: 'Old Page' });
        screen.getByRole('button', { name: 'Go' }).click();

        await waitFor(() => {
            expect(container.querySelector('article')).toBeNull();
        });

        resolveSecondFetch?.(resolveFetchResponse('# New Page'));
        await screen.findByRole('heading', { name: 'New Page' });
    });
});
