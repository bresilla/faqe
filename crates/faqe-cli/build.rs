use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED: &[&str] = &[
    "faqe_web.js",
    "faqe_web_bg.wasm",
    "LICENSE",
    "THIRD_PARTY.md",
];

fn main() {
    println!("cargo:rerun-if-env-changed=FAQE_EMBED_DIR");
    let root = env::var_os("FAQE_EMBED_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("FAQE_EMBED_DIR is unset; build faqe-cli through `make build`"));
    for required in REQUIRED {
        let path = root.join(required);
        assert!(
            path.is_file(),
            "embedded runtime is incomplete: {} is missing; run `make web-bundle`",
            path.display()
        );
    }
    assert!(
        root.join("licenses").is_dir(),
        "embedded runtime is incomplete: licenses/ is missing; run `make web-bundle`"
    );
    let wasm = fs::read(root.join("faqe_web_bg.wasm")).expect("read staged WASM");
    assert!(
        wasm.len() >= 8 && &wasm[..4] == b"\0asm",
        "staged faqe_web_bg.wasm has an invalid magic header"
    );

    let mut paths = REQUIRED.iter().map(PathBuf::from).collect::<Vec<_>>();
    collect_files(&root, &root.join("licenses"), &mut paths);
    paths.sort();
    let mut generated = format!(
        "pub const EMBEDDED_SCHEMA_VERSION: u32 = {};\n\
         pub const EMBEDDED_BUILD_MODE: &str = ",
        faqe_model::SITE_SCHEMA_VERSION
    );
    generated.push_str(&format!("{:?};\n", env::var("PROFILE").unwrap_or_default()));
    generated.push_str("pub const EMBEDDED_ENTRIES: &[(&str, usize, &str)] = &[\n");
    for relative in paths {
        let absolute = root.join(&relative);
        let bytes = fs::read(&absolute).unwrap_or_else(|error| {
            panic!(
                "could not read embedded file {}: {error}",
                absolute.display()
            )
        });
        println!("cargo:rerun-if-changed={}", absolute.display());
        generated.push_str(&format!(
            "    ({:?}, {}, {:?}),\n",
            slash_path(&relative),
            bytes.len(),
            hex_digest(&bytes)
        ));
    }
    generated.push_str("];\n");
    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("embedded_manifest.rs");
    fs::write(output, generated).expect("write embedded runtime manifest");
}

fn collect_files(root: &Path, directory: &Path, output: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", directory.display()))
        .map(|entry| entry.expect("read embedded directory entry"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, output);
        } else if path.is_file() {
            output.push(
                path.strip_prefix(root)
                    .expect("embedded relative path")
                    .to_owned(),
            );
        }
    }
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
