// https://github.com/simeg/eureka/blob/master/src/git.rs
// https://github.com/crate-ci/cargo-release/blob/master/src/git.rs

use std::env;
use std::path::Path;
use std::process::Command;
use crate::error::SnippextError;
use crate::SnippextResult;

pub fn checkout_files(
    remote: &str,
    branch: Option<&str>,
    cone_patterns: Option<Vec<String>>,
    dir: &Path
) -> SnippextResult<()> {
    // if cone_patterns are specified lets do a no-checkout clone with a sparse-checkout
    // otherwise just do a regular clone
    let mut clone_command = Command::new("git")
        .arg("clone");

    if cone_patterns.is_some() {
        clone_command.arg("--no-checkout");
    }

    if let Some(branch) = branch {
        clone_command.arg("--branch").arg(branch);
    }

    clone_command
        .arg(remote)
        .arg(dir)
        .current_dir("./")
        .output()
        .map_err(SnippextError::from)?;


    if cone_patterns.is_some() {
        Command::new("git")
            .arg("sparse-checkout")
            .arg("init")
            .arg("--cone")
            .current_dir(dir)
            .output()
            .map_err(SnippextError::from)?;

        Command::new("git")
            .arg("sparse-checkout")
            .arg("set")
            .arg(cone_patterns.unwrap().join(" "))
            .current_dir(dir)
            .output()
            .map_err(SnippextError::from)?;
    }

    Ok(())
}

