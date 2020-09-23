mod app;
mod helper;
use app::*;
use chrono::{Local, TimeZone};
use clap::{Clap, ValueHint};
use console::style;
use helper::get_pb;
use prettytable::{cell, row, Table};
use std::{
    path::Path,
    thread::sleep,
    time::{Duration, Instant},
};
use subprocess::Exec;

#[derive(Clap)]
#[clap(
    version = "1.0",
    author = "Saanu Reghunadh",
    about = "Windows Hello™ style facial authentication for Linux written in Rust"
)]
struct Opts {
    #[clap(short, long, value_hint=ValueHint::Username, env="SUDO_USER", default_value=" ")]
    user: String,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(about = "Face model related commands")]
    Model(ModelOpts),
    #[clap(about = "Open configuration file in default text editor")]
    Config(EmptyOpts),
}

#[derive(Clap)]
struct ModelOpts {
    #[clap(subcommand)]
    subcmd: ModelSubCommand,
}

#[derive(Clap)]
enum ModelSubCommand {
    #[clap(about = "Add a face model")]
    Add(InputStringOpts),
    #[clap(about = "Remove a face model")]
    Remove(InputIntOpts),
    #[clap(about = "List all face models")]
    List(EmptyOpts),
    #[clap(about = "Test against all face models")]
    Check(EmptyOpts),
    #[clap(about = "Clear all face models")]
    Clear(EmptyOpts),
}

#[derive(Clap)]
struct InputStringOpts {
    #[clap(about = "Label for the face model")]
    label: String,
}

#[derive(Clap)]
struct InputIntOpts {
    #[clap(about = "ID of the face model")]
    id: usize,
}

#[derive(Clap)]
struct EmptyOpts {}

fn main() {
    let opts: Opts = Opts::parse();
    if opts.user.trim().is_empty() {
        return println!("Please run this command as root");
    } else {
        if opts.user == "root" {
            return println!("User can't be root");
        }
    }
    let base_path = Path::new("/lib/security/pam_hola");
    match opts.subcmd {
        // All model related command
        SubCommand::Model(o) => {
            let pb = get_pb();
            match o.subcmd {
                // Add a face model command
                ModelSubCommand::Add(x) => {
                    pb.set_message(&format!(
                        "Adding model for user {}",
                        style(&opts.user).bold().blue()
                    ));
                    sleep(Duration::from_secs(1));
                    pb.set_message("Initializing models and camera");
                    let a = &mut App::new(base_path, &opts.user);
                    a.start_capture();
                    pb.set_message(
                    "Detecting face, please make sure you are in a well lit room, CTRL+C to exit",
                );
                    loop {
                        if let Some(encodings) = a.process_next_frame() {
                            if encodings.len() >= 1 {
                                a.push_model(
                                    encodings.first().unwrap().as_ref().to_vec(),
                                    x.label.clone(),
                                );
                                pb.set_message("Saving face encodings");
                                match a.save_model() {
                                    Ok(_) => {
                                        pb.finish_with_message(&format!(
                                            "Successfully added model for user {}",
                                            style(&opts.user).bold().blue()
                                        ));
                                        return;
                                    }
                                    Err(_) => {
                                        pb.finish_with_message(
                                            &style("Error saving the models")
                                                .bold()
                                                .red()
                                                .to_string(),
                                        );
                                    }
                                }
                            } else {
                                pb.finish_with_message(
                                    &style("Found more than one person").bold().red().to_string(),
                                );
                            }
                        }
                    }
                }

                // Clear all face models command
                ModelSubCommand::Clear(_) => {
                    pb.set_message("Initializing models");
                    let a = &mut App::new(base_path, &opts.user);
                    if a.models().is_empty() {
                        return pb.finish_with_message(&format!(
                            "No models found for user {}",
                            style(&opts.user).bold().blue()
                        ));
                    }
                    pb.set_message("Clearing");
                    a.clear_models();
                    pb.set_message("Saving face encodings");
                    match a.save_model() {
                        Ok(_) => {
                            pb.finish_with_message(&format!(
                                "Successfully cleared models for user: {}",
                                style(&opts.user).bold().blue()
                            ));
                            return;
                        }
                        Err(_) => {
                            pb.finish_with_message(
                                &style("Error saving the models").bold().red().to_string(),
                            );
                        }
                    }
                }

                // Remove a face model command
                ModelSubCommand::Remove(x) => {
                    pb.set_message("Initializing models");
                    let a = &mut App::new(base_path, &opts.user);
                    if a.models().is_empty() {
                        return pb.finish_with_message(&format!(
                            "No models found for user {}",
                            style(&opts.user).bold().blue()
                        ));
                    }
                    pb.set_message("Removing");
                    let index = match a.find_model(x.id) {
                        Some(idx) => idx,
                        None => {
                            return pb.finish_with_message(
                                &style("Invalid ID").bold().red().to_string(),
                            );
                        }
                    };
                    a.remove_model(index);
                    pb.set_message("Saving face encodings");
                    match a.save_model() {
                        Ok(_) => {
                            pb.finish_with_message(&format!(
                                "Successfully removed model for user {} with ID {}",
                                style(&opts.user).bold().blue(),
                                style(x.id).bold().green()
                            ));
                            return;
                        }
                        Err(_) => {
                            pb.finish_with_message(
                                &style("Error saving the models").bold().red().to_string(),
                            );
                        }
                    }
                }

                // List all face model command
                ModelSubCommand::List(_) => {
                    pb.set_message("Initializing models");
                    let a = &mut App::new(base_path, &opts.user);
                    pb.set_message(&format!(
                        "Fetching models for user {}",
                        style(&opts.user).bold().blue()
                    ));
                    if a.models().is_empty() {
                        return pb.finish_with_message(&format!(
                            "No models found for user {}",
                            style(&opts.user).bold().blue()
                        ));
                    }
                    pb.finish_and_clear();
                    println!("Models for user {}", style(&opts.user).bold().blue());
                    let mut table = Table::new();
                    table.add_row(row!["ID", "Label", "Added on"]);
                    for m in a.models().iter() {
                        table.add_row(row![
                            style(m.id).bold().dim().to_string(),
                            style(&m.label).bold().to_string(),
                            Local.timestamp(m.time, 0).to_string(),
                        ]);
                    }
                    table.printstd();
                }

                // Test against all face models command
                ModelSubCommand::Check(_) => {
                    pb.set_message("Initializing models and camera");
                    let a = &mut App::new(base_path, &opts.user);
                    if a.models().is_empty() {
                        return pb.finish_with_message(&format!(
                            "No models found for user {}",
                            style(&opts.user).bold().blue()
                        ));
                    }
                    a.start_capture();
                    pb.set_message(
                    "Detecting face, please make sure you are in a well lit room, CTRL+C to exit",
                );
                    let start_time = Instant::now();
                    loop {
                        if let Some(encodings) = a.process_next_frame() {
                            if encodings.iter().any(|e| a.identify(e.clone())) {
                                return pb.finish_with_message(&format!(
                                    "Identified face as {} in {:?}",
                                    style(&opts.user).bold().blue(),
                                    start_time.elapsed()
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Open config.toml with default editor
        SubCommand::Config(_) => {
            let editor = std::env::var("EDITOR").unwrap_or("/bin/nano".to_string());
            let config_file_path = base_path.join("config.toml");
            if let Err(err) = Exec::cmd(&editor).arg(config_file_path).join() {
                println!("Error opening config file: {:?}", err);
            }
        }
    }
}
