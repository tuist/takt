use crate::cli::support::{CommandContext, OutputFormat, print_json};
use crate::core;
use crate::output::style;
use crate::output::table::TaktTable;
use clap::Args;
use color_eyre::eyre::Result;

#[derive(Debug, Args)]
pub(crate) struct ConceptsCommand;

impl ConceptsCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        let concepts = core::concepts();

        if context.format == OutputFormat::Json {
            return print_json(&concepts);
        }

        println!("{} {}", style::title("Takt"), style::muted(concepts.chain));
        println!();

        let mut table = TaktTable::new(&["Concept", "Role", "Scope", "Carries"]);
        for concept in concepts.concepts {
            table.add_row([concept.name, concept.role, concept.scope, concept.carries]);
        }
        table.print()?;

        println!();
        println!(
            "{} {}",
            style::label("Runtime rule:"),
            concepts.runtime_rule
        );

        Ok(())
    }
}
