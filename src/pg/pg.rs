use std::collections::HashMap;

use postgres::{types::Type, Client, NoTls};

pub struct PostgresConnection {
    credentials: PostgresCredentials,
    client: Option<Client>,
}
impl PostgresConnection {
    pub fn create(credentials: PostgresCredentials) -> PostgresConnection {
        PostgresConnection {
            credentials,
            client: None,
        }
    }

    pub fn connect(&mut self) -> bool {
        let connection_string = format!(
            "host = {} user = {} password = {} dbname = {}",
            self.credentials.host,
            self.credentials.username,
            self.credentials.password,
            self.credentials.database
        );
        self.client = match Client::connect(&connection_string, NoTls) {
            Ok(cn) => Some(cn),
            Err(_) => None,
        };
        return self.client.is_some();
    }

    pub fn close(self) {
        if let Some(c) = self.client {
            _ = c.close();
        }
    }

    pub fn get_table(&mut self, table: &String) -> Option<PostgresResult> {
        if let Some(client) = &mut self.client {
            let mut column_names: Vec<(String, Type)> = Vec::new();
            let mut values: Vec<Vec<Box<dyn PostgresRow>>> = Vec::new();
            let query = client.query(&format!("select * from {}", table), &[]);
            match query {
                Ok(rows) => {
                    for row in rows {
                        if column_names.len() == 0 {
                            column_names = row
                                .columns()
                                .iter()
                                .map(|a| (a.name().to_string(), a.type_().clone()))
                                .collect();
                        }
                        let mut data: Vec<Box<dyn PostgresRow>> = Vec::new();
                        for c in &column_names {
                            if c.1 == Type::BOOL {
                                let value: bool = row.get(c.0.as_str());
                                data.push(Box::new(PostgresBoolRow { value }))
                            } else if c.1 == Type::INT2 || c.1 == Type::INT4 || c.1 == Type::INT8 {
                                let value: i32 = row.get(c.0.as_str());
                                data.push(Box::new(PostgresI32Row { value }))
                            } else if c.1 == Type::TEXT {
                                let value: String = row.get(c.0.as_str());
                                data.push(Box::new(PostgresStringRow { value }))
                            }
                        }
                        values.push(data);
                    }
                }
                Err(er) => println!("{:?}", er),
            }
            Some(PostgresResult {
                columns: column_names.iter().map(|a| a.0.to_string()).collect(),
                rows: values,
            })
        } else {
            None
        }
    }
}

pub struct PostgresResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Box<dyn PostgresRow>>>,
}

pub trait PostgresRow {
    fn value(&self) -> String;
}

pub struct PostgresStringRow {
    value: String,
}
impl PostgresRow for PostgresStringRow {
    fn value(&self) -> String {
        return self.value.clone();
    }
}
pub struct PostgresI32Row {
    value: i32,
}
impl PostgresRow for PostgresI32Row {
    fn value(&self) -> String {
        return self.value.to_string();
    }
}
pub struct PostgresBoolRow {
    value: bool,
}
impl PostgresRow for PostgresBoolRow {
    fn value(&self) -> String {
        return if self.value {
            "true".to_string()
        } else {
            "false".to_string()
        };
    }
}

pub struct PostgresCredentials {
    pub host: String,
    pub username: String,
    pub password: String,
    pub database: String,
}
impl PostgresCredentials {
    const HOST_KEY: &str = "url";
    const USERNAME_KEY: &str = "user";
    const PASSWORD_KEY: &str = "pass";
    const DATABASE_KEY: &str = "db";

    pub fn create_from_params(params: &HashMap<String, String>) -> PostgresCredentials {
        let host = params
            .get(&PostgresCredentials::HOST_KEY.to_string())
            .unwrap_or(&String::new())
            .to_string();
        let username = params
            .get(&PostgresCredentials::USERNAME_KEY.to_string())
            .unwrap_or(&String::new())
            .to_string();
        let password = params
            .get(&PostgresCredentials::PASSWORD_KEY.to_string())
            .unwrap_or(&String::new())
            .to_string();
        let database = params
            .get(&PostgresCredentials::DATABASE_KEY.to_string())
            .unwrap_or(&String::new())
            .to_string();
        PostgresCredentials {
            host,
            username,
            password,
            database,
        }
    }
}
