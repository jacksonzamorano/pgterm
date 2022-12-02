use std::{fs, path::{Path, PathBuf}, ffi::OsStr};

use iocontrol::IOControl;
use pg::{PostgresConnection, PostgresCredentials};

pub mod iocontrol;
pub mod pg;

fn start_control_loop(mut connection: PostgresConnection, mut console: IOControl) {
    let mut last_command = String::new();
    loop {
        let cmd_option = console.ask_for(">");
        if let Some(cmd) = cmd_option {
            if cmd == "quit" {
                connection.close();
                break;
            } else if cmd == "recall" {
                console.publish(&last_command);
            } else {
                last_command = cmd.clone();
                let words = cmd
                    .split(' ')
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>();
                let verb = &words[0];
                if verb == "get" {
                    if words.len() < 2 {
                        console.command_error("get", "get [TABLE_NAME]");
                        continue;
                    }
                    let query = connection.get_table(&words[1]);
                    if let Some(res) = query {
                        let table_values: Vec<Vec<String>> = res
                            .rows
                            .iter()
                            .map(|a| a.iter().map(|b| b.value()).collect())
                            .collect::<Vec<Vec<String>>>();
                        console.create_table(&res.columns, table_values, 14);
                    }
                } else if verb == "describe" {
                    if words.len() < 2 {
                        console.command_error("describe", "describe [TABLE_NAME]");
                        continue;
                    }
                    let query = connection.describe_table(&words[1]);
                    if let Some(res) = query {
                        let table_values: Vec<Vec<String>> = res
                            .iter()
                            .map(|a| {
                                return vec![
                                    a.name.clone(),
                                    a.data_type.clone(),
                                    if a.is_nullable {
                                        "Yes".to_string()
                                    } else {
                                        "No".to_string()
                                    },
                                ];
                            })
                            .collect();
                        console.create_table(
                            &[
                                "Name".to_string(),
                                "Type".to_string(),
                                "Nullable".to_string(),
                            ],
                            table_values,
                            14,
                        );
                    }
                } else if verb == "export" {
                    if words.len() < 3 {
                        console.command_error("export", "export [TABLE_NAME] [CSV_PATH]");
                        continue;
                    }
                    let path = &words[2];
                    let query = connection.get_table(&words[1]);
                    if let Some(res) = query {
                        let mut table_values: Vec<Vec<String>> = Vec::new();
                        table_values.push(res.columns);
                        table_values.extend(
                            res.rows
                                .iter()
                                .map(|a| a.iter().map(|b| b.value()).collect())
                                .collect::<Vec<Vec<String>>>(),
                        );

                        let output = table_values
                            .iter()
                            .map(|a| return a.join(","))
                            .collect::<Vec<String>>()
                            .join("\n");

                        match fs::write(path, output) {
                            Ok(_) => console.publish("File saved."),
                            Err(er) => console.publish_lines(&[
                                "Error saving".to_string(),
                                format!("{}", er)
                            ])
                        }
                    }
                } else if verb == "clear" {
                    console.clear();
                }
            }
        }
    }
}

fn main() {
    let mut console = IOControl::create();
    let mut credentials = PostgresCredentials::create_from_params(console.get_startup_parameters());
    console.complete_credentials(&mut credentials);
    console.announce(&[
        "Connecting...",
        &format!("Connecting to database {:?}", credentials.host),
        &format!("as user {:?}", credentials.username),
    ]);
    let mut connection = PostgresConnection::create(credentials);
    let connected = connection.connect();
    if connected {
        console.clear();
        start_control_loop(connection, console);
    } else {
        console.announce(&["Could not connect.", "Check credentials again."])
    }
}
