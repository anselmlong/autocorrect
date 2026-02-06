use self_update::cargo_crate_version;
use std::error::Error;

const REPO_OWNER: &str = "anselmlong";
const REPO_NAME: &str = "autocorrect";
const BIN_NAME: &str = "autocorrect";

pub struct Updater;

impl Updater {
    pub fn check_and_update() -> Result<bool, Box<dyn Error>> {
        let status = self_update::backends::github::Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name(BIN_NAME)
            .show_download_progress(true)
            .show_output(false)
            .no_confirm(false)
            .current_version(cargo_crate_version!())
            .build()?
            .update()?;

        match status {
            self_update::Status::UpToDate(v) => {
                println!("Already up to date: {}", v);
                Ok(false)
            }
            self_update::Status::Updated(v) => {
                println!("Updated to version: {}", v);
                Ok(true)
            }
        }
    }

    pub fn check_version() -> Result<Option<String>, Box<dyn Error>> {
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .build()?
            .fetch()?;

        if let Some(latest) = releases.first() {
            let current = cargo_crate_version!();
            let latest_version = latest.version.trim_start_matches('v');
            if latest_version != current {
                Ok(Some(latest.version.clone()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
