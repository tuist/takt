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

#[cfg(test)]
mod tests {
    use super::{label, muted, title};

    #[test]
    fn style_helpers_match_snapshots() {
        let snapshot = format!(
            "title: {}\nlabel: {}\nmuted: {}\n",
            format!("{}", title("Takt").force_styling(true)).escape_debug(),
            format!("{}", label("Kind").force_styling(true)).escape_debug(),
            format!("{}", muted("quiet").force_styling(true)).escape_debug(),
        );

        insta::assert_snapshot!(snapshot);
    }
}
