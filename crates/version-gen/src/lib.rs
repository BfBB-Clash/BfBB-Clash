//! A simple build-script library to monitor git repo changes and generate a version number based on repo status.

use std::path::Path;

use git2::Repository;

/// Set a build-time env var `CLASH_VERSION` that contains the annotated version number.
///
/// If built from a clean repo on a tagged commit, the version number is simply `CARGO_PKG_VERSION`,
/// otherwise the version number will be suffixed with the short hash of the current commit and `-dirty`
/// if there are uncommitted changes.
///
/// `manifest_path` is the path to the project being built. This is required to recreate cargo's default
/// re-run behavior for build scripts.
pub fn gen_clash_version(manifest_path: impl AsRef<Path>) -> anyhow::Result<()> {
    let repo = Repository::open_from_env()?;
    let path = repo.path();
    let head_path = path.join("HEAD");
    let head_ref = repo.find_reference("HEAD")?.resolve()?;
    let ref_path = path.join(head_ref.name().unwrap());
    let tags_path = path.join("refs/tags");
    assert!(head_path.exists());
    assert!(ref_path.exists());
    assert!(tags_path.exists());
    println!("cargo:rerun-if-changed={}", head_path.display());
    println!("cargo:rerun-if-changed={}", ref_path.display());
    println!("cargo:rerun-if-changed={}", tags_path.display());
    monitor_files(manifest_path)?;

    // This could probably also be done through git2 but I can't be bothered to figure it out right now.
    // This should be fine.
    let dirty = !std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()?
        .stdout
        .is_empty();
    let tag = std::process::Command::new("git")
        .args(["tag", "--points-at=HEAD"])
        .output()?
        .stdout;
    let tag = String::from_utf8(tag)?;
    match (tag.is_empty(), dirty) {
        (false, false) => {
            println!(
                "cargo:rustc-env=CLASH_VERSION={}",
                env!("CARGO_PKG_VERSION")
            );
        }
        _ => {
            let des = std::process::Command::new("git")
                .args(["describe", "--always", "--dirty=-dirty"])
                .output()?
                .stdout;
            let des = String::from_utf8(des)?;
            println!(
                "cargo:rustc-env=CLASH_VERSION={}-{}",
                env!("CARGO_PKG_VERSION"),
                des
            );
        }
    }

    Ok(())
}

// TODO: This is dumb as hell.
//       Adding the git repo files to "rerun-if-changed" disables the default behavior and there's no way to get it back for now.
fn monitor_files(dir: impl AsRef<Path>) -> std::io::Result<()> {
    for ent in std::fs::read_dir(dir)? {
        let ent = ent?;
        if ent.file_type()?.is_dir() {
            monitor_files(ent.path())?;
        } else {
            println!("cargo:rerun-if-changed={}", ent.path().display());
        }
    }

    Ok(())
}
