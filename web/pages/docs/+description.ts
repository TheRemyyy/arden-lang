import type { PageContext } from 'vike/types';
import type { DocsPageData } from '../../src/lib/content.server';
import { SITE_NAME } from '../../src/lib/site';

export default function description(pageContext: PageContext) {
    const data = pageContext.data as DocsPageData | undefined;
    return data?.description ?? `${SITE_NAME} documentation.`;
}
