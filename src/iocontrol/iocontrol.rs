use crate::pg::PostgresCredentials;
use std::{
    collections::HashMap,
    env,
    io::{stdin, stdout, Stdin, Stdout, Write},
};

pub struct IOControl {
    startup_parameters: HashMap<String, String>,
    cin: Stdin,
    cout: Stdout,
}
impl IOControl {
    pub fn get_startup_parameters(&self) -> &HashMap<String, String> {
        return &self.startup_parameters;
    }

    pub fn create() -> IOControl {
        let mut parsed_args: HashMap<String, String> = HashMap::new();
        let input_args = env::args().collect::<Vec<String>>()[1..].to_vec();

        let input_count = input_args.len();
        let mut current_input_i = 1;
        while current_input_i < input_count {
            parsed_args.insert(
                input_args[current_input_i - 1].clone()[1..].to_string(),
                input_args[current_input_i].clone(),
            );
            current_input_i += 2;
        }

        IOControl {
            startup_parameters: parsed_args,
            cin: stdin(),
            cout: stdout(),
        }
    }

    pub fn complete_credentials(&mut self, credentials: &mut PostgresCredentials) {
        if credentials.host == "" {
            if let Some(r) = self.ask_for("Host:") {
                credentials.host = r;
            }
        }
        if credentials.username == "" {
            if let Some(r) = self.ask_for("Username:") {
                credentials.username = r;
            }
        }
        if credentials.password == "" && credentials.pass_required {
            if let Some(r) = self.ask_for("Password:") {
                credentials.password = r;
            }
        }
        if credentials.database == "" {
            if let Some(r) = self.ask_for("Database:") {
                credentials.database = r;
            }
        }
    }

    pub fn clear(&mut self) {
        _ = self.cout.write_all("\x1B[2J\x1B[1;1H".as_bytes());
    }

    pub fn ask_for(&mut self, title: &str) -> Option<String> {
        let mut handle = self.cout.lock();
        handle.write_all(title.as_bytes()).ok()?;
        handle.write_all(" ".as_bytes()).ok()?;
        handle.flush().ok()?;
        let mut read = String::new();
        self.cin.read_line(&mut read).ok()?;
        return Some(read.trim().to_string());
    }

    pub fn create_table(&mut self, header: &[String], values: Vec<Vec<String>>, col_size: usize) {
        let mut lines: Vec<String> = Vec::new();
        let mut header_line = "| ".to_string();
        for v in header {
            header_line += &self.pad_value(v.clone(), col_size.clone());
            header_line += " | ";
        }
        lines.push(header_line[0..header_line.chars().count() - 1].to_string());
        for l in values {
            let mut line = "| ".to_string();
            for v in l {
                let input_val = v.to_owned();
                line += &self.pad_value(input_val, col_size.clone());
                line += " | ";
            }
            lines.push(line[0..line.chars().count() - 1].to_string());
        }
        self.publish_lines(&lines);
    }

    pub fn publish_lines(&mut self, lines: &[String]) {
        let mut handle = self.cout.lock();
        for l in lines {
            _ = handle.write_all(l.as_bytes());
            _ = handle.write(&[b'\n']);
        }
    }
    pub fn publish(&mut self, l: &str) {
        let mut handle = self.cout.lock();
        _ = handle.write_all(l.as_bytes());
        _ = handle.write(&[b'\n']);
    }

    pub fn announce(&mut self, titles: &[&str]) {
        self.clear();
        if let Some((w, h)) = term_size::dimensions() {
            let top_pad = h / 2 - titles.len() - 2;
            let bot_pad = h / 2;

            let mut top_pad_string = String::new();
            let mut bot_pad_string = String::new();

            let mut t_i = 0;
            let mut b_i = 0;
            while t_i < top_pad {
                top_pad_string.push_str(&"\n");
                t_i += 1;
            }
            while b_i < bot_pad {
                bot_pad_string.push_str(&"\n");
                b_i += 1;
            }

            let mut handle = self.cout.lock();
            _ = handle.write_all(top_pad_string.as_bytes());
            for t in titles {
                let left_pad = (w - t.len()) / 2;
                let mut l_i = 0;
                let mut left_pad_string = String::new();
                while l_i < left_pad {
                    left_pad_string.push_str(&" ");
                    l_i += 1;
                }
                _ = handle.write_all(left_pad_string.as_bytes());
                _ = handle.write_all(t.as_bytes());
                _ = handle.write_all("\n".as_bytes());
            }
            _ = handle.write_all(bot_pad_string.as_bytes());
            _ = handle.flush();
        }
    }

    pub fn width(&self) -> Option<usize> {
        if let Some((w, _)) = term_size::dimensions() {
            Some(w)
        } else {
            None
        }
    }

    pub fn start_loading(&self) {
        // self.clear();
    }

    pub fn command_error(&mut self, command: &str, usage: &str) {
        let mut output = String::new();
        output += command;
        output += ": ";
        output += usage;
        self.publish_lines(&[
            "Invalid usage!".to_string(),
            output
        ]);
    }

    fn pad_value(&self, mut input: String, mut fixed_size: usize) -> String {
    	let mut line = String::new();
        let mut val_size = input.chars().count();
        if val_size > fixed_size {
            input = input[0..fixed_size - 5].to_string();
            input += "(...)";
            val_size = fixed_size;
        }
        let pad_len = fixed_size - val_size;
        let mut pad = String::new();
        let mut pad_i = 0;
        while pad_i < pad_len / 2 {
            pad += " ";
            pad_i += 1;
        }
        line += &pad;
        // Add extra space when number is odd
        // due to integer divison
        if (pad_len / 2) * 2 < pad_len {
            line += " ";
        }
        line += &input;
        line += &pad;
        line
    }
}
