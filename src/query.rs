use color_eyre::eyre::{Result, bail};
use schemars::JsonSchema;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ListEnvelope<T> {
    pub command: &'static str,
    pub total: usize,
    pub limited: bool,
    pub results: Vec<T>,
}

impl<T> ListEnvelope<T> {
    pub fn new(command: &'static str, total_before_limit: usize, results: Vec<T>) -> Self {
        let limited = results.len() < total_before_limit;
        Self {
            command,
            total: total_before_limit,
            limited,
            results,
        }
    }
}

/// Parse a compact duration like `30s`, `5m`, `2h`, `7d`, `4w` into milliseconds.
pub fn parse_duration_ms(input: &str) -> Result<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("duration must not be empty");
    }

    let split_at = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    if split_at == 0 {
        bail!(
            "duration '{}' must start with digits followed by a unit (s|m|h|d|w)",
            input
        );
    }

    let (number, unit) = trimmed.split_at(split_at);
    let value: u64 = number
        .parse()
        .map_err(|_| color_eyre::eyre::eyre!("duration '{}' has an invalid number", input))?;

    let factor_ms: u64 = match unit {
        "s" => 1_000,
        "m" => 60 * 1_000,
        "h" => 60 * 60 * 1_000,
        "d" => 24 * 60 * 60 * 1_000,
        "w" => 7 * 24 * 60 * 60 * 1_000,
        "" => bail!("duration '{}' is missing a unit (s|m|h|d|w)", input),
        other => bail!("duration '{}' has an unsupported unit '{}'", input, other),
    };

    value
        .checked_mul(factor_ms)
        .ok_or_else(|| color_eyre::eyre::eyre!("duration '{}' overflows", input))
}

pub fn now_unix_ms() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| color_eyre::eyre::eyre!("system clock is before UNIX_EPOCH: {error}"))?;
    Ok(duration.as_millis() as u64)
}

/// Generate a run id that is unique within this process. The format is
/// `run-<unix_ms>-<seq>`; the wall-clock prefix keeps ids sortable, the
/// monotonically-increasing seq guarantees uniqueness even when two runs
/// land in the same millisecond (notably: a workflow record and its first
/// child step run, which is the case for templated workflows).
pub fn new_run_id() -> Result<(String, u64)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let now = now_unix_ms()?;
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    Ok((format!("run-{now}-{seq}"), now))
}

pub fn since_threshold_unix_ms(duration: &str) -> Result<u64> {
    let now = now_unix_ms()?;
    let span = parse_duration_ms(duration)?;
    Ok(now.saturating_sub(span))
}

/// A flat key=value equality predicate. The path uses dot notation; the
/// callee decides which paths it understands.
#[derive(Debug, Clone)]
pub struct Predicate {
    pub path: String,
    pub value: String,
}

pub fn parse_predicate(input: &str) -> Result<Predicate> {
    let trimmed = input.trim();
    let (path, value) = trimmed
        .split_once("==")
        .or_else(|| trimmed.split_once('='))
        .ok_or_else(|| {
            color_eyre::eyre::eyre!("predicate '{}' must look like path=value", input)
        })?;
    let path = path.trim();
    let value = value.trim();
    if path.is_empty() {
        bail!("predicate '{}' has an empty path", input);
    }
    Ok(Predicate {
        path: path.to_string(),
        value: value.to_string(),
    })
}

pub fn parse_key_value(input: &str) -> Result<(String, String)> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| color_eyre::eyre::eyre!("expected key=value, got '{}'", input))?;
    Ok((key.trim().to_string(), value.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compact_durations() {
        assert_eq!(parse_duration_ms("30s").unwrap(), 30_000);
        assert_eq!(parse_duration_ms("5m").unwrap(), 300_000);
        assert_eq!(parse_duration_ms("2h").unwrap(), 7_200_000);
        assert_eq!(parse_duration_ms("1d").unwrap(), 86_400_000);
        assert_eq!(parse_duration_ms("1w").unwrap(), 604_800_000);
    }

    #[test]
    fn rejects_bad_durations() {
        assert!(parse_duration_ms("").is_err());
        assert!(parse_duration_ms("abc").is_err());
        assert!(parse_duration_ms("10").is_err());
        assert!(parse_duration_ms("10y").is_err());
    }

    #[test]
    fn parses_predicate() {
        let predicate = parse_predicate("tags.env=prod").unwrap();
        assert_eq!(predicate.path, "tags.env");
        assert_eq!(predicate.value, "prod");

        let predicate = parse_predicate("name == report").unwrap();
        assert_eq!(predicate.path, "name");
        assert_eq!(predicate.value, "report");
    }

    #[test]
    fn list_envelope_marks_truncation() {
        let envelope = ListEnvelope::new("test", 10, vec![1u32, 2, 3]);
        assert_eq!(envelope.total, 10);
        assert!(envelope.limited);

        let envelope = ListEnvelope::new("test", 3, vec![1u32, 2, 3]);
        assert!(!envelope.limited);
    }
}
