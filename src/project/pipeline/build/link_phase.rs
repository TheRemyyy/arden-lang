use crate::cache::{
    compute_link_fingerprint, dedupe_link_inputs, save_link_manifest_cache, should_skip_final_link,
    BuildTimings, LinkManifestCache, LINK_MANIFEST_CACHE_SCHEMA,
};
use crate::cli::output::{print_cli_cache, print_cli_step};
use crate::linker::{link_objects, LinkConfig};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum FinalLinkPhaseError {
    FinalLink(String),
    ManifestSave(String),
}

impl fmt::Display for FinalLinkPhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FinalLink(message) | Self::ManifestSave(message) => write!(f, "{message}"),
        }
    }
}

impl From<FinalLinkPhaseError> for String {
    fn from(value: FinalLinkPhaseError) -> Self {
        value.to_string()
    }
}

pub(crate) struct FinalLinkInputs<'a, 'b> {
    pub(crate) previous_link_manifest: Option<&'a LinkManifestCache>,
    pub(crate) output_path: &'a Path,
    pub(crate) link: &'a LinkConfig<'b>,
    pub(crate) project_root: &'a Path,
    pub(crate) object_paths: Vec<Option<PathBuf>>,
    pub(crate) cache_miss_count: usize,
}

pub(crate) fn run_final_link_phase(
    build_timings: &mut BuildTimings,
    inputs: FinalLinkInputs<'_, '_>,
) -> Result<(), String> {
    run_final_link_phase_impl(build_timings, inputs).map_err(Into::into)
}

fn run_final_link_phase_impl(
    build_timings: &mut BuildTimings,
    inputs: FinalLinkInputs<'_, '_>,
) -> Result<(), FinalLinkPhaseError> {
    let link_inputs = build_timings.measure_step("link input assembly", || {
        dedupe_link_inputs(inputs.object_paths.into_iter().flatten().collect())
    });
    let current_link_manifest =
        build_timings.measure_step("link manifest prep", || LinkManifestCache {
            schema: LINK_MANIFEST_CACHE_SCHEMA.to_string(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            link_fingerprint: compute_link_fingerprint(
                inputs.output_path,
                &link_inputs,
                inputs.link,
            ),
            link_inputs: link_inputs.clone(),
        });

    if should_skip_final_link(
        inputs.previous_link_manifest,
        &current_link_manifest,
        inputs.output_path,
        inputs.cache_miss_count,
    ) {
        print_cli_cache("Reused final link output from manifest cache");
        build_timings.record_counts(
            "final link",
            &[("objects", link_inputs.len()), ("linked", 0), ("reused", 1)],
        );
    } else {
        print_cli_step("Linking final artifact");
        build_timings
            .measure("final link", || {
                link_objects(&link_inputs, inputs.output_path, inputs.link)
            })
            .map_err(FinalLinkPhaseError::FinalLink)?;
        build_timings.record_counts(
            "final link",
            &[("objects", link_inputs.len()), ("linked", 1), ("reused", 0)],
        );
        build_timings
            .measure("link manifest save", || {
                save_link_manifest_cache(inputs.project_root, &current_link_manifest)
            })
            .map_err(FinalLinkPhaseError::ManifestSave)?;
    }

    Ok(())
}
