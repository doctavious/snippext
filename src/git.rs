// https://github.com/simeg/eureka/blob/master/src/git.rs
// https://github.com/crate-ci/cargo-release/blob/master/src/git.rs

use std::env;
use git2::{Cred, Error, RemoteCallbacks, Repository};
use std::path::Path;
use std::process::Command;
use crate::error::SnippextError;
use crate::SnippextResult;

pub trait GitManagement {
    fn checkout_branch(&self, branch_name: &str) -> Result<(), git2::Error>;
    // fn add(&self) -> Result<(), git2::Error>;
    // fn commit(&self, subject: &str) -> Result<git2::Oid, git2::Error>;
    // fn push(&self, branch_name: &str) -> Result<(), git2::Error>;
}

pub struct Git {
    repo: Option<git2::Repository>,
}

impl GitManagement for Git {
    fn checkout_branch(&self, branch_name: &str) -> Result<(), Error> {
        todo!()
    }
}

// TODO: Do we need to allow users to specify path to clone to and path of ssh creds?
// sparse clone / depth 1?
// git2-rs doesnt appear to support sparse checkout, yet, because lib2git doesnt
fn git_clone(remote: &str) {
    // HTTP clone
    let repo = match Repository::clone(remote, "/path/to/a/repo") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };

    // SSH clone
    // Prepare callbacks.
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
            None,
        )
    });

    // Prepare fetch options.
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);

    // Prepare builder.
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    // let mut checkout_builder = CheckoutBuilder::new()

    // Clone the project.
    builder.clone(
        "git@github.com:rust-lang/git2-rs.git",
        Path::new("/tmp/git2-rs"),
    );
}

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

