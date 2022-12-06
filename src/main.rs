use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
};

use iocontrol::IOControl;
use pg::{
    PostgresColumn, PostgresConnection, PostgresCredentials, PostgresRow, PostgresRowMatcher,
    PostgresTable,
};

pub mod iocontrol;
pub mod pg;

fn start_control_loop(mut connection: PostgresConnection, mut console: IOControl) {
    let mut last_command = String::new();
    'il: loop {
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
                            .map(|a| a.iter().map(|b| b.display()).collect())
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
                } else if verb == "csv" {
                    if words.len() < 3 {
                        console.command_error("csv", "csv [TABLE_NAME] [CSV_PATH]");
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
                                .map(|a| a.iter().map(|b| b.display()).collect())
                                .collect::<Vec<Vec<String>>>(),
                        );

                        let output = table_values
                            .iter()
                            .map(|a| return a.join(","))
                            .collect::<Vec<String>>()
                            .join("\n");

                        match fs::write(path, output) {
                            Ok(_) => console.publish("File saved."),
                            Err(er) => console
                                .publish_lines(&["Error saving".to_string(), format!("{}", er)]),
                        }
                    }
                } else if verb == "export" {
                    if words.len() < 2 {
                        console.command_error("export", "export [DESTINATION_PATH]");
                        continue;
                    }
                    let path = &words[1];

                    let mut output = String::new();

                    if let Some(tables) = connection.list_tables() {
                        'proc: for t in tables {
                            let table_data = connection.get_table(&t);
                            let table_description = connection.describe_table(&t);
                            output += "#table=";
                            output += &t;
                            output += "\n";
                            output += "+schema:\n";
                            if let Some(td) = table_description {
                                for c in td {
                                    output += "%|";
                                    output += &c.name;
                                    output += "|";
                                    output += &c.data_type;
                                    output += "|";
                                    output += if c.is_nullable { "y_null" } else { "n_null" };
                                    output += "\n";
                                }
                            } else {
                                console.publish("Could not retrieve table schema.");
                                break 'proc;
                            }
                            output += "-schema";
                            output += "\n+data:";
                            if let Some(td) = table_data {
                                for r in td.rows {
                                    output += "\n%";
                                    let i_max = r.len();
                                    let mut i_cur = 0;
                                    while i_cur < i_max {
                                        output += "|";
                                        output += &td.columns[i_cur];
                                        output += "=_=";
                                        output += &r[i_cur].display();
                                        i_cur += 1;
                                    }
                                }
                            } else {
                                console.publish("Could not get data in table.");
                                break 'proc;
                            }
                            output += "\n-data";
                            output += "\n\n\n";
                        }

                        match fs::write(path, output) {
                            Ok(_) => console.publish("Backup saved."),
                            Err(er) => console
                                .publish_lines(&["Error saving".to_string(), format!("{}", er)]),
                        }
                    } else {
                        console.publish("Could not get list of tables.");
                    }
                } else if verb == "import" {
                    if words.len() < 2 {
                        console.command_error("import", "import [IMPORT_PATH]");
                        continue;
                    }
                    let input_file_result = fs::File::open(&words[1]);
                    if let Ok(input_file) = input_file_result {
                        let mut reader = BufReader::new(input_file);
                        let mut line = String::new();

                        let mut tables: Vec<PostgresTable> = Vec::new();
                        let mut table = PostgresTable::new();

                        loop {
                            let size = reader.read_line(&mut line).unwrap_or(0);
                            if size > 0 {
                                if line.starts_with("#table") {
                                    // This line defines a new table.
                                    // If current table is worth anything, add it.
                                    if table.name != "" {
                                        tables.push(table);
                                    }
                                    // Reset the current table
                                    table = PostgresTable::new();
                                    // Get name and set it.
                                    let split_query = line.split("=").collect::<Vec<&str>>();
                                    if split_query.len() > 1 {
                                        table.name = split_query[1].to_string();
                                    } else {
                                        console.publish("Invalid format!");
                                        continue 'il;
                                    }
                                } else if line.starts_with("+schema") {
                                    let mut schema_line = String::new();

                                    loop {
                                        let size = reader.read_line(&mut schema_line).unwrap_or(0);
                                        if size == 0 {
                                            break;
                                        }
                                        if schema_line.contains("-schema") {
                                            // End of schema portion;
                                            break;
                                        } else if schema_line.starts_with("%") {
                                            let data_columns: Vec<String> = schema_line
                                                [2..schema_line.len()]
                                                .split("|")
                                                .collect::<Vec<&str>>()
                                                .iter()
                                                .map(|a| a.to_string())
                                                .collect::<Vec<String>>();
                                            table.columns.push(PostgresColumn {
                                                name: data_columns[0].clone(),
                                                data_type: data_columns[1].clone(),
                                                is_nullable: data_columns[2] == "y_null",
                                            });
                                        }
                                        schema_line = String::new();
                                    }
                                } else if line.starts_with("+data") {
                                    let mut data_line = String::new();
                                    loop {
                                        let size = reader.read_line(&mut data_line).unwrap_or(0);
                                        if size == 0 {
                                            break;
                                        }
                                        if data_line.contains("-data") {
                                            // End of schema portion;
                                            break;
                                        } else if data_line.starts_with("%") {
                                            let mut row_data: Vec<Box<dyn PostgresRow>> =
                                                Vec::new();
                                            let row: HashMap<String, String> = data_line
                                                [2..data_line.len()]
                                                .split("|")
                                                .collect::<Vec<&str>>()
                                                .iter()
                                                .map(|a| a.to_string())
                                                .collect::<Vec<String>>()
                                                .iter()
                                                .filter(|a| a.split("=_=").count() > 1)
                                                .map(|a| {
                                                    let split = a
                                                        .split("=_=")
                                                        .map(|a| a.to_string())
                                                        .collect::<Vec<String>>();
                                                    return (split[0].clone(), split[1].clone());
                                                })
                                                .collect();
                                            for col in &table.columns {
                                                if let Some(val) = row.get(&col.name) {
                                                    row_data.push(PostgresRowMatcher::match_type(
                                                        &col.data_type,
                                                        val,
                                                    ));
                                                }
                                            }
                                            table.data.push(row_data);
                                        }
                                        data_line = String::new();
                                    }
                                }
                                line = String::new();
                            } else {
                                break;
                            }
                        }

                        if let Some(confirmation) = console.ask_for(&format!(
                            "Import data with {} tables and {} rows? (y for yes)",
                            tables.len(),
                            tables.iter().map(|a| a.columns.len() as i32).sum::<i32>()
                        )) {
                            if confirmation.contains("y") {
                                continue 'il;
                            }
                            for t in tables {}
                        }
                    } else {
                        console.publish("Could not open specified file.");
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
