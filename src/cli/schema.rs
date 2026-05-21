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
    pub(crate) fn run(self) -> Result<()> {
        match self.target {
            SchemaTarget::All => {
                let bundle = SchemaBundle {
                    package: schema_for!(PackageManifest),
                    runtime: schema_for!(RuntimeProfile),
                    capability: schema_for!(CapabilityDefinition),
                    action: schema_for!(ActionDefinition),
                    workflow: schema_for!(WorkflowDefinition),
                };

                println!("{}", serde_json::to_string_pretty(&bundle)?);
            }
            SchemaTarget::Package => print_schema::<PackageManifest>()?,
            SchemaTarget::Runtime => print_schema::<RuntimeProfile>()?,
            SchemaTarget::Capability => print_schema::<CapabilityDefinition>()?,
            SchemaTarget::Action => print_schema::<ActionDefinition>()?,
            SchemaTarget::Workflow => print_schema::<WorkflowDefinition>()?,
        }

        Ok(())
    }
}

fn print_schema<T>() -> Result<()>
where
    T: schemars::JsonSchema + Serialize,
{
    println!("{}", serde_json::to_string_pretty(&schema_for!(T))?);
    Ok(())
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
