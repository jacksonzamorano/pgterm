use std::collections::HashMap;

use postgres::{
    types::{private::BytesMut, FromSql, IsNull, ToSql, Type},
    Client, NoTls,
};

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
            "host = {} user = {} {}{} dbname = {}",
            self.credentials.host,
            self.credentials.username,
            if self.credentials.pass_required {
                "password = "
            } else {
                ""
            },
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
                                let matched: Box<dyn PostgresRow> =
                                    match row.try_get::<&str, bool>(c.0.as_str()) {
                                        Ok(val) => Box::new(PostgresBoolRow { value: val }),
                                        Err(_) => Box::new(PostgresNullRow {}),
                                    };
                                data.push(matched)
                            } else if c.1 == Type::INT2 || c.1 == Type::INT4 || c.1 == Type::INT8 {
                                let matched: Box<dyn PostgresRow> =
                                    match row.try_get::<&str, i32>(c.0.as_str()) {
                                        Ok(val) => Box::new(PostgresI32Row { value: val }),
                                        Err(_) => Box::new(PostgresNullRow {}),
                                    };
                                data.push(matched)
                            } else if c.1 == Type::TEXT {
                                let matched: Box<dyn PostgresRow> =
                                    match row.try_get::<&str, String>(c.0.as_str()) {
                                        Ok(val) => Box::new(PostgresStringRow { value: val }),
                                        Err(_) => Box::new(PostgresNullRow {}),
                                    };
                                data.push(matched)
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

    pub fn describe_table(&mut self, table: &String) -> Option<Vec<PostgresColumn>> {
        if let Some(client) = &mut self.client {
            let mut values: Vec<PostgresColumn> = Vec::new();
            let query = client.query(&format!("select column_name, data_type, is_nullable from information_schema.columns where table_name='{}';", table), &[]);
            match query {
                Ok(rows) => {
                    for row in rows {
                        let t: &str = row.get("is_nullable");
                        values.push(PostgresColumn {
                            name: row.get("column_name"),
                            data_type: row.get("data_type"),
                            is_nullable: if t == "NO" { false } else { true },
                        });
                    }
                }
                Err(er) => println!("{:?}", er),
            }
            Some(values)
        } else {
            None
        }
    }

    pub fn list_tables(&mut self) -> Option<Vec<String>> {
        if let Some(client) = &mut self.client {
            let tables = client
                .query("select table_name from information_schema.tables where table_schema != 'pg_catalog' AND table_schema != 'information_schema'", &[])
                .ok()?;
            Some(
                tables
                    .iter()
                    .map(|a| a.get("table_name"))
                    .filter(|a: &String| !a.starts_with(&"pg_"))
                    .collect(),
            )
        } else {
            None
        }
    }
}

pub struct PostgresTable {
    pub columns: Vec<PostgresColumn>,
    pub data: Vec<Vec<Box<dyn PostgresRow>>>,
    pub name: String,
}
impl PostgresTable {
    pub fn new() -> PostgresTable {
        PostgresTable {
            columns: Vec::new(),
            data: Vec::new(),
            name: String::new(),
        }
    }
}

pub struct PostgresResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Box<dyn PostgresRow>>>,
}

pub struct PostgresColumn {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
}

pub trait PostgresRow {
    fn display(&self) -> String;
    fn value(&self) -> Option<Box<dyn ToSql>>;
}

pub struct PostgresRowMatcher {}
impl PostgresRowMatcher {
    pub fn match_type(t: &String, data: &String) -> Box<dyn PostgresRow> {
        if t == &"boolean".to_string() {
            Box::new(PostgresBoolRow { value: data == "true" })
        } else if t == &"text".to_string() {
            Box::new(PostgresStringRow { value: data.clone() })
        } else if t == &"number".to_string() {
            Box::new(PostgresI32Row { value: data.parse().unwrap_or(0) })
        } else {
            Box::new(PostgresNullRow {})
        }
    }
}

pub struct PostgresStringRow {
    value: String,
}
impl PostgresRow for PostgresStringRow {
    fn display(&self) -> String {
        return self.value.clone();
    }
    fn value(&self) -> Option<Box<dyn ToSql>> {
        return Some(Box::new(self.value.clone()));
    }
}
pub struct PostgresI32Row {
    value: i32,
}
impl PostgresRow for PostgresI32Row {
    fn display(&self) -> String {
        return self.value.to_string();
    }
    fn value(&self) -> Option<Box<dyn ToSql>> {
        return Some(Box::new(self.value));
    }
}
pub struct PostgresBoolRow {
    value: bool,
}
impl PostgresRow for PostgresBoolRow {
    fn display(&self) -> String {
        return if self.value {
            "true".to_string()
        } else {
            "false".to_string()
        };
    }
    fn value(&self) -> Option<Box<dyn ToSql>> {
        return Some(Box::new(self.value));
    }
}
pub struct PostgresNullRow {}
impl PostgresRow for PostgresNullRow {
    fn display(&self) -> String {
        return String::new();
    }

    fn value(&self) -> Option<Box<dyn ToSql>> {
        return None;
    }
}

#[derive(Debug)]
pub struct PostgresCredentials {
    pub host: String,
    pub username: String,
    pub password: String,
    pub database: String,
    pub pass_required: bool,
}

impl PostgresCredentials {
    const HOST_KEY: &str = "url";
    const USERNAME_KEY: &str = "user";
    const PASSWORD_KEY: &str = "pass";
    const DATABASE_KEY: &str = "db";
    const NOPASS_KEY: &str = "np";

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
        let nopass = params
            .get(&PostgresCredentials::NOPASS_KEY.to_string())
            .unwrap_or(&String::new())
            .to_string();
        PostgresCredentials {
            host,
            username,
            password,
            database,
            pass_required: nopass != "y",
        }
    }
}
