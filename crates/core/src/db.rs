// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
pub struct FrameDatabase {
    // Fields for the database connection and configuration
    connection: rusqlite::Connection,
}

pub struct User {
    name: String,
    age: u32,
}

impl FrameDatabase {
    pub fn new() -> Self {
        // Initialize the database connection and configuration
        let connection = rusqlite::Connection::open_in_memory().unwrap();

        let query = "
            CREATE TABLE users (name TEXT, age INTEGER);
            INSERT INTO users VALUES ('Alice', 42);
            INSERT INTO users VALUES ('Bob', 69);
        ";
        connection.execute_batch(query).unwrap();
        FrameDatabase {
            connection,
        }
    }

    pub fn query(&self, query: &str) -> Result<String, String> {
        // Execute the query and return the result or an error

        let query = "SELECT name FROM users";

        let mut statement = match self.connection.prepare(query) {
            Ok(stmt) => stmt,
            Err(e) => return Err(format!("Failed to prepare statement: {}", e)),
        };

        let user_iter = match statement.query_map([], |row| {
            Ok(User {
                name: row.get(0)?,
                age: 0, // Age is not selected in this query
            })
        }) {
            Ok(iter) => iter,
            Err(e) => return Err(format!("Failed to execute query: {}", e)),
        };

        for user in user_iter {
            match user {
                Ok(user) => println!("User: {}", user.name),
                Err(e) => return Err(format!("Failed to retrieve user: {}", e)),
            }
        }

        Ok("Query result".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query() {
        let db = FrameDatabase::new();
        match db.query("SELECT name FROM users") {
            Ok(result) => println!("Query successful: {}", result),
            Err(e) => println!("Query failed: {}", e),
        };

    }
}
