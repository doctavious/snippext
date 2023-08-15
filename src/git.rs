// https://github.com/simeg/eureka/blob/master/src/git.rs
// https://github.com/crate-ci/cargo-release/blob/master/src/git.rs

use std::path::PathBuf;
use std::process::Command;

use crate::error::SnippextError;
use crate::SnippextResult;

pub(crate) fn checkout_files(
    remote: &String,
    branch: Option<String>,
    cone_patterns: Option<Vec<String>>,
    dir: &PathBuf,
) -> SnippextResult<()> {
    // if cone_patterns are specified lets do a no-checkout clone with a sparse-checkout
    // otherwise just do a regular clone
    let mut clone_command = Command::new("git");
    clone_command.arg("clone");

    if cone_patterns.is_some() {
        clone_command.arg("--no-checkout");
    }

    if let Some(branch) = branch {
        clone_command.arg("--branch").arg(branch);
    }

    let clone_output = clone_command
        .arg(remote)
        .arg(".")
        .current_dir(&dir)
        .output()
        .map_err(SnippextError::from)?;

    if !clone_output.status.success() {
        return Err(SnippextError::GeneralError(String::from_utf8(
            clone_output.stderr,
        )?));
    }

    if cone_patterns.is_some() {
        let sparse_checkout_init = Command::new("git")
            .arg("sparse-checkout")
            .arg("init")
            .arg("--cone")
            .current_dir(&dir)
            .output()
            .map_err(SnippextError::from)?;

        if !sparse_checkout_init.status.success() {
            return Err(SnippextError::GeneralError(String::from_utf8(
                sparse_checkout_init.stderr,
            )?));
        }

        let sparse_checkout_set = Command::new("git")
            .arg("sparse-checkout")
            .arg("set")
            .arg(cone_patterns.unwrap().join(" "))
            .current_dir(&dir)
            .output()
            .map_err(SnippextError::from)?;

        if !sparse_checkout_set.status.success() {
            return Err(SnippextError::GeneralError(String::from_utf8(
                sparse_checkout_set.stderr,
            )?));
        }
    }

    Ok(())
}

pub(crate) fn get_remote_url() -> SnippextResult<String> {
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("--all origin")
        .current_dir(".")
        .output()
        .map_err(SnippextError::from)?;

    if !output.status.success() {
        return Err(SnippextError::GeneralError(String::from_utf8(
            output.stderr,
        )?));
    }

    let remote_url = String::from_utf8(output.stdout)?;

    // TODO: parse

    Ok(remote_url)
}
