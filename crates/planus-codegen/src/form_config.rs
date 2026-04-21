//! Emits a JSON sidecar of per-type UI form annotations.

use std::collections::BTreeMap;

use heck::ToUpperCamelCase;
use planus_types::ast::Docstrings;
use planus_types::intermediate::{AbsolutePath, DeclarationKind, Declarations};
use serde::Serialize;

use crate::backend_translation::SchemaAnnotations;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct FieldMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<i64>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    collapsible: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    unsupported: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    immutable: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    inline: bool,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    allowed_values: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unsupported_fields: Vec<String>,
}

impl FieldMeta {
    fn is_empty(&self) -> bool {
        self.section.is_none()
            && self.priority.is_none()
            && !self.collapsible
            && !self.unsupported
            && !self.immutable
            && !self.inline
            && self.allowed_values.is_empty()
            && self.unsupported_fields.is_empty()
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeMeta {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<String, FieldMeta>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unsupported_variants: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    section_order: Vec<String>,
}

impl TypeMeta {
    fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.unsupported_variants.is_empty() && self.section_order.is_empty()
    }
}

fn parse_section_order(docstrings: &Docstrings) -> Vec<String> {
    for line in docstrings.iter_strings_without_locations() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@sections ") {
            return rest
                .split(',')
                .map(|s| {
                    let t = s.trim();
                    t.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(t).to_string()
                })
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    Vec::new()
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
                meta.section_order = parse_section_order(&decl.docstrings);
                for (field_name, field) in &table.fields {
                    let ann = SchemaAnnotations::parse(&field.docstrings);
                    let fm = FieldMeta {
                        section: ann.section,
                        priority: ann.priority.as_deref().and_then(|s| s.parse::<i64>().ok()),
                        collapsible: ann.collapsible,
                        unsupported: ann.unsupported,
                        immutable: ann.immutable,
                        inline: ann.inline,
                        allowed_values: ann.allowed_values.into_iter().collect(),
                        unsupported_fields: ann.unsupported_fields,
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
