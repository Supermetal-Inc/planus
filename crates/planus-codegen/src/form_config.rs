//! Emits a JSON sidecar describing per-type UI form metadata (sections,
//! priorities, collapsibles, unsupported variants, immutable fields).
//! Read by the frontend codegen to produce a static `FORM_CONFIG` map that
//! `ZodForm` looks up by schema reference at render time.

use std::collections::BTreeMap;

use heck::ToUpperCamelCase;
use planus_types::intermediate::{AbsolutePath, DeclarationKind, Declarations};
use serde::Serialize;

use crate::backend_translation::SchemaAnnotations;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct FieldMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    collapsible: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    unsupported: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    immutable: bool,
}

impl FieldMeta {
    fn is_empty(&self) -> bool {
        self.section.is_none()
            && self.priority.is_none()
            && !self.collapsible
            && !self.unsupported
            && !self.immutable
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeMeta {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<String, FieldMeta>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unsupported_variants: Vec<String>,
}

impl TypeMeta {
    fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.unsupported_variants.is_empty()
    }
}

/// Joins a declaration's namespace path into UpperCamelCase to match the
/// `#[schema(as = ...)]` name emitted by the Rust backend. Stays in sync with
/// the schema_name computation in `rust::mod::translate_{table,enum,union}`.
fn schema_name_for(path: &AbsolutePath) -> String {
    path.0
        .iter()
        .map(|s| s.to_upper_camel_case())
        .collect::<String>()
}

pub fn generate_form_config(declarations: &Declarations) -> eyre::Result<String> {
    let mut out = BTreeMap::<String, TypeMeta>::new();

    for (path, decl) in &declarations.declarations {
        let mut meta = TypeMeta::default();

        match &decl.kind {
            DeclarationKind::Table(table) => {
                for (field_name, field) in &table.fields {
                    let ann = SchemaAnnotations::parse(&field.docstrings);
                    let fm = FieldMeta {
                        section: ann.section,
                        priority: ann.priority,
                        collapsible: ann.collapsible,
                        unsupported: ann.unsupported,
                        immutable: ann.immutable,
                    };
                    if !fm.is_empty() {
                        meta.fields.insert(field_name.clone(), fm);
                    }
                }
            }
            DeclarationKind::Union(union) => {
                for (variant_name, variant) in &union.variants {
                    let ann = SchemaAnnotations::parse(&variant.docstrings);
                    if ann.unsupported {
                        meta.unsupported_variants.push(variant_name.clone());
                    }
                }
            }
            _ => {}
        }

        if !meta.is_empty() {
            out.insert(schema_name_for(path), meta);
        }
    }

    Ok(serde_json::to_string_pretty(&out)?)
}
