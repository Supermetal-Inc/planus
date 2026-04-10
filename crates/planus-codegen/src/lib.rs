//! Library for doing code generation for types from [`planus_types`].
//!
//! This library is an internal implementation
//! detail of [planus-cli](https://docs.rs/planus-cli).
//!
//! Feel free to use it, however there are no stability guarantees.

use askama::Template;
use planus_types::intermediate::Declarations;

use crate::{
    analysis::run_analysis, backend_translation::run_backend, dot::DotBackend, rust::RustBackend,
};

mod analysis;
mod backend;
mod backend_translation;
mod dot;
mod rust;
mod templates;

pub fn generate_rust(declarations: &Declarations) -> eyre::Result<String> {
    let default_analysis = run_analysis(declarations, &mut rust::analysis::DefaultAnalysis);
    let eq_analysis = run_analysis(declarations, &mut rust::analysis::EqAnalysis);
    let infallible_analysis = run_analysis(
        declarations,
        &mut rust::analysis::InfallibleConversionAnalysis,
    );

    // Detect short names that appear in more than one namespace so we can
    // give them fully qualified OpenAPI schema names to prevent collisions.
    let colliding_names = {
        use heck::ToUpperCamelCase;
        let mut counts = std::collections::HashMap::<String, usize>::new();
        for path in declarations.declarations.keys() {
            if let Some(short) = path.0.last() {
                *counts.entry(short.to_upper_camel_case()).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name)
            .collect::<std::collections::HashSet<_>>()
    };

    let output = run_backend(
        &mut RustBackend {
            default_analysis,
            eq_analysis,
            infallible_analysis,
            colliding_names,
        },
        declarations,
    );
    let res = templates::rust::Namespace(&output).render().unwrap();
    let res = rust::format_string(&res, Some(1_000_000))?;
    let res = rust::format_string(&res, None)?;
    Ok(res)
}

pub fn generate_dot(declarations: &Declarations) -> String {
    let output = run_backend(&mut DotBackend { color_seed: 0 }, declarations);
    let res = templates::dot::Namespace(&output).render().unwrap();
    res
}
