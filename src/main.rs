use iocontrol::IOControl;
use pg::{PostgresConnection, PostgresCredentials};

pub mod iocontrol;
pub mod pg;

fn start_control_loop(mut connection: PostgresConnection, mut console: IOControl) {
    loop {
        let cmd_option = console.ask_for(">");
        if let Some(cmd) = cmd_option {
            if cmd == "quit" {
                connection.close();
                break;
            } else {
                let words = cmd.split(' ').map(|a| a.to_string()).collect::<Vec<String>>();
                let verb = &words[0];
                if verb == "get" {
                    // dbg!(&words[1]);
                    let fixed_size = 14;
                    let query = connection.get_table(&words[1]);
                    if let Some(res) = query {
                        let mut lines: Vec<String> = Vec::new();

                        let mut header_line = "| ".to_string();
                        for c in res.columns {
                            header_line += &console.pad_value(c, fixed_size);
                            header_line += " | ";
                        }
                        lines.push(header_line);
                        for r in &res.rows {
                            let mut line = "| ".to_string();
                            for d in r {
                                line += &console.pad_value(d.value(), fixed_size);
                                line += " | ";
                            }
                            lines.push(line);
                        }

                        console.publish_lines(&lines);
                    }
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
        console.announce(&[
            "Could not connect.",
            "Check credentials again."
        ])
    }
}
