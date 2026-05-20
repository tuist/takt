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
    fn title_renders_bold_cyan_text() {
        let rendered = format!("{}", title("Takt").force_styling(true));
        assert!(rendered.contains("\x1b[1m"));
        assert!(rendered.contains("\x1b[36m"));
        assert!(rendered.ends_with("Takt\x1b[0m"));
    }

    #[test]
    fn label_renders_bold_magenta_text() {
        let rendered = format!("{}", label("Kind").force_styling(true));
        assert!(rendered.contains("\x1b[1m"));
        assert!(rendered.contains("\x1b[35m"));
        assert!(rendered.ends_with("Kind\x1b[0m"));
    }

    #[test]
    fn muted_renders_dim_text() {
        let rendered = format!("{}", muted("quiet").force_styling(true));
        assert!(rendered.contains("\x1b[2m"));
        assert!(rendered.ends_with("quiet\x1b[0m"));
    }
}
