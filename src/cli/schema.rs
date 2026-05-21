use crate::cli::support::{CommandContext, print_data};
use crate::core::{self, SchemaTarget};
use clap::Args;
use color_eyre::eyre::Result;

#[derive(Debug, Args)]
pub(crate) struct SchemaCommand {
    #[arg(value_enum, default_value_t = SchemaTarget::All)]
    target: SchemaTarget,
}

impl SchemaCommand {
    pub(crate) fn run(self, context: CommandContext) -> Result<()> {
        print_data(&core::schema_for_target(self.target), context.format)
    }
}
