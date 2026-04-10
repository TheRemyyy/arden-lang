import { LegalPage } from '../../src/components/LegalPage';

const sections = [
    {
        title: 'No account system',
        body: [
            'This website does not require user accounts, paid subscriptions, or personal profile creation.',
            'If that changes in the future, the policy should be updated before any collection starts.',
        ],
    },
    {
        title: 'No analytics or ad tracking by design',
        body: [
            'The project is intended to be documentation-first and open source. It is not designed around behavioral advertising or user profiling.',
            'No cookies, tracking pixels, or ad-tech identifiers are intentionally used as part of the core website experience.',
        ],
    },
    {
        title: 'Open-source project context',
        body: [
            'Arden is distributed as an open-source project under the Apache License 2.0. The website exists mainly to publish docs, install guidance, and release information.',
            'Public repository activity on GitHub is governed by GitHub’s own terms and privacy policies, not by this page.',
        ],
    },
    {
        title: 'Contact and common-sense reserve',
        body: [
            'If a bug, hosting issue, abuse event, or legal request ever requires a limited operational response, the site operator may keep only the minimal technical information reasonably necessary to resolve that issue.',
            'This page is a project policy summary, not individualized legal advice.',
        ],
    },
];

export default function Page() {
    return (
        <LegalPage
            eyebrow="Legal"
            title="Privacy Policy"
            intro="Arden is an open-source documentation site. The intended model is simple: publish docs, releases, and install information without turning the website into a data collection machine."
            sections={sections}
        />
    );
}

