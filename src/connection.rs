use std::env;

use spacetimedb_sdk::*;

use crate::module_bindings::*;

pub fn connect() -> DbConnection {
    // The URI of the SpacetimeDB instance hosting our chat module.
    let host: String = env::var("SPACETIMEDB_HOST").unwrap_or("http://localhost:3000".to_string());

    // The module name we chose when we published our module.
    let db_name: String =
        env::var("SPACETIMEDB_DB_NAME").unwrap_or("solarance-movement-prototype".to_string());

    // Connect to the database
    let conn = DbConnection::builder()
        .with_database_name(db_name)
        .with_uri(host)
        .on_connect(|ctx, _id, _| {
            println!("Connected to SpacetimeDB");

            ctx.subscription_builder().subscribe_to_all_tables();
        })
        .on_connect_error(|_ctx, e| {
            eprintln!("Connection error: {:?}", e);
            std::process::exit(1);
        })
        .build()
        .expect("Failed to connect");

    conn.run_threaded();

    conn
}
