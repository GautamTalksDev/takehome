use netpay_conformance::{check_committed, report_from_repo, require_complete, write_report};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut write = false;
    let mut check = false;
    let mut require = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--write" => write = true,
            "--check" => check = true,
            "--require-complete" => require = true,
            "--help" | "-h" => {
                eprintln!("usage: netpay-conformance [--write] [--check] [--require-complete]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    let root = netpay_conformance::workspace_root();
    let report = match report_from_repo() {
        Ok(r) => r,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };
    if require {
        if let Err(err) = require_complete(&report) {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    }
    if write {
        if let Err(err) = write_report(&root, &report) {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
        eprintln!(
            "wrote CONFORMANCE.md and conformance.json ({:?})",
            report.status
        );
    }
    if check {
        if let Err(err) = check_committed(&root, &report) {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
        eprintln!("conformance report is current");
    }
    if !write && !check && !require {
        match netpay_conformance::render_json(&report) {
            Ok(json) => print!("{json}"),
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}
