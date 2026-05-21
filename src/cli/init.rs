use crate::cli::support::{
    CommandContext, OutputFormat, structured_json_string, written_files_summary,
};
use crate::core;
use crate::scaffold::CodingAgent;
use clap::Args;
use color_eyre::eyre::Result;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(crate) struct InitCommand {
    /// Package name to write into the manifest
    name: String,
    /// Optional package description
    #[arg(short, long, env = "TAKT_INIT_DESCRIPTION")]
    description: Option<String>,
    /// Output path for the package manifest
    #[arg(
        short,
        long,
        env = "TAKT_INIT_OUTPUT",
        default_value = "takt.json",
        value_name = "PATH"
    )]
    output: PathBuf,
    /// Coding-agent bootstrap to write into the package
    #[arg(short = 'a', long, env = "TAKT_INIT_CODING_AGENT", value_enum, default_value_t = CodingAgent::Codex)]
    coding_agent: CodingAgent,
    /// Overwrite an existing file
    #[arg(short, long, env = "TAKT_INIT_FORCE")]
    force: bool,
}

impl InitCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let output = core::init_package(
            self.name,
            self.description,
            self.output,
            self.force,
            self.coding_agent,
        )?;
        print!("{}", render_init_output(&output, context.format)?);
        Ok(())
    }
}

fn render_init_output(output: &core::InitOutput, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Text => Ok(written_files_summary(&output.files)),
        OutputFormat::Json | OutputFormat::Toon => structured_json_string(output, format),
    }
}

#[cfg(test)]
mod tests {
    use super::render_init_output;
    use crate::cli::support::OutputFormat;
    use crate::core::{InitOutput, WrittenFile};
    use crate::domain::PackageManifest;
    use crate::scaffold::CodingAgent;
    use color_eyre::eyre::Result;
    use std::path::PathBuf;

    #[test]
    fn text_output_matches_snapshot() -> Result<()> {
        let output = sample_init_output();
        insta::assert_snapshot!(render_init_output(&output, OutputFormat::Text)?, @r#"
        Wrote takt.json
        Wrote AGENTS.md
        "#);
        Ok(())
    }

    #[test]
    fn toon_output_matches_snapshot() -> Result<()> {
        let output = sample_init_output();
        insta::assert_snapshot!(render_init_output(&output, OutputFormat::Toon)?, @r#"
        {"command":"init","coding_agent":"codex","package":{"api_version":"takt.dev/v1alpha1","kind":"Package","package":{"name":"@acme/test","version":"0.1.0","description":"Test package"},"runtimes":{"default":{"sandbox":"microsandbox","image":"ghcr.io/example/takt-runtime@sha256:replace-me","cpus":1,"memory_mb":512,"network":{"mode":"disabled"}}},"capabilities":{"example.run":{"description":"Example capability scaffold","runtime":"default","handler":{"language":"typescript","entrypoint":"handlers/example.ts"},"input":{"path":"schemas/example-input.json","description":"Input schema for the example capability"},"output":{"path":"schemas/example-output.json","description":"Output schema for the example capability"}}}},"files":[{"label":"package","path":"takt.json"},{"label":"agent guide","path":"AGENTS.md"}]}
        "#);
        Ok(())
    }

    fn sample_init_output() -> InitOutput {
        InitOutput {
            command: "init",
            coding_agent: CodingAgent::Codex,
            package: PackageManifest::starter("@acme/test".into(), Some("Test package".into())),
            files: vec![
                WrittenFile {
                    label: "package".into(),
                    path: PathBuf::from("takt.json"),
                },
                WrittenFile {
                    label: "agent guide".into(),
                    path: PathBuf::from("AGENTS.md"),
                },
            ],
        }
    }
}
