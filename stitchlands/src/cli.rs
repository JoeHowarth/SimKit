use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use stitchlands::{CliOptions, RunMode};

#[derive(Parser, Debug)]
#[command(name = "stitchlands", version, about = "Stitchlands runner", long_about = None)]
pub struct CliArgs {
    /// Mode: live (with window) or headless (no window, fixed ticks)
    #[arg(long, value_enum, default_value_t = ModeArg::Live)]
    pub mode: ModeArg,

    /// Scenario path (.ron)
    #[arg(long)]
    pub scenario: Option<PathBuf>,

    /// Load world from snapshot (.ron) instead of scenario
    #[arg(long)]
    pub snapshot: Option<PathBuf>,

    /// Number of FixedUpdate ticks to run (required in headless)
    #[arg(long, required_if_eq("mode", "headless"))]
    pub ticks: Option<u64>,

    /// RNG seed (default: 1)
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ModeArg {
    Live,
    Headless,
}

impl From<ModeArg> for RunMode {
    fn from(value: ModeArg) -> Self {
        match value {
            ModeArg::Live => RunMode::Live,
            ModeArg::Headless => RunMode::Headless,
        }
    }
}

pub fn parse_cli() -> CliOptions {
    let args = CliArgs::parse();
    CliOptions {
        mode: args.mode.into(),
        scenario: args.scenario,
        snapshot: args.snapshot,
        ticks: args.ticks,
        seed: args.seed,
    }
}
