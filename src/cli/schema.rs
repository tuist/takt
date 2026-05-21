use crate::cli::support::{OutputFormat, print_data};
use crate::domain::{
    ActionDefinition, CapabilityDefinition, PackageManifest, RuntimeProfile, WorkflowDefinition,
};
use clap::{Args, ValueEnum};
use color_eyre::eyre::Result;
use schemars::schema_for;
use serde::Serialize;

#[derive(Debug, Args)]
pub(crate) struct SchemaCommand {
    #[arg(value_enum, default_value_t = SchemaTarget::All)]
    target: SchemaTarget,
}

impl SchemaCommand {
    pub(crate) fn run(self, format: OutputFormat) -> Result<()> {
        match self.target {
            SchemaTarget::All => {
                let bundle = SchemaBundle {
                    package: schema_for!(PackageManifest),
                    runtime: schema_for!(RuntimeProfile),
                    capability: schema_for!(CapabilityDefinition),
                    action: schema_for!(ActionDefinition),
                    workflow: schema_for!(WorkflowDefinition),
                };

                print_data(&bundle, format)?;
            }
            SchemaTarget::Package => print_schema::<PackageManifest>(format)?,
            SchemaTarget::Runtime => print_schema::<RuntimeProfile>(format)?,
            SchemaTarget::Capability => print_schema::<CapabilityDefinition>(format)?,
            SchemaTarget::Action => print_schema::<ActionDefinition>(format)?,
            SchemaTarget::Workflow => print_schema::<WorkflowDefinition>(format)?,
        }

        Ok(())
    }
}

fn print_schema<T>(format: OutputFormat) -> Result<()>
where
    T: schemars::JsonSchema + Serialize,
{
    print_data(&schema_for!(T), format)
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SchemaTarget {
    All,
    Package,
    Runtime,
    Capability,
    Action,
    Workflow,
}

#[derive(Debug, Serialize)]
struct SchemaBundle {
    package: schemars::schema::RootSchema,
    runtime: schemars::schema::RootSchema,
    capability: schemars::schema::RootSchema,
    action: schemars::schema::RootSchema,
    workflow: schemars::schema::RootSchema,
}
