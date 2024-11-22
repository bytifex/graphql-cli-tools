use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

use error::{InvalidServerEndpointScheme, WsConnectionInitError};
use futures_util::{SinkExt, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
use uuid::Uuid;

pub async fn execute(
    server_endpoint: impl AsRef<str>,
    headers: HeaderMap,
    query_path: impl AsRef<Path>,
    operation_name: Option<impl AsRef<str>>,
    variables: serde_json::Map<String, serde_json::Value>,
    response_processor: impl FnMut(GraphQlResponse) -> Result<(), Box<dyn std::error::Error>>,
    try_reconnect_duration: Option<std::time::Duration>,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = load_query(query_path)?;

    if server_endpoint.as_ref().starts_with("http://")
        || server_endpoint.as_ref().starts_with("https://")
    {
        http_request(
            server_endpoint,
            headers,
            query,
            operation_name,
            variables,
            response_processor,
            try_reconnect_duration,
        )
        .await
    } else if server_endpoint.as_ref().starts_with("ws://")
        || server_endpoint.as_ref().starts_with("wss://")
    {
        ws_request(
            server_endpoint,
            headers,
            query,
            operation_name,
            variables,
            response_processor,
            try_reconnect_duration,
        )
        .await
    } else {
        Err(InvalidServerEndpointScheme.into())
    }
}

pub fn load_query(query_path: impl AsRef<Path>) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(query_path.as_ref())?;
    let mut query = String::new();
    file.read_to_string(&mut query)?;

    Ok(query)
}

pub fn load_variables(
    variables_from_json: Option<PathBuf>,
    variables_list: Vec<(String, serde_json::Value)>,
) -> Result<serde_json::Map<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let mut variables = if let Some(json_path) = variables_from_json {
        let mut file = std::fs::File::open(json_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        serde_json::from_str(&contents)?
    } else {
        serde_json::Map::default()
    };

    variables.extend(
        variables_list
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );

    Ok(variables)
}

pub async fn try_http_request(
    server_endpoint: impl AsRef<str>,
    headers: HeaderMap,
    query: String,
    operation_name: Option<impl AsRef<str>>,
    variables: serde_json::Map<String, serde_json::Value>,
    response_processor: &mut impl FnMut(GraphQlResponse) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::ClientBuilder::new().build()?;

    let response = client
        .post(server_endpoint.as_ref())
        .headers(headers)
        .json(&json!({
            "operationName": operation_name.as_ref().map(|s| s.as_ref()),
            "query": query,
            "variables": variables,
        }))
        .send()
        .await?;

    let response = response.json::<GraphQlResponse>().await?;

    response_processor(response)?;

    Ok(())
}

pub async fn http_request(
    server_endpoint: impl AsRef<str>,
    headers: HeaderMap,
    query: String,
    operation_name: Option<impl AsRef<str>>,
    variables: serde_json::Map<String, serde_json::Value>,
    mut response_processor: impl FnMut(GraphQlResponse) -> Result<(), Box<dyn std::error::Error>>,
    try_reconnect_duration: Option<std::time::Duration>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if let Err(e) = try_http_request(
            server_endpoint.as_ref(),
            headers.clone(),
            query.clone(),
            operation_name.as_ref().map(|s| s.as_ref()),
            variables.clone(),
            &mut response_processor,
        )
        .await
        {
            println!("{:?}", e);
        }

        if let Some(duration) = try_reconnect_duration {
            tokio::time::sleep(duration).await;
        } else {
            break Ok(());
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GraphQlResponse {
    pub data: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<serde_json::Map<String, serde_json::Value>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WsResponse {
    #[allow(unused)]
    r#type: String,
    #[allow(unused)]
    id: String,
    payload: Option<GraphQlResponse>,
}

async fn try_ws_request(
    server_endpoint: impl AsRef<str>,
    headers: HeaderMap,
    query: String,
    operation_name: Option<impl AsRef<str>>,
    variables: serde_json::Map<String, serde_json::Value>,
    response_processor: &mut impl FnMut(GraphQlResponse) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut request = server_endpoint.as_ref().into_client_request()?;

    request.headers_mut().extend(headers);
    request.headers_mut().insert(
        "sec-websocket-protocol",
        HeaderValue::from_str("graphql-transport-ws")?,
    );

    request.headers_mut().insert(
        "sec-websocket-extensions",
        HeaderValue::from_str("permessage-deflate; client_max_window_bits")?,
    );

    request.extensions_mut().insert("permessage-deflate");
    request.extensions_mut().insert("client_max_window_bits");

    let (mut ws_stream, mut _server_response) = tokio_tungstenite::connect_async(request).await?;

    println!("{:?}", _server_response);

    ws_stream
        .send(Message::text(serde_json::to_string(&json!({
            "type": "connection_init",
            "payload": {}
        }))?))
        .await?;

    ws_stream.next().await.ok_or(WsConnectionInitError)??;

    ws_stream
        .send(Message::text(serde_json::to_string(&json!({
            "id": Uuid::new_v4().to_string(),
            "type": "subscribe",
            "payload": {
                "operationName": operation_name.as_ref().map(|s| s.as_ref()),
                "query": query,
                "variables": variables,
            }
        }))?))
        .await?;

    while let Some(message) = ws_stream.next().await {
        match message {
            Ok(message) => {
                if let Ok(message) = message.into_text() {
                    let response = serde_json::from_str::<WsResponse>(&message)?;

                    if let Some(payload) = response.payload {
                        response_processor(payload)?;
                    } else {
                        println!("{}", serde_json::to_string_pretty(&response)?);
                        if response.r#type == "complete" {
                            break;
                        }
                    }
                } else {
                    println!("Invalid message received from websocket");
                }
            }
            Err(e) => {
                println!("{e}");
            }
        }
    }

    Ok(())
}

pub async fn ws_request(
    server_endpoint: impl AsRef<str>,
    headers: HeaderMap,
    query: String,
    operation_name: Option<impl AsRef<str>>,
    variables: serde_json::Map<String, serde_json::Value>,
    mut response_processor: impl FnMut(GraphQlResponse) -> Result<(), Box<dyn std::error::Error>>,
    try_reconnect_duration: Option<std::time::Duration>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if let Err(e) = try_ws_request(
            server_endpoint.as_ref(),
            headers.clone(),
            query.clone(),
            operation_name.as_ref().map(|s| s.as_ref()),
            variables.clone(),
            &mut response_processor,
        )
        .await
        {
            println!("{:?}", e);
        }

        if let Some(duration) = try_reconnect_duration {
            tokio::time::sleep(duration).await;
        } else {
            break Ok(());
        }
    }
}

pub mod error {
    #[derive(Debug, thiserror::Error)]
    #[error("WsConnectionInitError")]
    pub struct WsConnectionInitError;

    #[derive(Debug, thiserror::Error)]
    #[error("InvalidServerEndpointScheme")]
    pub struct InvalidServerEndpointScheme;
}
