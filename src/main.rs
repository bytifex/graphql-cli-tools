mod cli;

use clap::Parser;
use cli::Cli;
use graphql_cli_tools::{
    client::{execute, load_variables},
    schema_diff::diff_schema,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli {
        Cli::Client(params) => {
            let variables = load_variables(params.variables_from_json, params.variables)?;
            let headers = params.headers.into_iter().collect();

            execute(
                params.server_endpoint,
                headers,
                params.query_path,
                params.operation_name,
                variables,
                |response| {
                    println!("{}", serde_json::to_string_pretty(&response)?);

                    Ok(())
                },
                params
                    .try_reconnect_duration
                    .map(|duration| duration.into()),
            )
            .await
        }
        Cli::DiffSchema(params) => {
            diff_schema(params.schema_source_left, params.schema_source_right)
        }
    }
}
