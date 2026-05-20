use console::{StyledObject, style};

pub fn title(value: impl ToString) -> StyledObject<String> {
    style(value.to_string()).bold().cyan()
}

pub fn label(value: impl ToString) -> StyledObject<String> {
    style(value.to_string()).bold().magenta()
}

pub fn muted(value: impl ToString) -> StyledObject<String> {
    style(value.to_string()).dim()
}
