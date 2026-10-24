use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

const MANIFEST: &str = "../../fixtures/workloads/manifest.json";

#[derive(Clone, Debug)]
struct FixtureManifest {
    seed: u64,
    fixtures: Vec<String>,
    profiles: Value,
}

#[derive(Clone, Debug)]
struct Record {
    path: String,
    bytes: usize,
    sha256: String,
}

type FileEntries = Vec<(String, Vec<u8>)>;

pub fn run<I>(args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = OsString>,
{
    let parsed = parse_args(args)?;
    let manifest = load_manifest()?;
    if parsed.fixture != "all" && !manifest.fixtures.iter().any(|id| id == &parsed.fixture) {
        return Err(format!("unknown fixture: {}", parsed.fixture).into());
    }
    let sizes = profile_sizes(&manifest.profiles, &parsed.profile)?;
    if parsed.out.exists() && fs::read_dir(&parsed.out)?.next().is_some() {
        return Err(format!("output directory is not empty: {}", parsed.out.display()).into());
    }
    fs::create_dir_all(&parsed.out)?;

    let selected: Vec<&str> = if parsed.fixture == "all" {
        manifest.fixtures.iter().map(String::as_str).collect()
    } else {
        vec![parsed.fixture.as_str()]
    };
    let mut summaries = Vec::new();
    for fixture in selected {
        summaries.push(generate(
            fixture,
            &parsed.profile,
            manifest.seed,
            &sizes,
            &parsed.out.join(fixture),
        )?);
    }
    println!("{}", serde_json::to_string_pretty(&summaries)?);
    Ok(())
}

#[derive(Debug)]
struct Args {
    fixture: String,
    profile: String,
    out: PathBuf,
}

fn parse_args<I>(args: I) -> Result<Args, Box<dyn Error>>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let fixture = args
        .next()
        .ok_or("usage: workload-fixtures <fixture|all> --out DIR [--profile default|extended]")?;
    let mut out = None;
    let mut profile = "default".to_owned();
    while let Some(flag) = args.next() {
        match flag.to_str() {
            Some("--out") => {
                out = Some(PathBuf::from(
                    args.next().ok_or("--out requires a directory")?,
                ))
            }
            Some("--profile") => {
                profile = args
                    .next()
                    .ok_or("--profile requires a value")?
                    .into_string()
                    .map_err(|_| "profile must be valid UTF-8")?
            }
            Some(other) => return Err(format!("unknown argument: {other}").into()),
            None => return Err("arguments must be valid UTF-8".into()),
        }
    }
    Ok(Args {
        fixture: fixture
            .into_string()
            .map_err(|_| "fixture ID must be valid UTF-8")?,
        profile,
        out: out.ok_or("--out is required")?,
    })
}

fn load_manifest() -> Result<FixtureManifest, Box<dyn Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(MANIFEST);
    let raw: Value = serde_json::from_slice(&fs::read(path)?)?;
    if raw["manifest_version"].as_u64() != Some(1) {
        return Err("unsupported workload manifest version".into());
    }
    let seed = raw["seed"]
        .as_u64()
        .ok_or("manifest seed must be a non-negative integer")?;
    let fixtures_raw = raw["fixtures"]
        .as_array()
        .ok_or("manifest fixtures must be an array")?;
    let fixtures: Vec<String> = fixtures_raw
        .iter()
        .map(|fixture| {
            fixture["id"]
                .as_str()
                .map(str::to_owned)
                .ok_or("fixture id must be a string")
        })
        .collect::<Result<_, _>>()?;
    let unique: BTreeSet<&str> = fixtures.iter().map(String::as_str).collect();
    if fixtures.is_empty() || unique.len() != fixtures.len() {
        return Err("fixture IDs must be unique and non-empty".into());
    }
    for id in &fixtures {
        if !matches!(
            id.as_str(),
            "tiny-graph"
                | "wide-tree"
                | "deep-tree"
                | "binary-mix"
                | "many-repos"
                | "hot-repo"
                | "long-ci-waits"
        ) {
            return Err(format!("manifest names unsupported fixture ID: {id}").into());
        }
    }
    let profiles = raw["profiles"].clone();
    if !profiles.is_object() {
        return Err("manifest profiles must be an object".into());
    }
    Ok(FixtureManifest {
        seed,
        fixtures,
        profiles,
    })
}

#[derive(Clone, Copy, Debug)]
struct Sizes {
    wide: usize,
    depth: usize,
    binary: usize,
    repos: usize,
    hot: usize,
    waits: usize,
}

fn profile_sizes(profiles: &Value, profile: &str) -> Result<Sizes, Box<dyn Error>> {
    let values = profiles
        .get(profile)
        .ok_or_else(|| format!("unknown profile: {profile}"))?;
    let number = |field: &str| -> Result<usize, Box<dyn Error>> {
        let n = values[field]
            .as_u64()
            .ok_or_else(|| format!("profile {profile} requires positive integer {field}"))?;
        if n == 0 || n > usize::MAX as u64 {
            return Err(format!("profile {profile} has invalid size for {field}").into());
        }
        Ok(n as usize)
    };
    Ok(Sizes {
        wide: number("wide_files")?,
        depth: number("deep_directory_components")?,
        binary: number("binary_objects")?,
        repos: number("repositories")?,
        hot: number("hot_repository_files")?,
        waits: number("long_waits")?,
    })
}

fn generate(
    fixture: &str,
    profile: &str,
    seed: u64,
    sizes: &Sizes,
    out: &Path,
) -> Result<Value, Box<dyn Error>> {
    let entries = file_entries(fixture, seed, sizes)?;
    let mut records = Vec::with_capacity(entries.len());
    for (relative, bytes) in entries {
        let target = out.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, &bytes)?;
        records.push(Record {
            path: relative,
            bytes: bytes.len(),
            sha256: hex(&Sha256::digest(&bytes)),
        });
    }
    records.sort_by(|a, b| a.path.cmp(&b.path));
    let record_values: Vec<Value> = records
        .iter()
        .map(|r| json!({"path":r.path,"bytes":r.bytes,"sha256":r.sha256}))
        .collect();
    let canonical = serde_json::to_vec(&record_values)?;
    let total_bytes: usize = records.iter().map(|record| record.bytes).sum();
    let summary = json!({
        "fixture_id": fixture,
        "profile": profile,
        "seed": seed,
        "file_count": records.len(),
        "total_bytes": total_bytes,
        "content_sha256": hex(&Sha256::digest(&canonical)),
        "files": record_values,
    });
    fs::write(
        out.join("fixture.json"),
        format!("{}\n", serde_json::to_string_pretty(&summary)?),
    )?;
    Ok(summary)
}

fn file_entries(fixture: &str, seed: u64, sizes: &Sizes) -> Result<FileEntries, Box<dyn Error>> {
    let entries = match fixture {
        "tiny-graph" => {
            let nodes = json!([
                {"id":"root","parents":[],"tree":"t0"},
                {"id":"left-1","parents":["root"],"tree":"t1"},
                {"id":"left-2","parents":["left-1"],"tree":"t2"},
                {"id":"right-1","parents":["root"],"tree":"t3"},
                {"id":"merge","parents":["left-2","right-1"],"tree":"t4"}
            ]);
            vec![(
                "graph.json".to_owned(),
                serde_json::to_vec(&json!({"seed":seed,"nodes":nodes}))?,
            )]
        }
        "wide-tree" => (0..sizes.wide)
            .map(|i| {
                (
                    format!("tree/file-{i:05}.txt"),
                    format!("wide fixture seed={seed} index={i}\n").into_bytes(),
                )
            })
            .collect(),
        "deep-tree" => {
            let path = (0..sizes.depth)
                .map(|i| format!("d{i:03}"))
                .collect::<Vec<_>>()
                .join("/")
                + "/leaf.txt";
            vec![(
                path,
                format!("deep fixture seed={seed} depth={}\n", sizes.depth).into_bytes(),
            )]
        }
        "binary-mix" => {
            let mut rng = SplitMix64::new(seed);
            let mut files = Vec::with_capacity(sizes.binary + 1);
            for i in 0..sizes.binary {
                let bytes = (0..1024).map(|_| rng.next_u64() as u8).collect();
                files.push((format!("binary/blob-{i:04}.bin"), bytes));
            }
            files.push((
                "binary/readme.txt".to_owned(),
                b"Synthetic binary share; payloads use SplitMix64 bytes.\n".to_vec(),
            ));
            files
        }
        "many-repos" => (0..sizes.repos)
            .map(|i| {
                (
                    format!("repo-{i:05}/README.txt"),
                    format!("repository={i} seed={seed}\n").into_bytes(),
                )
            })
            .collect(),
        "hot-repo" => (0..sizes.hot)
            .map(|i| {
                (
                    format!("hot/src/file-{i:05}.rs"),
                    format!("// synthetic hot-repository path {i}\n").into_bytes(),
                )
            })
            .collect(),
        "long-ci-waits" => {
            let waits: Vec<Value> = (0..sizes.waits).map(|i| json!({
                "id":format!("wait-{i:05}"),
                "duration_seconds":30 * (i + 1),
                "callback_order":if i % 2 == 0 {"approval-before-ci"} else {"ci-before-approval"},
            })).collect();
            vec![(
                "waits.json".to_owned(),
                serde_json::to_vec(&json!({"seed":seed,"waits":waits}))?,
            )]
        }
        _ => return Err(format!("unsupported fixture: {fixture}").into()),
    };
    Ok(entries)
}

/// SplitMix64 stream; one low byte is emitted per output byte.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "workload-fixtures-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn summaries_match_independent_payload_scan_for_every_fixture() {
        let manifest = load_manifest().unwrap();
        let sizes = profile_sizes(&manifest.profiles, "default").unwrap();
        let root = temp_dir("hashes");
        for fixture in manifest.fixtures {
            let directory = root.join(&fixture);
            let summary = generate(&fixture, "default", manifest.seed, &sizes, &directory).unwrap();
            let mut records = Vec::new();
            for entry in walk_files(&directory) {
                let path = entry
                    .strip_prefix(&directory)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if path == "fixture.json" {
                    continue;
                }
                let payload = fs::read(&entry).unwrap();
                records.push(json!({"path":path,"bytes":payload.len(),"sha256":hex(&Sha256::digest(payload))}));
            }
            records.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
            assert_eq!(summary["files"], json!(records));
            assert_eq!(
                summary["file_count"].as_u64().unwrap() as usize,
                records.len()
            );
            let total: usize = records
                .iter()
                .map(|r| r["bytes"].as_u64().unwrap() as usize)
                .sum();
            assert_eq!(summary["total_bytes"].as_u64().unwrap() as usize, total);
            assert_eq!(
                summary["content_sha256"],
                hex(&Sha256::digest(serde_json::to_vec(&records).unwrap()))
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tiny_graph_is_an_ordered_fork_and_merge_dag() {
        let manifest = load_manifest().unwrap();
        let sizes = profile_sizes(&manifest.profiles, "default").unwrap();
        let root = temp_dir("graph");
        let directory = root.join("graph");
        generate("tiny-graph", "default", manifest.seed, &sizes, &directory).unwrap();
        let graph: Value =
            serde_json::from_slice(&fs::read(directory.join("graph.json")).unwrap()).unwrap();
        let nodes = graph["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 5);
        let mut seen = BTreeSet::new();
        for node in nodes {
            for parent in node["parents"].as_array().unwrap() {
                assert!(seen.contains(parent.as_str().unwrap()));
            }
            seen.insert(node["id"].as_str().unwrap().to_owned());
        }
        let by_id: std::collections::BTreeMap<&str, &Value> = nodes
            .iter()
            .map(|n| (n["id"].as_str().unwrap(), n))
            .collect();
        assert_eq!(by_id["left-1"]["parents"], json!(["root"]));
        assert_eq!(by_id["right-1"]["parents"], json!(["root"]));
        assert_eq!(by_id["merge"]["parents"], json!(["left-2", "right-1"]));
        fs::remove_dir_all(root).unwrap();
    }

    fn walk_files(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for entry in fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_files(&path));
            } else {
                files.push(path);
            }
        }
        files
    }
}
