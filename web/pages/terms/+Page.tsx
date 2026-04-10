import { LegalPage } from '../../src/components/LegalPage';

const sections = [
    {
        title: 'Project scope',
        body: [
            'The Arden website is provided to publish documentation, installation guidance, changelog history, and related open-source project information.',
            'Use of the compiler, source code, and repository remains subject to the Apache License 2.0 and any applicable third-party licenses referenced by the project.',
        ],
    },
    {
        title: 'No warranty',
        body: [
            'The website, source code, binaries, and documentation are provided on an “as is” and “as available” basis, without guarantees of uninterrupted service, merchantability, fitness for a particular purpose, or non-infringement.',
            'If you rely on Arden for production work, that remains your engineering and deployment decision.',
        ],
    },
    {
        title: 'Acceptable use',
        body: [
            'Do not use the website or project resources in a way that attempts to disrupt hosting, abuse infrastructure, scrape aggressively, or violate applicable law.',
            'Reasonable mirroring, indexing, and ordinary open-source usage are consistent with the project’s public nature.',
        ],
    },
    {
        title: 'Liability boundary',
        body: [
            'To the maximum extent permitted by applicable law, the site operator is not liable for indirect, incidental, special, consequential, or business-interruption damages arising from use of the site, documentation, or distributed software.',
            'Nothing on this page limits any rights or obligations that cannot legally be disclaimed under applicable law.',
        ],
    },
];

export default function Page() {
    return (
        <LegalPage
            eyebrow="Legal"
            title="Terms of Use"
            intro="These terms are meant to keep the website and project surface explicit: Arden is an open-source project, not a hosted paid platform with custom service guarantees."
            sections={sections}
        />
    );
}

