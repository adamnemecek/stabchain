pub mod selector;

use criterion::{
    criterion_group,
    BenchmarkId,
    Criterion,
};
const RANGE_OF_VALUES: [usize; 5] = [8, 10, 16, 20, 32];
use {
    rand::{
        Rng,
        SeedableRng,
    },
    stabchain::{
        group::{
            stabchain::{
                base::selectors::DefaultSelector,
                builder::{
                    random::parameters::RandomAlgoParameters,
                    *,
                },
            },
            Group,
        },
        perm::actions::SimpleApplication,
    },
};

use num::BigUint;

///Macro for benchmarking a specific stabiliser chain strategy.
macro_rules! bench_stabchain_impl {
    ($bencher: ident, $name:expr, $i:ident, $group:tt, $strat:expr) => {
        $bencher.bench_with_input(BenchmarkId::new($name, $i), $i, |b, i| {
            let g = $group(i);
            b.iter(|| g.stabchain_with_strategy($strat()))
        });
    };
}

///Macro for benchmarking a specific stabiliser chain strategy with a known order.
macro_rules! bench_stabchain_impl_with_order {
    ($bencher: ident, $name:expr, $i:ident, $group:tt, $strat:expr) => {
        $bencher.bench_with_input(BenchmarkId::new($name, $i), $i, |b, i| {
            let g = $group(i);
            let order = g.stabchain().order();
            b.iter(|| g.stabchain_with_strategy($strat(order.clone())))
        });
    };
}

fn stabchain_cyclic(c: &mut Criterion) {
    let mut group = c.benchmark_group("group__stabchain__ss__cyclic");
    // Rng to generate seeds for stategy rngs.
    let mut rng = rand_xorshift::XorShiftRng::from_seed([42; 16]);
    for i in RANGE_OF_VALUES.iter() {
        group.bench_with_input(BenchmarkId::new("default", i), i, |b, i| {
            let g = Group::cyclic(*i);
            b.iter(|| g.stabchain())
        });
        bench_stabchain_impl!(group, "naive", i, (|i: &usize| Group::cyclic(*i)), || {
            NaiveBuilderStrategy::new(SimpleApplication::default(), DefaultSelector::default())
        });
        bench_stabchain_impl!(group, "ift", i, (|i: &usize| Group::cyclic(*i)), || {
            IftBuilderStrategy::new(SimpleApplication::default(), DefaultSelector::default())
        });
        bench_stabchain_impl!(group, "random_shallow", i, (|i: &usize| Group::cyclic(*i)), || {
            RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default().rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        });
        bench_stabchain_impl_with_order!(
            group,
            "random_shallow_known_order",
            i,
            (|i: &usize| Group::cyclic(*i)),
            |i: BigUint| RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .order(i)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
        bench_stabchain_impl_with_order!(
            group,
            "random_shallow_known_order_quick_test",
            i,
            (|i: &usize| Group::cyclic(*i)),
            |i: BigUint| RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .order(i)
                    .quick_test(true)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
        bench_stabchain_impl!(
            group,
            "random_shallow_quick_test",
            i,
            (|i: &usize| Group::cyclic(*i)),
            || RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .quick_test(true)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
    }
    group.finish();
}

fn stabchain_symmetric(c: &mut Criterion) {
    let mut group = c.benchmark_group("group__stabchain__ss__symmetric");
    // Rng to generate seeds for stategy rngs.
    let mut rng = rand_xorshift::XorShiftRng::from_seed([42; 16]);
    for i in RANGE_OF_VALUES.iter() {
        group.bench_with_input(BenchmarkId::new("default", i), i, |b, i| {
            let g = Group::symmetric(*i);
            b.iter(|| g.stabchain())
        });
        bench_stabchain_impl!(group, "naive", i, (|i: &usize| Group::symmetric(*i)), || {
            NaiveBuilderStrategy::new(SimpleApplication::default(), DefaultSelector::default())
        });
        bench_stabchain_impl!(group, "ift", i, (|i: &usize| Group::symmetric(*i)), || {
            IftBuilderStrategy::new(SimpleApplication::default(), DefaultSelector::default())
        });
        bench_stabchain_impl!(group, "random_shallow", i, (|i: &usize| Group::symmetric(*i)), || {
            RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default().rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        });
        bench_stabchain_impl!(
            group,
            "random_shallow_quick_test",
            i,
            (|i: &usize| Group::symmetric(*i)),
            || RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .quick_test(true)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
        bench_stabchain_impl_with_order!(
            group,
            "random_shallow_known_order",
            i,
            (|i: &usize| Group::symmetric(*i)),
            |i: BigUint| RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .order(i)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
        bench_stabchain_impl_with_order!(
            group,
            "random_shallow_known_order_quick_test",
            i,
            (|i: &usize| Group::symmetric(*i)),
            |i: BigUint| RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .order(i)
                    .quick_test(true)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
    }
    group.finish();
}

fn stabchain_direct_product_symm(c: &mut Criterion) {
    let mut group = c.benchmark_group("group__stabchain__ss__product_symmetric");
    // Rng to generate seeds for stategy rngs.
    let mut rng = rand_xorshift::XorShiftRng::from_seed([42; 16]);
    for i in RANGE_OF_VALUES.iter() {
        group.bench_with_input(BenchmarkId::new("default", i), i, |b, i| {
            let g = Group::product(&Group::symmetric(*i), &Group::symmetric(*i));
            b.iter(|| g.stabchain())
        });
        group.sample_size(20);
        bench_stabchain_impl!(
            group,
            "naive",
            i,
            (|i: &usize| Group::product(&Group::symmetric(*i), &Group::symmetric(*i))),
            || NaiveBuilderStrategy::new(SimpleApplication::default(), DefaultSelector::default())
        );
        bench_stabchain_impl!(
            group,
            "ift",
            i,
            (|i: &usize| Group::product(&Group::symmetric(*i), &Group::symmetric(*i))),
            || IftBuilderStrategy::new(SimpleApplication::default(), DefaultSelector::default())
        );
        bench_stabchain_impl!(
            group,
            "random_shallow",
            i,
            (|i: &usize| Group::product(&Group::symmetric(*i), &Group::symmetric(*i))),
            || RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default().rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
        bench_stabchain_impl!(
            group,
            "random_shallow_quick_test",
            i,
            (|i: &usize| Group::product(&Group::symmetric(*i), &Group::symmetric(*i))),
            || RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .quick_test(true)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen()))
            )
        );
        bench_stabchain_impl_with_order!(
            group,
            "random_shallow_known_order",
            i,
            (|i: &usize| Group::product(&Group::symmetric(*i), &Group::symmetric(*i))),
            |i: BigUint| RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .order(i)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
        bench_stabchain_impl_with_order!(
            group,
            "random_shallow_known_order_quick_test",
            i,
            (|i: &usize| Group::product(&Group::symmetric(*i), &Group::symmetric(*i))),
            |i: BigUint| RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .order(i)
                    .quick_test(true)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
    }
    group.finish();
}

fn stabchain_copies_of_cyclic(c: &mut Criterion) {
    use stabchain::group::utils::copies_of_cyclic;
    let mut group = c.benchmark_group("group__stabchain__ss__copies_cyclic");
    // Rng to generate seeds for stategy rngs.
    let mut rng = rand_xorshift::XorShiftRng::from_seed([42; 16]);
    group.sample_size(10);
    for i in RANGE_OF_VALUES.iter() {
        group.bench_with_input(BenchmarkId::new("default", i), i, |b, i| {
            let g = copies_of_cyclic(&[*i, *i, *i, *i, *i]);
            b.iter(|| g.stabchain())
        });
        bench_stabchain_impl!(
            group,
            "naive",
            i,
            (|i: &usize| copies_of_cyclic(&[*i, *i, *i, *i, *i])),
            || NaiveBuilderStrategy::new(SimpleApplication::default(), DefaultSelector::default())
        );
        bench_stabchain_impl!(
            group,
            "ift",
            i,
            (|i: &usize| copies_of_cyclic(&[*i, *i, *i, *i, *i])),
            || IftBuilderStrategy::new(SimpleApplication::default(), DefaultSelector::default())
        );
        bench_stabchain_impl!(
            group,
            "random_shallow",
            i,
            (|i: &usize| copies_of_cyclic(&[*i, *i, *i, *i, *i])),
            || RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default().rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
        bench_stabchain_impl!(
            group,
            "random_shallow_quick_test",
            i,
            (|i: &usize| copies_of_cyclic(&[*i, *i, *i, *i, *i])),
            || RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .quick_test(true)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen()))
            )
        );
        bench_stabchain_impl_with_order!(
            group,
            "random_shallow_known_order",
            i,
            (|i: &usize| Group::product(&Group::symmetric(*i), &Group::symmetric(*i))),
            |i: BigUint| RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .order(i)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
        bench_stabchain_impl_with_order!(
            group,
            "random_shallow_known_order_quick_test",
            i,
            (|i: &usize| Group::product(&Group::symmetric(*i), &Group::symmetric(*i))),
            |i: BigUint| RandomBuilderStrategyShallow::new_with_params(
                SimpleApplication::default(),
                DefaultSelector::default(),
                RandomAlgoParameters::default()
                    .order(i)
                    .quick_test(true)
                    .rng(rand_xorshift::XorShiftRng::from_seed(rng.gen())),
            )
        );
    }
    group.finish();
}

criterion_group!(
    stabchain,
    stabchain_cyclic,
    stabchain_symmetric,
    stabchain_direct_product_symm,
    stabchain_copies_of_cyclic,
);
