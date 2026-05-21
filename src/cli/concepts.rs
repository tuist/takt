use crate::cli::support::{OutputFormat, print_json};
use crate::output::style;
use crate::output::table::TaktTable;
use clap::Args;
use color_eyre::eyre::Result;
use serde::Serialize;

const CONCEPT_CHAIN: &str = "package -> capability -> action -> workflow -> run -> artifact";
const RUNTIME_RULE: &str =
    "capabilities execute on named runtime profiles; workflows never point at images directly.";

#[derive(Debug, Args)]
pub(crate) struct ConceptsCommand;

impl ConceptsCommand {
    pub(crate) fn run(self, format: OutputFormat) -> Result<()> {
        let concepts = vec![
            ConceptRow::new(
                "Package",
                "Distributable unit published to a registry",
                "Registry",
                "Capabilities and runtime profiles",
            ),
            ConceptRow::new(
                "Capability",
                "Reusable interface exported by a package",
                "Package",
                "Runtime, handler, input schema, output schema",
            ),
            ConceptRow::new(
                "Action",
                "Project-local configured use of a capability",
                "Project",
                "Defaults, secret refs, labels, account selection",
            ),
            ConceptRow::new(
                "Workflow",
                "Ordered graph that composes actions",
                "Project",
                "Action steps plus dependencies",
            ),
            ConceptRow::new(
                "Run",
                "One execution of an action or workflow",
                "Runtime",
                "Logs, status, timings, provenance",
            ),
            ConceptRow::new(
                "Artifact",
                "Persisted output from a run",
                "Datastore",
                "Structured data or files",
            ),
        ];

        if format == OutputFormat::Json {
            return print_json(&ConceptsOutput {
                chain: CONCEPT_CHAIN,
                runtime_rule: RUNTIME_RULE,
                concepts,
            });
        }

        println!("{} {}", style::title("Takt"), style::muted(CONCEPT_CHAIN));
        println!();

        let mut table = TaktTable::new(&["Concept", "Role", "Scope", "Carries"]);
        for concept in concepts {
            table.add_row([concept.name, concept.role, concept.scope, concept.carries]);
        }
        table.print()?;

        println!();
        println!("{} {}", style::label("Runtime rule:"), RUNTIME_RULE);

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct ConceptsOutput {
    chain: &'static str,
    runtime_rule: &'static str,
    concepts: Vec<ConceptRow>,
}

#[derive(Debug, Serialize)]
struct ConceptRow {
    name: &'static str,
    role: &'static str,
    scope: &'static str,
    carries: &'static str,
}

impl ConceptRow {
    const fn new(
        name: &'static str,
        role: &'static str,
        scope: &'static str,
        carries: &'static str,
    ) -> Self {
        Self {
            name,
            role,
            scope,
            carries,
        }
    }
}
