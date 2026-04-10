import type { PageContextServer } from 'vike/types';
import { searchSite, type SearchResult } from '../../src/lib/site-search';

export type SearchPageData = {
    initialQuery: string;
    initialResults: SearchResult[];
};

function normalizeQuery(rawQuery: string | undefined): string {
    return rawQuery?.trim().slice(0, 120) ?? '';
}

export async function data(pageContext: PageContextServer): Promise<SearchPageData> {
    const initialQuery = normalizeQuery(pageContext.urlParsed.search.q);

    return {
        initialQuery,
        initialResults: searchSite(initialQuery),
    };
}
