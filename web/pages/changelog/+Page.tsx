import { useData } from 'vike-react/useData';
import { ChangelogContent } from '../../src/components/ChangelogContent';
import type { ChangelogPageData } from '../../src/lib/content.server';

export default function Page() {
    const data = useData<ChangelogPageData>();
    return <ChangelogContent releases={data.releases} />;
}
