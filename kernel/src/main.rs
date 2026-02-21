use clap::{Arg, ArgAction, Command};
use cog_kernel::test_harness::TestHarness;

fn main() {
    let cli = Command::new("cog-kernel")
        .arg_required_else_help(true)
        .subcommand(Command::new("simulate").about("Run COA simulator"))
        .subcommand(
            Command::new("stress")
                .about("Run stress test")
                .arg(Arg::new("nodes").long("nodes").default_value("10000"))
                .arg(Arg::new("iterations").long("iterations").default_value("5000")),
        )
        .subcommand(
            Command::new("validate-log")
                .about("Verify log integrity")
                .arg(Arg::new("path").long("path")),
        )
        .subcommand(
            Command::new("report")
                .about("Generate integrity report")
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
        );

    let matches = cli.get_matches();

    match matches.subcommand() {
        Some(("simulate", _)) => {
            println!("Running simulator...");
        }
        Some(("stress", args)) => {
            let nodes: usize = args.get_one::<String>("nodes").unwrap().parse().unwrap();
            let iterations: usize = args.get_one::<String>("iterations").unwrap().parse().unwrap();
            TestHarness::run_stress_test(nodes, iterations);
        }
        Some(("validate-log", _)) => {
            println!("Validating log...");
        }
        Some(("report", _)) => {
            println!("Generating report...");
        }
        _ => {}
    }
}
