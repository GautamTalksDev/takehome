use std::io::{self, Read};
use std::process::ExitCode;
use takehome_core::{
    calculate_wire, jurisdictions_json, rule_set_versions_json, ENGINE_BUILD_SHA256,
};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("jurisdictions") => {
            println!("{}", jurisdictions_json());
            ExitCode::SUCCESS
        }
        Some("versions") => {
            println!("{}", rule_set_versions_json());
            ExitCode::SUCCESS
        }
        Some("engine-sha") => {
            println!("{ENGINE_BUILD_SHA256}");
            ExitCode::SUCCESS
        }
        Some("calculate") => match calculate_stdin() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::from(1)
            }
        },
        Some("calculate-batch") => match calculate_batch_stdin() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::from(1)
            }
        },
        _ => {
            eprintln!(
                "usage: takehome jurisdictions | versions | engine-sha | calculate | calculate-batch"
            );
            ExitCode::from(2)
        }
    }
}

fn calculate_stdin() -> Result<(), String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    let body = calculate_wire(&buf);
    let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    if parsed.get("error").is_some() {
        return Err(body);
    }
    println!("{body}");
    Ok(())
}

fn calculate_batch_stdin() -> Result<(), String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    let requests: Vec<serde_json::Value> = serde_json::from_str(&buf).map_err(|e| e.to_string())?;
    let mut responses = Vec::with_capacity(requests.len());
    for value in requests {
        let request_json = serde_json::to_string(&value).map_err(|e| e.to_string())?;
        responses.push(calculate_wire(&request_json));
    }
    println!(
        "{}",
        serde_json::to_string(&responses).map_err(|e| e.to_string())?
    );
    Ok(())
}
