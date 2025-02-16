#[macro_use]
extern crate clap;
#[macro_use]
extern crate strum_macros;

mod catch;
mod config;
mod git;
mod init;
mod install;
mod package;
mod repo;

use anyhow::Result;
use clap::{App, AppSettings, Arg, SubCommand};
use colored::*;

use config::Config;
use repo::Repo;

fn clap_app<'a, 'b>() -> App<'a, 'b> {
    App::new("emplace")
        .version(crate_version!())
        .author(crate_authors!())
        .after_help("https://github.com/tversteeg/emplace")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("init")
            .about("Prints the shell function used to execute emplace")
            .arg(
                Arg::with_name("shell")
                .value_name("SHELL")
                .help(
                    "The name of the currently running shell\nCurrently supported options: bash",
                )
                .required(true)
            )
        )
        .subcommand(
            SubCommand::with_name("catch")
            .about("Capture a command entired in a terminal")
            .arg(
                Arg::with_name("line")
                .value_name("LINE")
                .help(
                    "The command as entired in the terminal",
                )
                .required(true)
            )
        )
        .subcommand(
            SubCommand::with_name("install")
            .about("Install the packages that have been mirrored from other machines")
        )
}

fn main() -> Result<()> {
    let matches = clap_app().get_matches();

    match matches.subcommand() {
        ("init", Some(sub_m)) => {
            let shell_name = sub_m.value_of("shell").expect("Shell name is missing.");
            init::init_main(shell_name).expect("Could not initialize terminal script");
        }
        ("catch", Some(sub_m)) => {
            let line = sub_m.value_of("line").expect("Line is missing");
            let mut catches = catch::catch(line).expect("Could not parse line");

            if catches.0.is_empty() {
                // Nothing found, just return
                return Ok(());
            }

            // Filter out the packages that are already in the repository
            // Get the config
            let config = match Config::from_default_file().expect("Retrieving config went wrong") {
                Some(config) => config,
                None => Config::new().expect("Initializing new config failed"),
            };
            let repo = Repo::new(config).expect("Could not initialize git repository");
            catches.filter_saved_packages(
                &repo
                    .read()
                    .expect("Could not read packages file from repository"),
            );

            let len = catches.0.len();
            if len == 0 {
                // Nothing found after filtering
                return Ok(());
            }

            // Print the info
            match len {
                1 => println!("{}", "Mirror this command?".green().bold()),
                n => println!("{}", format!("Mirror these {} commands?", n).green().bold()),
            }
            for catch in catches.0.iter() {
                println!("- {}", catch.colour_full_name());
            }

            // Ask if it needs to be mirrored
            if !dialoguer::Confirmation::new()
                .interact()
                .expect("Could not create dialogue")
            {
                // Exit, we don't need to do anything
                return Ok(());
            }

            repo.mirror(catches).expect("Could not mirror commands");
        }
        ("install", Some(_)) => {
            // Get the config
            let config = match Config::from_default_file().expect("Retrieving config went wrong") {
                Some(config) => config,
                None => Config::new().expect("Initializing new config failed"),
            };

            let repo = Repo::new(config).expect("Could not initialize git repository");

            match repo.read() {
                Ok(packages) => {
                    if let Err(err) = crate::install::install(packages) {
                        println!(
                            "{}",
                            format!("Could not install new changes: {}", err)
                                .red()
                                .bold()
                        );
                    }
                }
                Err(err) => println!("{}", format!("Error: {}", err).red().bold()),
            };
        }
        (&_, _) => {}
    };

    Ok(())
}
