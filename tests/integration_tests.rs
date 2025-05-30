use std::{
    fs::File,
    io::BufReader,
    stringify,
};

use {
    lazy_static::lazy_static,
    rand::seq::IteratorRandom,
    rayon::prelude::*,
};

use stabchain::{
    group::{
        group_library::DecoratedGroup,
        orbit::transversal::valid_transversal,
        stabchain::{
            correct_stabchain_order,
            valid_stabchain,
        },
    },
    perm::{
        export::ExportablePermutation,
        impls::sync::SyncPermutation,
    },
};

use stabchain::{
    group::stabchain::{
        base::selectors::*,
        builder::{
            random::parameters::RandomAlgoParameters,
            *,
        },
    },
    perm::actions::*,
};

// We use this to limit the number of groups to test
const DEFAULT_LIMIT: usize = 1000;

lazy_static! {
    static ref GROUP_LIBRARY: Vec<DecoratedGroup<SyncPermutation>> = load_libraries("data.zip");
    static ref LIMIT: usize = std::env::var("STABCHAIN_GROUP_TESTING_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(DEFAULT_LIMIT);
    static ref NO_LIMIT: bool = std::env::var("STABCHAIN_GROUP_TESTING_NO_LIMIT").is_ok();
}

fn load_libraries(zip: &str) -> Vec<DecoratedGroup<SyncPermutation>> {
    let zip_file = File::open(zip).unwrap();
    let mut archive = zip::ZipArchive::new(zip_file).unwrap();

    let mut paths = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = file.sanitized_name();

        if let Some(p) = outpath.parent() {
            if !p.exists() {
                std::fs::create_dir_all(p).unwrap();
            }
        }

        let mut outfile = File::create(&outpath).unwrap();
        std::io::copy(&mut file, &mut outfile).unwrap();
        paths.push(outpath);
    }

    paths.iter().flat_map(|p| group_library(p)).collect()
}

fn group_library(path: &std::path::Path) -> impl IntoIterator<Item = DecoratedGroup<SyncPermutation>> {
    let input = File::open(path).unwrap();
    let input = BufReader::new(input);

    let groups: Vec<DecoratedGroup<ExportablePermutation>> = serde_json::from_reader(input).unwrap();
    groups.into_iter().map(|g| g.map(SyncPermutation::from))
}

fn number_of_tests() -> usize {
    if *NO_LIMIT {
        GROUP_LIBRARY.len()
    } else {
        *LIMIT
    }
}

fn general_test<F, E>(name: &str, validator: F, error_limit: usize)
where
    F: Fn(DecoratedGroup<SyncPermutation>) -> Result<(), E> + Send + Sync,
    E: std::fmt::Debug + Send,
{
    let mut rng = rand::thread_rng();

    let groups = if *NO_LIMIT {
        GROUP_LIBRARY.to_vec()
    } else {
        GROUP_LIBRARY.iter().cloned().choose_multiple(&mut rng, *LIMIT)
    };

    let errors = groups
        .par_iter()
        .cloned()
        .flat_map(|g| {
            let validation = validator(g.clone());
            validation.map_err(|err| (g, err)).err()
        })
        .collect::<Vec<_>>();

    println!("[{}] {} errors out of {}", name, errors.len(), number_of_tests());

    for (g, err) in &errors {
        println!("[{}] Error {:?}", name, &err);
        println!("[{}] Error on {}", name, g.group());
    }

    assert!(errors.len() <= error_limit);
}

#[test]
fn test_transversals() {
    general_test(
        "transversal",
        |g| {
            let transversal = g.group().transversal(0);
            valid_transversal(&transversal)
        },
        0,
    )
}

#[cfg(test)]
macro_rules! test_stabilizer_on_strategy {
    ($strategy:expr, $short:ident, $error: expr) => {
        #[test]
        fn $short() {
            general_test(
                stringify!($short),
                |g| {
                    let stabilizer = g.group().stabchain_with_strategy($strategy);
                    correct_stabchain_order(&stabilizer, g.order().clone())?;
                    valid_stabchain(&stabilizer)
                },
                $error,
            );
        }
    };
}

#[cfg(test)]
macro_rules! test_stabilizer_on_strategy_with_order {
    ($strategy:expr, $short:ident, $error: expr) => {
        #[test]
        fn $short() {
            general_test(
                stringify!($short),
                |g| {
                    let stabilizer = g.group().stabchain_with_strategy($strategy(g.order().clone()));
                    correct_stabchain_order(&stabilizer, g.order().clone())?;
                    valid_stabchain(&stabilizer)
                },
                $error,
            );
        }
    };
}

test_stabilizer_on_strategy!(
    NaiveBuilderStrategy::new(SimpleApplication::default(), LmpSelector,),
    test_naive_stabilizer,
    0
);

test_stabilizer_on_strategy!(
    IftBuilderStrategy::new(SimpleApplication::default(), LmpSelector,),
    test_ift_stabilizer,
    0
);

test_stabilizer_on_strategy!(
    RandomBuilderStrategyShallow::new(SimpleApplication::default(), FmpSelector,),
    test_random_shallow_stabilizer,
    (number_of_tests() as f32 * 0.005).floor() as usize
);

test_stabilizer_on_strategy!(
    RandomBuilderStrategyShallow::new_with_params(
        SimpleApplication::default(),
        FmpSelector,
        RandomAlgoParameters::default().quick_test(true)
    ),
    test_random_shallow_stabilizer_quick_test,
    (number_of_tests() as f32 * 0.01).floor() as usize
);

test_stabilizer_on_strategy_with_order!(
    |order| RandomBuilderStrategyShallow::new_with_params(
        SimpleApplication::default(),
        FmpSelector,
        RandomAlgoParameters::default().quick_test(true).order(order)
    ),
    test_random_shallow_known_order,
    0
);
