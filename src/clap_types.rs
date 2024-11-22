use std::str::FromStr;

use clap::{builder::TypedValueParser, error::ErrorKind, Arg, Command, Error};
use http::{HeaderName, HeaderValue};

#[derive(Debug, Clone)]
pub struct ClapKeyJsonValueParser;

impl TypedValueParser for ClapKeyJsonValueParser {
    type Value = (String, serde_json::Value);

    fn parse_ref(
        &self,
        cmd: &Command,
        _arg: Option<&Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, Error> {
        let value = value.to_string_lossy();

        let (variable_name, variable_value): (&str, serde_json::Value) = if let Some(equals_pos) =
            value.find("=")
        {
            let (variable_name, variable_value) = value.split_at(equals_pos);

            let variable_value = &variable_value[1..variable_value.len()];
            let variable_value = if variable_value.is_empty() {
                serde_json::Value::Null
            } else if variable_value == "true" {
                serde_json::Value::Bool(true)
            } else if variable_value == "false" {
                serde_json::Value::Bool(false)
            } else if variable_value.starts_with("\"") && variable_value.ends_with("\"") {
                serde_json::Value::String(variable_value[1..variable_value.len() - 1].to_string())
            } else if let Ok(value) = variable_value.parse::<i128>() {
                serde_json::Value::Number(serde_json::Number::from_i128(value).ok_or_else(
                    || {
                        cmd.clone().error(
                            ErrorKind::InvalidValue,
                            "cannot convert from i128 to serde_json::Number",
                        )
                    },
                )?)
            } else if let Ok(value) = variable_value.parse::<u128>() {
                serde_json::Value::Number(serde_json::Number::from_u128(value).ok_or_else(
                    || {
                        cmd.clone().error(
                            ErrorKind::InvalidValue,
                            "cannot convert from u128 to serde_json::Number",
                        )
                    },
                )?)
            } else if let Ok(value) = variable_value.parse::<f64>() {
                serde_json::Value::Number(serde_json::Number::from_f64(value).ok_or_else(|| {
                    cmd.clone().error(
                        ErrorKind::InvalidValue,
                        "cannot convert from f64 to serde_json::Number",
                    )
                })?)
            } else if (variable_value.starts_with("[") && variable_value.ends_with("]"))
                || (variable_value.starts_with("{") && variable_value.ends_with("}"))
            {
                serde_json::from_str(variable_value)
                    .map_err(|e| cmd.clone().error(ErrorKind::InvalidValue, e.to_string()))?
            } else {
                serde_json::Value::String(variable_value.to_string())
            };

            (variable_name, variable_value)
        } else {
            (value.as_ref(), serde_json::Value::Null)
        };

        Ok((variable_name.into(), variable_value))
    }
}

#[derive(Debug, Clone)]
pub struct ClapHttpHeaderParser;

impl TypedValueParser for ClapHttpHeaderParser {
    type Value = (HeaderName, HeaderValue);

    fn parse_ref(
        &self,
        cmd: &Command,
        _arg: Option<&Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, Error> {
        let value = value.to_string_lossy();

        let (header_name, header_value) = if let Some(equals_pos) = value.find("=") {
            let (header_name, header_value) = value.split_at(equals_pos);

            (header_name, &header_value[1..header_value.len()])
        } else {
            (value.as_ref(), "")
        };

        Ok((
            HeaderName::from_str(header_name)
                .map_err(|e| cmd.clone().error(ErrorKind::ValueValidation, e))?,
            HeaderValue::from_str(header_value)
                .map_err(|e| cmd.clone().error(ErrorKind::ValueValidation, e))?,
        ))
    }
}
