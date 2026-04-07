import type { PageContext } from 'vike/types';
import type { DocsPageData } from '../../src/lib/content.server';
import { SITE_NAME } from '../../src/lib/site';

export default function title(pageContext: PageContext) {
    const data = pageContext.data as DocsPageData | undefined;
    return data ? `${data.title} | ${SITE_NAME} Docs` : `${SITE_NAME} Docs`;
}
