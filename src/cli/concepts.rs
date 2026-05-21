use crate::output::style;
use crate::output::table::TaktTable;
use clap::Args;
use color_eyre::eyre::Result;
use serde::Serialize;

#[derive(Debug, Args)]
pub(crate) struct ConceptsCommand {
    /// Print the concepts in JSON instead of a table
    #[arg(long)]
    json: bool,
}

impl ConceptsCommand {
    pub(crate) fn run(self) -> Result<()> {
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

        if self.json {
            println!("{}", serde_json::to_string_pretty(&concepts)?);
            return Ok(());
        }

        println!(
            "{} {}",
            style::title("Takt"),
            style::muted("package -> capability -> action -> workflow -> run -> artifact")
        );
        println!();

        let mut table = TaktTable::new(&["Concept", "Role", "Scope", "Carries"]);
        for concept in concepts {
            table.add_row([concept.name, concept.role, concept.scope, concept.carries]);
        }
        table.print()?;

        println!();
        println!(
            "{} {}",
            style::label("Runtime rule:"),
            "capabilities execute on named runtime profiles; workflows never point at images directly."
        );

        Ok(())
    }
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
