use std::{io::Write, path::PathBuf, process::ExitCode};

use clap::{Parser, ValueHint};
use color_eyre::Result;
use planus_codegen::generate_form_config;
use planus_translation::translate_files_with_options;

/// Generate a JSON sidecar describing per-type UI form metadata for the
/// frontend (sections, priorities, collapsibles, unsupported variants,
/// immutable fields).
#[derive(Parser)]
pub struct Command {
    #[clap(value_hint = ValueHint::FilePath)]
    files: Vec<PathBuf>,

    /// Output JSON file path.
    #[clap(short = 'o')]
    #[clap(value_hint = ValueHint::AnyPath)]
    output_filename: PathBuf,
}

impl Command {
    pub fn run(self, options: super::AppOptions) -> Result<ExitCode> {
        let Some(declarations) =
            translate_files_with_options(&self.files, options.to_converter_options())
        else {
            return Ok(ExitCode::FAILURE);
        };

        let json = generate_form_config(&declarations)?;

        let mut file = std::fs::File::create(&self.output_filename)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;

        Ok(ExitCode::SUCCESS)
    }
}
