use base64::Engine;
use color_eyre::eyre::{Result, bail, eyre};
use flate2::{Compression, write::GzEncoder};
use serde_json::json;
use sha2::{Digest, Sha512};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tar::{Builder, Header};
use tempfile::tempdir;

#[test]
fn install_resolves_registry_dependency_into_store_and_project_skills() -> Result<()> {
    let temp = tempdir()?;
    let root = temp.path().join("root");
    fs::create_dir_all(&root)?;
    fs::create_dir_all(root.join(".takt/store/v1/files"))?;
    fs::create_dir_all(root.join(".takt/cache"))?;

    write_json(
        &root.join("takt.json"),
        json!({
            "api_version": "takt.dev/v1alpha1",
            "name": "@acme/root",
            "version": "0.1.0",
            "node": "22.12.0",
            "capabilities": {}
        }),
    )?;
    write_json(
        &root.join("package.json"),
        json!({
            "name": "@acme/root",
            "version": "0.1.0",
            "dependencies": {
                "@acme/dep": "^1.0.0"
            }
        }),
    )?;

    let tarball = package_tarball()?;
    let integrity = format!(
        "sha512-{}",
        base64::engine::general_purpose::STANDARD.encode(Sha512::digest(&tarball))
    );
    let registry = LocalRegistry::start(tarball, integrity.clone())?;
    fs::write(
        root.join(".npmrc"),
        format!("registry={}\n", registry.url()),
    )?;

    let home = temp.path().join("home");
    let xdg_data_home = temp.path().join("xdg-data");
    let xdg_cache_home = temp.path().join("xdg-cache");
    fs::create_dir_all(&home)?;

    run_takt(
        &[
            "--dir",
            root.to_str().ok_or_else(|| eyre!("non-utf8 root path"))?,
            "--format",
            "json",
            "install",
        ],
        &home,
        &xdg_data_home,
        &xdg_cache_home,
    )?;

    let lockfile: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("takt.lock.json"))?)?;
    assert_eq!(
        lockfile["packages"]["@acme/dep"]["version"],
        serde_json::Value::String("1.2.3".into())
    );
    assert_eq!(
        lockfile["packages"]["@acme/dep"]["integrity"],
        serde_json::Value::String(integrity)
    );

    assert!(
        root.join(".takt/store/v1/files")
            .read_dir()?
            .next()
            .is_some()
    );
    assert!(
        root.join(".takt/store/v1/index")
            .read_dir()?
            .next()
            .is_some()
    );
    assert!(
        root.join(".takt/cache/virtual-store")
            .read_dir()?
            .next()
            .is_some()
    );

    let projected_skill = root
        .join(".agents/skills")
        .join("takt-dep--acme-dep--dep-skill")
        .join("SKILL.md");
    assert!(
        projected_skill.exists(),
        "missing {}",
        projected_skill.display()
    );

    fs::create_dir_all(root.join("actions"))?;
    write_json(
        &root.join("actions/echo.json"),
        json!({
            "api_version": "takt.dev/v1alpha1",
            "kind": "Action",
            "name": "echo",
            "capability": "@acme/dep#echo.run"
        }),
    )?;

    run_takt(
        &[
            "--dir",
            root.to_str().ok_or_else(|| eyre!("non-utf8 root path"))?,
            "--format",
            "json",
            "validate",
            "action",
            "echo",
        ],
        &home,
        &xdg_data_home,
        &xdg_cache_home,
    )?;

    let output = run_takt(
        &[
            "--dir",
            root.to_str().ok_or_else(|| eyre!("non-utf8 root path"))?,
            "--format",
            "json",
            "run",
            "action",
            "echo",
            "--no-persist",
        ],
        &home,
        &xdg_data_home,
        &xdg_cache_home,
    )?;
    assert!(output.contains("\"package\": \"@acme/dep\""));
    assert!(output.contains("\"capability\": \"echo.run\""));

    Ok(())
}

struct LocalRegistry {
    address: String,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl LocalRegistry {
    fn start(tarball: Vec<u8>, integrity: String) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?.to_string();
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let tarball = Arc::new(tarball);
        let packument = Arc::new(packument_json(&address, &integrity)?);

        let handle = thread::spawn(move || {
            while !thread_shutdown.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = handle_request(stream, &packument, &tarball);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            address,
            shutdown,
            handle: Some(handle),
        })
    }

    fn url(&self) -> String {
        format!("http://{}/", self.address)
    }
}

impl Drop for LocalRegistry {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(&self.address);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_takt(
    args: &[&str],
    home: &Path,
    xdg_data_home: &Path,
    xdg_cache_home: &Path,
) -> Result<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_takt"))
        .args(args)
        .env("HOME", home)
        .env("XDG_DATA_HOME", xdg_data_home)
        .env("XDG_CACHE_HOME", xdg_cache_home)
        .output()?;

    if !output.status.success() {
        bail!(
            "takt {:?} failed with {}\nstdout:\n{}\nstderr:\n{}",
            args,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn handle_request(mut stream: TcpStream, packument: &str, tarball: &[u8]) -> Result<()> {
    let mut request = [0_u8; 4096];
    let bytes_read = stream.read(&mut request)?;
    let request = String::from_utf8_lossy(&request[..bytes_read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| eyre!("missing request path"))?;
    let normalized_path = path.to_ascii_lowercase();

    if normalized_path == "/@acme%2fdep" {
        write_response(stream, "200 OK", "application/json", packument.as_bytes())?;
    } else if normalized_path == "/tarballs/dep-1.2.3.tgz" {
        write_response(stream, "200 OK", "application/octet-stream", tarball)?;
    } else {
        write_response(stream, "404 Not Found", "text/plain", b"not found")?;
    }

    Ok(())
}

fn write_response(
    mut stream: TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}

fn packument_json(address: &str, integrity: &str) -> Result<String> {
    Ok(serde_json::to_string(&json!({
        "dist-tags": {
            "latest": "1.2.3"
        },
        "versions": {
            "1.2.3": {
                "dist": {
                    "tarball": format!("http://{address}/tarballs/dep-1.2.3.tgz"),
                    "integrity": integrity
                }
            }
        }
    }))?)
}

fn package_tarball() -> Result<Vec<u8>> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(encoder);
    append_file(
        &mut builder,
        "package/takt.json",
        serde_json::to_vec_pretty(&json!({
            "api_version": "takt.dev/v1alpha1",
            "name": "@acme/dep",
            "version": "1.2.3",
            "node": "22.12.0",
            "capabilities": {
                "echo.run": {
                    "description": "Echo",
                    "handler": {
                        "entrypoint": "handlers/echo.mjs",
                        "argv": []
                    },
                    "input": {
                        "path": "schemas/input.json"
                    },
                    "output": {
                        "path": "schemas/output.json"
                    },
                    "permissions": {}
                }
            }
        }))?,
        0o644,
    )?;
    append_file(
        &mut builder,
        "package/package.json",
        serde_json::to_vec_pretty(&json!({
            "name": "@acme/dep",
            "version": "1.2.3"
        }))?,
        0o644,
    )?;
    append_file(
        &mut builder,
        "package/handlers/echo.mjs",
        b"export default async function echo(input) { return input; }\n".to_vec(),
        0o644,
    )?;
    append_file(
        &mut builder,
        "package/schemas/input.json",
        b"{\"type\":\"object\"}\n".to_vec(),
        0o644,
    )?;
    append_file(
        &mut builder,
        "package/schemas/output.json",
        b"{\"type\":\"object\"}\n".to_vec(),
        0o644,
    )?;
    append_file(
        &mut builder,
        "package/.agents/skills/dep-skill/SKILL.md",
        b"---\nname: dep-skill\ndescription: Dependency skill\n---\n".to_vec(),
        0o644,
    )?;
    let encoder = builder.into_inner()?;
    Ok(encoder.finish()?)
}

fn append_file(
    builder: &mut Builder<GzEncoder<Vec<u8>>>,
    path: &str,
    content: Vec<u8>,
    mode: u32,
) -> Result<()> {
    let mut header = Header::new_gnu();
    header.set_path(path)?;
    header.set_size(content.len() as u64);
    header.set_mode(mode);
    header.set_cksum();
    builder.append(&header, content.as_slice())?;
    Ok(())
}

fn write_json(path: &PathBuf, value: serde_json::Value) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&value)?))?;
    Ok(())
}
