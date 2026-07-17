use std::path::PathBuf;
use std::process::ExitCode;

use aqc_requirement_architecture::check_repository_roots;

mod output;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(core_manifest) = arguments.next().map(PathBuf::from) else {
        let _ = output::stderr(
            "usage: aqc-requirement-architecture <core-manifest> <repository-root>...\n",
        );
        return ExitCode::from(2);
    };
    let roots = arguments.map(PathBuf::from).collect::<Vec<_>>();
    if roots.is_empty() {
        let _ = output::stderr(
            "usage: aqc-requirement-architecture <core-manifest> <repository-root>...\n",
        );
        return ExitCode::from(2);
    }
    match check_repository_roots(&core_manifest, &roots) {
        Ok(report) => {
            match serde_json::to_string_pretty(&report) {
                Ok(serialized) => {
                    if output::stdout(&format!("{serialized}\n")).is_err() {
                        return ExitCode::from(2);
                    }
                }
                Err(error) => {
                    let _ = output::stderr(&format!(
                        "failed to serialize architecture report: {error}\n"
                    ));
                    return ExitCode::from(2);
                }
            }
            if report.violations.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            let _ = output::stderr(&format!("architecture check failed: {error}\n"));
            ExitCode::from(2)
        }
    }
}
