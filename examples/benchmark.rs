use {
    std::{
        fs::File,
        io::BufReader,
        str::FromStr,
    },
    structopt::StructOpt,
};

use stabchain::{
    group::{
        group_library::DecoratedGroup,
        stabchain::{
            base::selectors::LmpSelector,
            builder::*,
        },
    },
    perm::{
        actions::SimpleApplication,
        export::ExportablePermutation,
        *,
    },
};

use std::time::Instant;

use criterion::black_box;

use tracing::Level;

#[derive(Debug)]
enum BenchMode {
    Deterministic,
    DeterministicIFT,
    Random,
    RandomShallow,
}

impl FromStr for BenchMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "deterministic" => Self::Deterministic,
            "ift" => Self::DeterministicIFT,
            "random" => Self::Random,
            "shallow" => Self::RandomShallow,
            _ => return Err("Could not parse".to_string()),
        })
    }
}

fn load_libraries(paths: &[&str]) -> Vec<DecoratedGroup<DefaultPermutation>> {
    paths.iter().flat_map(|p| group_library(p)).collect()
}

fn group_library(path: &str) -> impl IntoIterator<Item = DecoratedGroup<DefaultPermutation>> {
    let input = File::open(path).unwrap();
    let input = BufReader::new(input);

    let groups: Vec<DecoratedGroup<ExportablePermutation>> = serde_json::from_reader(input).unwrap();
    groups.into_iter().map(|g| g.map(DefaultPermutation::from))
}

#[derive(StructOpt)]
struct Arguments {
    #[structopt(short, long)]
    mode: BenchMode,
}

fn bench<S: BuilderStrategy<DefaultPermutation> + Clone>(lib: Vec<DecoratedGroup>, strategy: S) {
    println!("Starting benches ...");
    let progress_bar = indicatif::ProgressBar::new(lib.len() as u64);
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .progress_chars("##-"),
    );

    let start = Instant::now();
    for g in lib {
        let stabchain = g.group().stabchain_with_strategy(strategy.clone());
        black_box(stabchain);
        progress_bar.inc(1)
    }
    let duration = start.elapsed();

    progress_bar.finish_with_message(format!("Finished in {:?}", duration));

    println!("Finished in {:?}", duration);
}

fn main() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    #[cfg(debug_assertions)]
    {
        println!("Running benches in non release mode is not a good idea");
    }

    let args = Arguments::from_args();

    println!("Loading libraries");

    let group_library = load_libraries(&["data/small.json", "data/transitive.json"]);

    println!("Libraries loaded");

    #[allow(deprecated)]
    match args.mode {
        BenchMode::Deterministic => bench(
            group_library,
            DefaultStrategy::new(SimpleApplication::default(), LmpSelector),
        ),
        BenchMode::DeterministicIFT => bench(
            group_library,
            IftBuilderStrategy::new(SimpleApplication::default(), LmpSelector),
        ),
        BenchMode::Random => bench(
            group_library,
            RandomBuilderStrategyNaive::new(SimpleApplication::default(), LmpSelector),
        ),
        BenchMode::RandomShallow => bench(
            group_library,
            RandomBuilderStrategyShallow::new(SimpleApplication::default(), LmpSelector),
        ),
    }
}
