use netpay_grid_gen::{
    canonical_bytes, generate, july_boundary_queue, sampling_report, stratified_smoke_queue,
    GridError,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut out_dir: Option<PathBuf> = None;
    let mut manifest_only = false;
    let mut report = false;
    let mut emit_queue = false;
    let mut smoke: Option<usize> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                out_dir = args.next().map(PathBuf::from);
                if out_dir.is_none() {
                    eprintln!(
                        "usage: netpay-grid-gen [--out DIR] [--manifest] [--report] [--queue] [--smoke N]"
                    );
                    return ExitCode::from(2);
                }
            }
            "--manifest" => manifest_only = true,
            "--report" => report = true,
            "--queue" => emit_queue = true,
            "--smoke" => {
                let Some(raw) = args.next() else {
                    eprintln!("--smoke requires N");
                    return ExitCode::from(2);
                };
                match raw.parse::<usize>() {
                    Ok(n) => smoke = Some(n),
                    Err(_) => {
                        eprintln!("--smoke N must be an integer");
                        return ExitCode::from(2);
                    }
                }
            }
            "--help" | "-h" => {
                eprintln!(
                    "usage: netpay-grid-gen [--out DIR] [--manifest] [--report] [--queue] [--smoke N]"
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    match run(out_dir.as_deref(), manifest_only, report, emit_queue, smoke) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn run(
    out_dir: Option<&std::path::Path>,
    manifest_only: bool,
    report: bool,
    emit_queue: bool,
    smoke: Option<usize>,
) -> Result<(), GridError> {
    let grid = generate()?;
    if emit_queue {
        let queue = july_boundary_queue(&grid);
        let json = serde_json::to_string_pretty(&queue)
            .map_err(|e| GridError::Message(e.to_string()))?;
        if let Some(dir) = out_dir {
            fs::create_dir_all(dir).map_err(|e| GridError::Message(e.to_string()))?;
            fs::write(dir.join("pdoc-queue.json"), format!("{json}\n"))
                .map_err(|e| GridError::Message(e.to_string()))?;
            eprintln!(
                "PDOC queue {} distinct capturable July forms → {}",
                queue.len(),
                dir.display()
            );
        } else {
            println!("{json}");
        }
        return Ok(());
    }
    if let Some(n) = smoke {
        let queue = stratified_smoke_queue(&grid, n);
        let json = serde_json::to_string_pretty(&queue)
            .map_err(|e| GridError::Message(e.to_string()))?;
        if let Some(dir) = out_dir {
            fs::create_dir_all(dir).map_err(|e| GridError::Message(e.to_string()))?;
            fs::write(dir.join("smoke-queue.json"), format!("{json}\n"))
                .map_err(|e| GridError::Message(e.to_string()))?;
            eprintln!("smoke queue {} forms → {}", queue.len(), dir.display());
        } else {
            println!("{json}");
        }
        return Ok(());
    }
    if report {
        let sampling = sampling_report(&grid, 0);
        println!(
            "{}",
            serde_json::to_string_pretty(&sampling)
                .map_err(|e| GridError::Message(e.to_string()))?
        );
        return Ok(());
    }
    let manifest = serde_json::to_string_pretty(&grid.manifest)
        .map_err(|e| GridError::Message(e.to_string()))?;
    if manifest_only {
        println!("{manifest}");
        return Ok(());
    }
    if let Some(dir) = out_dir {
        fs::create_dir_all(dir).map_err(|e| GridError::Message(e.to_string()))?;
        fs::write(dir.join("manifest.json"), format!("{manifest}\n"))
            .map_err(|e| GridError::Message(e.to_string()))?;
        let bytes = canonical_bytes(&grid)?;
        fs::write(dir.join("grid.canonical"), &bytes)
            .map_err(|e| GridError::Message(e.to_string()))?;
        let mut jsonl = String::new();
        for case in &grid.cases {
            jsonl.push_str(
                &serde_json::to_string(case).map_err(|e| GridError::Message(e.to_string()))?,
            );
            jsonl.push('\n');
        }
        fs::write(dir.join("cases.jsonl"), jsonl).map_err(|e| GridError::Message(e.to_string()))?;
        eprintln!(
            "grid {}  cases {}  sha {}",
            grid.manifest.grid_version, grid.manifest.case_count, grid.manifest.git_sha
        );
        return Ok(());
    }
    println!("{manifest}");
    Ok(())
}
