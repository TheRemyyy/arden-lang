import type { PageContextServer } from 'vike/types';
import { loadDocPage } from '../../src/lib/content.server';

export async function data(pageContext: PageContextServer) {
    return loadDocPage(pageContext.urlPathname);
}
