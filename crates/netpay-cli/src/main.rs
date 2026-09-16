use netpay_core::rules::loader::EMBEDDED_REGISTRY;
use netpay_core::{calculate, Request};
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("jurisdictions") => {
            println!("{}", netpay_core::jurisdictions_json());
            ExitCode::SUCCESS
        }
        Some("calculate") => match calculate_stdin() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::from(1)
            }
        },
        _ => {
            eprintln!("usage: netpay jurisdictions | netpay calculate < request.json");
            ExitCode::from(2)
        }
    }
}

fn calculate_stdin() -> Result<(), String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    let req = Request::from_json(&buf).map_err(|e| e.to_string())?;
    let resp = calculate(&req, &EMBEDDED_REGISTRY).map_err(|e| e.to_string())?;
    let employee = serde_json::json!({
        "federal_tax": resp.employee.federal_tax.to_string(),
        "provincial_tax": resp.employee.provincial_tax.to_string(),
        "total_tax": resp.employee.total_tax.to_string(),
        "cpp": resp.employee.cpp.to_string(),
        "cpp2": resp.employee.cpp2.to_string(),
        "ei": resp.employee.ei.to_string(),
        "total_deductions": resp.employee.total_deductions.to_string(),
        "net_pay": resp.employee.net_pay.to_string(),
        "rule_set_version": resp.rule_set_version,
    });
    println!("{}", serde_json::to_string(&employee).map_err(|e| e.to_string())?);
    Ok(())
}
