import type { PageContext } from 'vike/types';
import type { ChangelogPageData } from '../../src/lib/content.server';
import { SITE_NAME } from '../../src/lib/site';

export default function title(pageContext: PageContext) {
    const data = pageContext.data as ChangelogPageData | undefined;
    return data ? `${data.title} | ${SITE_NAME}` : `${SITE_NAME} | Changelog`;
}
