use color_eyre::eyre::Result;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};

pub struct TaktTable {
    table: Table,
}

impl TaktTable {
    pub fn new(headers: &[&str]) -> Self {
        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic);

        let header_cells = headers
            .iter()
            .map(|header| {
                Cell::new(header)
                    .add_attribute(Attribute::Bold)
                    .add_attribute(Attribute::Italic)
                    .fg(Color::Magenta)
            })
            .collect::<Vec<_>>();

        table.set_header(header_cells);
        Self { table }
    }

    pub fn add_row<T>(&mut self, row: T)
    where
        T: Into<comfy_table::Row>,
    {
        self.table.add_row(row);
    }

    pub fn print(&self) -> Result<()> {
        println!("{self}");
        Ok(())
    }
}

impl std::fmt::Display for TaktTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.table)
    }
}
