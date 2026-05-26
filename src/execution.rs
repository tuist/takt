use crate::datastore::{ArtifactRecord, ProducerKind, StorageRef};
use crate::domain::{ArtifactType, RuntimeProfile, SANDBOX_MICROSANDBOX, SANDBOX_PROCESS};
use crate::query::now_unix_ms;
use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// File descriptor / environment contract the handler sees:
///
/// - `TAKT_RUN_ID`: the run identifier
/// - `TAKT_CAPABILITY`: the capability name being invoked
/// - `TAKT_PACKAGE_ROOT`: absolute path to the package root
/// - `TAKT_INPUT_PATH`: path to a JSON file containing the merged inputs
/// - `TAKT_RESULT_PATH`: path the handler MUST write a JSON result to
///
/// Result schema:
/// ```json
/// {
///   "output": <any>,            // optional immediate return value
///   "artifacts": [              // optional; persisted into the datastore
///     {
///       "name": "summary",
///       "type": "resource",     // or "file"
///       "value": <any>,         // required when type=resource
///       "path": "<filesystem path>", // required when type=file (relative to package_root or absolute)
///       "content_type": "application/json", // optional
///       "tags": { "env": "prod" }           // optional
///     }
///   ]
/// }
/// ```
pub struct ExecutionInput {
    pub run_id: String,
    pub capability: String,
    pub handler_entrypoint: PathBuf,
    pub package_root: PathBuf,
    pub inputs: BTreeMap<String, Value>,
    pub blobs_root: PathBuf,
    pub scratch_root: PathBuf,
    pub runtime: RuntimeProfile,
}

pub struct ExecutionOutcome {
    pub output: Option<Value>,
    pub artifacts: Vec<ArtifactRecord>,
    pub stdout_log_path: PathBuf,
    pub stderr_log_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct HandlerResult {
    #[serde(default)]
    output: Option<Value>,
    #[serde(default)]
    artifacts: Vec<HandlerArtifact>,
}

#[derive(Debug, Deserialize)]
struct HandlerArtifact {
    name: String,
    #[serde(rename = "type")]
    artifact_type: ArtifactType,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

pub fn execute_node_handler(input: ExecutionInput) -> Result<ExecutionOutcome> {
    let scratch = input.scratch_root.join(&input.run_id);
    fs::create_dir_all(&scratch)?;

    let input_path = scratch.join("input.json");
    let result_path = scratch.join("result.json");
    let stdout_path = scratch.join("stdout.log");
    let stderr_path = scratch.join("stderr.log");

    fs::write(&input_path, serde_json::to_string_pretty(&input.inputs)?)?;

    let handler_abs = if input.handler_entrypoint.is_absolute() {
        input.handler_entrypoint.clone()
    } else {
        input.package_root.join(&input.handler_entrypoint)
    };
    if !handler_abs.exists() {
        bail!(
            "handler entrypoint not found: {} (resolved from package root {})",
            handler_abs.display(),
            input.package_root.display()
        );
    }

    let stdout_file = fs::File::create(&stdout_path)?;
    let stderr_file = fs::File::create(&stderr_path)?;

    let mut command = build_handler_command(&input, &handler_abs, &input_path, &result_path)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    let mut child = command.spawn().map_err(|error| match error.kind() {
        ErrorKind::NotFound => sandbox_binary_not_found_error(&input.runtime),
        _ => eyre!(
            "failed to spawn handler '{}' under sandbox '{}': {}",
            handler_abs.display(),
            input.runtime.sandbox,
            error
        ),
    })?;

    let status = child.wait()?;

    if !status.success() {
        let stderr_tail = read_tail(&stderr_path, 4096).unwrap_or_default();
        bail!(
            "handler '{}' exited with status {}; stderr tail:\n{}",
            handler_abs.display(),
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "<signal>".into()),
            stderr_tail.trim()
        );
    }

    if !result_path.exists() {
        bail!(
            "handler '{}' exited successfully but did not write a result to {}. The handler must write JSON to the path in TAKT_RESULT_PATH.",
            handler_abs.display(),
            result_path.display()
        );
    }

    let raw = fs::read_to_string(&result_path)?;
    let result: HandlerResult = serde_json::from_str(&raw).map_err(|error| {
        eyre!(
            "handler result at {} is not valid JSON: {}",
            result_path.display(),
            error
        )
    })?;

    let mut artifacts = Vec::with_capacity(result.artifacts.len());
    for handler_artifact in result.artifacts {
        let artifact_id = format!(
            "art-{}-{}-1",
            input.run_id,
            sanitize_id(&handler_artifact.name)
        );
        let storage_ref = match handler_artifact.artifact_type {
            ArtifactType::Resource => {
                let value = handler_artifact.value.ok_or_else(|| {
                    eyre!(
                        "artifact '{}' is type=resource but did not provide a 'value'",
                        handler_artifact.name
                    )
                })?;
                StorageRef::Inline { value }
            }
            ArtifactType::File => {
                let raw_path = handler_artifact.path.as_ref().ok_or_else(|| {
                    eyre!(
                        "artifact '{}' is type=file but did not provide a 'path'",
                        handler_artifact.name
                    )
                })?;
                let source_abs = if Path::new(raw_path).is_absolute() {
                    PathBuf::from(raw_path)
                } else {
                    input.package_root.join(raw_path)
                };
                if !source_abs.exists() {
                    bail!(
                        "artifact '{}' file does not exist: {}",
                        handler_artifact.name,
                        source_abs.display()
                    );
                }
                let blob_dir = input.blobs_root.join(&artifact_id);
                fs::create_dir_all(&blob_dir)?;
                let filename = source_abs.file_name().ok_or_else(|| {
                    eyre!("artifact file path has no name: {}", source_abs.display())
                })?;
                let dest = blob_dir.join(filename);
                fs::copy(&source_abs, &dest)?;
                StorageRef::File { path: dest }
            }
        };

        artifacts.push(ArtifactRecord {
            id: artifact_id,
            run_id: input.run_id.clone(),
            producer_kind: ProducerKind::Capability,
            producer_name: input.capability.clone(),
            step_name: None,
            name: handler_artifact.name,
            artifact_type: handler_artifact.artifact_type,
            schema_ref: None,
            content_type: handler_artifact.content_type,
            version: 1,
            tags: handler_artifact.tags,
            created_at_unix_ms: now_unix_ms()?,
            retention: None,
            vary: Vec::new(),
            storage_ref,
        });
    }

    Ok(ExecutionOutcome {
        output: result.output,
        artifacts,
        stdout_log_path: stdout_path,
        stderr_log_path: stderr_path,
    })
}

fn build_handler_command(
    input: &ExecutionInput,
    handler_abs: &Path,
    input_path: &Path,
    result_path: &Path,
) -> Result<Command> {
    match input.runtime.sandbox.as_str() {
        SANDBOX_PROCESS => Ok(build_process_command(
            input,
            handler_abs,
            input_path,
            result_path,
        )),
        SANDBOX_MICROSANDBOX => {
            build_microsandbox_command(input, handler_abs, input_path, result_path)
        }
        other => bail!(
            "unsupported sandbox '{}' on runtime profile (supported: '{}' | '{}')",
            other,
            SANDBOX_PROCESS,
            SANDBOX_MICROSANDBOX
        ),
    }
}

fn build_process_command(
    input: &ExecutionInput,
    handler_abs: &Path,
    input_path: &Path,
    result_path: &Path,
) -> Command {
    let mut cmd = Command::new("node");
    cmd.arg(handler_abs);
    apply_takt_env(&mut cmd, input, input_path, result_path);
    cmd.current_dir(&input.package_root);
    cmd
}

fn build_microsandbox_command(
    input: &ExecutionInput,
    handler_abs: &Path,
    input_path: &Path,
    result_path: &Path,
) -> Result<Command> {
    let image = input.runtime.image.as_deref().ok_or_else(|| {
        eyre!(
            "runtime profile uses sandbox='microsandbox' but has no 'image' configured. \
             Add an OCI reference (e.g. docker.io/library/node:22-alpine or a digest-pinned image)."
        )
    })?;

    let mut cmd = Command::new("msb");
    cmd.arg("run").arg("--pull").arg("if-missing");

    match input.runtime.network.mode.as_str() {
        "disabled" => {
            cmd.arg("--no-network");
        }
        "allow-all" => {
            cmd.arg("--network-policy").arg("allow-all");
        }
        other => bail!(
            "unsupported network mode '{}' (supported: 'disabled' | 'allow-all')",
            other
        ),
    }

    if let Some(cpus) = input.runtime.cpus {
        cmd.arg("--cpus").arg(cpus.to_string());
    }
    if let Some(memory_mb) = input.runtime.memory_mb {
        cmd.arg("--memory").arg(format!("{memory_mb}M"));
    }

    cmd.arg("--workdir").arg(&input.package_root);

    // Mount the package root at the same path inside the VM so handler paths,
    // input/result paths, and the scratch directory (all under package_root)
    // resolve identically inside the sandbox.
    cmd.arg("--volume").arg(format!(
        "{}:{}",
        input.package_root.display(),
        input.package_root.display()
    ));

    apply_takt_env_via_msb(&mut cmd, input, input_path, result_path);

    cmd.arg(image);
    cmd.arg("--");
    cmd.arg("node");
    cmd.arg(handler_abs);
    Ok(cmd)
}

fn apply_takt_env(
    cmd: &mut Command,
    input: &ExecutionInput,
    input_path: &Path,
    result_path: &Path,
) {
    cmd.env("TAKT_RUN_ID", &input.run_id);
    cmd.env("TAKT_CAPABILITY", &input.capability);
    cmd.env("TAKT_PACKAGE_ROOT", &input.package_root);
    cmd.env("TAKT_INPUT_PATH", input_path);
    cmd.env("TAKT_RESULT_PATH", result_path);
}

fn apply_takt_env_via_msb(
    cmd: &mut Command,
    input: &ExecutionInput,
    input_path: &Path,
    result_path: &Path,
) {
    cmd.arg("--env")
        .arg(format!("TAKT_RUN_ID={}", input.run_id));
    cmd.arg("--env")
        .arg(format!("TAKT_CAPABILITY={}", input.capability));
    cmd.arg("--env").arg(format!(
        "TAKT_PACKAGE_ROOT={}",
        input.package_root.display()
    ));
    cmd.arg("--env")
        .arg(format!("TAKT_INPUT_PATH={}", input_path.display()));
    cmd.arg("--env")
        .arg(format!("TAKT_RESULT_PATH={}", result_path.display()));
}

fn sandbox_binary_not_found_error(runtime: &RuntimeProfile) -> color_eyre::Report {
    match runtime.sandbox.as_str() {
        SANDBOX_PROCESS => eyre!(
            "Node binary 'node' was not found on PATH. Install Node or use a runtime profile with sandbox='microsandbox'."
        ),
        SANDBOX_MICROSANDBOX => eyre!(
            "Microsandbox CLI 'msb' was not found on PATH. Install from https://microsandbox.dev (one-liner: `curl -fsSL https://install.microsandbox.dev | sh`) or switch the runtime profile back to sandbox='process'."
        ),
        other => eyre!("sandbox '{}' driver binary not found on PATH", other),
    }
}

fn read_tail(path: &Path, max_bytes: usize) -> Result<String> {
    let bytes = fs::read(path)?;
    let start = bytes.len().saturating_sub(max_bytes);
    Ok(String::from_utf8_lossy(&bytes[start..]).to_string())
}

fn sanitize_id(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}
