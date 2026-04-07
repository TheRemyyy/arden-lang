import { useData } from 'vike-react/useData';
import { DocsContent } from '../../src/components/DocsContent';
import type { DocsPageData } from '../../src/lib/content.server';

export default function Page() {
    const data = useData<DocsPageData>();
    return <DocsContent {...data} />;
}
