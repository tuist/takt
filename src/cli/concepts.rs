use crate::cli::support::{CommandContext, OutputFormat, structured_json_string};
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
        print!("{}", render_output(&concepts, context.format)?);
        Ok(())
    }
}

fn render_output(concepts: &core::ConceptsOutput, format: OutputFormat) -> Result<String> {
    if format != OutputFormat::Text {
        return structured_json_string(concepts, format);
    }

    let mut rendered = format!(
        "{} {}\n\n",
        style::title("Takt"),
        style::muted(concepts.chain)
    );

    let mut table = TaktTable::new(&["Concept", "Role", "Scope", "Carries"]);
    for concept in &concepts.concepts {
        table.add_row([concept.name, concept.role, concept.scope, concept.carries]);
    }
    rendered.push_str(&table.to_string());
    rendered.push_str("\n\n");
    rendered.push_str(&format!(
        "{} {}\n",
        style::label("Runtime rule:"),
        concepts.runtime_rule
    ));

    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::render_output;
    use crate::cli::support::OutputFormat;
    use crate::core;
    use color_eyre::eyre::Result;

    #[test]
    fn text_output_matches_snapshot() -> Result<()> {
        insta::assert_snapshot!(render_output(&core::concepts(), OutputFormat::Text)?, @r#"
        Takt package -> capability -> action -> workflow -> run -> artifact

         Concept     Role                                          Scope      Carries                                          
         Package     Distributable unit published to a registry    Registry   Capabilities and runtime profiles                
         Capability  Reusable interface exported by a package      Package    Runtime, handler, input schema, output schema    
         Action      Project-local configured use of a capability  Project    Defaults, secret refs, labels, account selection 
         Workflow    Ordered graph that composes actions           Project    Action steps plus dependencies                   
         Run         One execution of an action or workflow        Runtime    Logs, status, timings, provenance                
         Artifact    Persisted output from a run                   Datastore  Structured data or files                         

        Runtime rule: capabilities execute on named runtime profiles; workflows never point at images directly.
        "#);
        Ok(())
    }

    #[test]
    fn json_output_matches_snapshot() -> Result<()> {
        insta::assert_snapshot!(render_output(&core::concepts(), OutputFormat::Json)?, @r#"
        {
          "chain": "package -> capability -> action -> workflow -> run -> artifact",
          "runtime_rule": "capabilities execute on named runtime profiles; workflows never point at images directly.",
          "concepts": [
            {
              "name": "Package",
              "role": "Distributable unit published to a registry",
              "scope": "Registry",
              "carries": "Capabilities and runtime profiles"
            },
            {
              "name": "Capability",
              "role": "Reusable interface exported by a package",
              "scope": "Package",
              "carries": "Runtime, handler, input schema, output schema"
            },
            {
              "name": "Action",
              "role": "Project-local configured use of a capability",
              "scope": "Project",
              "carries": "Defaults, secret refs, labels, account selection"
            },
            {
              "name": "Workflow",
              "role": "Ordered graph that composes actions",
              "scope": "Project",
              "carries": "Action steps plus dependencies"
            },
            {
              "name": "Run",
              "role": "One execution of an action or workflow",
              "scope": "Runtime",
              "carries": "Logs, status, timings, provenance"
            },
            {
              "name": "Artifact",
              "role": "Persisted output from a run",
              "scope": "Datastore",
              "carries": "Structured data or files"
            }
          ]
        }
        "#);
        Ok(())
    }

    #[test]
    fn toon_output_matches_snapshot() -> Result<()> {
        insta::assert_snapshot!(render_output(&core::concepts(), OutputFormat::Toon)?, @r#"
        {"chain":"package -> capability -> action -> workflow -> run -> artifact","runtime_rule":"capabilities execute on named runtime profiles; workflows never point at images directly.","concepts":[{"name":"Package","role":"Distributable unit published to a registry","scope":"Registry","carries":"Capabilities and runtime profiles"},{"name":"Capability","role":"Reusable interface exported by a package","scope":"Package","carries":"Runtime, handler, input schema, output schema"},{"name":"Action","role":"Project-local configured use of a capability","scope":"Project","carries":"Defaults, secret refs, labels, account selection"},{"name":"Workflow","role":"Ordered graph that composes actions","scope":"Project","carries":"Action steps plus dependencies"},{"name":"Run","role":"One execution of an action or workflow","scope":"Runtime","carries":"Logs, status, timings, provenance"},{"name":"Artifact","role":"Persisted output from a run","scope":"Datastore","carries":"Structured data or files"}]}
        "#);
        Ok(())
    }
}
