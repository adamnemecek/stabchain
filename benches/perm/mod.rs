pub mod comparison;

use criterion::{
    black_box,
    criterion_group,
    BenchmarkId,
    Criterion,
};

use std::iter::FromIterator;

use stabchain::perm::{
    builder::PermBuilder,
    impls::standard::StandardPermutation,
    utils::random_permutation,
    *,
};

const RANGE_OF_VALUES: [usize; 7] = [8, 16, 32, 64, 128, 256, 512];

/// How costly is it to instantiate a random permutation
fn random_instantiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("permutation__random_creation");
    for i in RANGE_OF_VALUES.iter() {
        group.bench_with_input(BenchmarkId::new("default", i), i, |b, i| {
            b.iter(|| random_permutation::<DefaultPermutation>(*i))
        });
    }

    group.finish();
}

/// Checks the common case of computing (a * b)^-1 = b^-1 * a^-1
fn inverse_of_product(c: &mut Criterion) {
    use stabchain::perm::algos::inv;
    let mut group = c.benchmark_group("permutation__inv_prod");
    for i in RANGE_OF_VALUES.iter() {
        group.bench_with_input(BenchmarkId::new("default", i), i, |b, i| {
            let first = random_permutation::<StandardPermutation>(*i);
            let second = random_permutation(*i);
            b.iter(|| black_box(inv(&first.multiply(&second))))
        });
        group.bench_with_input(BenchmarkId::new("prod_of_inv", i), i, |b, i| {
            let first = random_permutation::<StandardPermutation>(*i);
            let second = random_permutation(*i);
            b.iter(|| {
                let first = inv(&first);
                let second = inv(&second);

                second.multiply(&first)
            })
        });
    }
    group.finish();
}

// Use specialized exponentitation vs generic multi join
fn exponentiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("permutation__exp");
    // Note, we use permutations of S_2n to the power of n, in order to avoid id as much as possible
    for i in RANGE_OF_VALUES.iter().map(|i| i * 2) {
        group.bench_with_input(BenchmarkId::new("pow", i), &i, |b, i| {
            let perm = random_permutation::<DefaultPermutation>(*i);
            b.iter(|| perm.pow((i / 2) as isize))
        });
        group.bench_with_input(BenchmarkId::new("multijoin", i), &i, |b, i| {
            use stabchain::perm::builder::join::MultiJoin;
            let perm = random_permutation::<DefaultPermutation>(*i);
            let join = MultiJoin::from_iter(std::iter::repeat_n(perm, i / 2));
            b.iter(|| join.collapse())
        });
    }
    group.finish();
}

fn exponentiation_small_exponent(c: &mut Criterion) {
    let mut group = c.benchmark_group("permutation__small_exp");
    for i in (1..16).step_by(2) {
        group.bench_with_input(BenchmarkId::new("pow", i), &i, |b, i| {
            let perm = random_permutation::<DefaultPermutation>(1024);
            b.iter(|| perm.pow(*i as isize))
        });
        group.bench_with_input(BenchmarkId::new("multijoin", i), &i, |b, i| {
            use stabchain::perm::builder::join::MultiJoin;
            let perm = random_permutation::<DefaultPermutation>(1024);
            let join = MultiJoin::from_iter(std::iter::repeat_n(perm, *i));
            b.iter(|| join.collapse())
        });
    }
    group.finish();
}

fn order_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("permutation__order_cmp");
    for i in [8, 16, 32, 64, 100].iter() {
        group.bench_with_input(BenchmarkId::new("default", i), i, |b, i| {
            let perm = random_permutation::<DefaultPermutation>(*i);
            b.iter(|| perm.order())
        });
        group.bench_with_input(BenchmarkId::new("iterated_mult", i), i, |b, i| {
            use crate::perm::algos::order_mult;
            let perm = random_permutation::<DefaultPermutation>(*i);
            b.iter(|| order_mult(&perm))
        });
        group.bench_with_input(BenchmarkId::new("cycle", i), i, |b, i| {
            use crate::perm::algos::order_cycle;
            let perm = random_permutation::<DefaultPermutation>(*i);
            b.iter(|| order_cycle(&perm))
        });
        group.bench_with_input(BenchmarkId::new("parallel", i), i, |b, i| {
            use {
                rayon::prelude::*,
                stabchain::perm::impls::sync::SyncPermutation,
            };
            let perm = random_permutation::<SyncPermutation>(*i);
            b.iter(|| match perm.lmp() {
                Some(n) => (0..n)
                    .into_par_iter()
                    .map(|i| (i, perm.pow(i as isize)))
                    .filter(|t| t.1.is_id())
                    .map(|t| t.0)
                    .min()
                    .unwrap(),
                None => 1,
            });
        });
    }
    group.finish();
}

/// Benchmark the check of an identity, although this should be constant due to it being an empty check.
fn identity_check(c: &mut Criterion) {
    let id = DefaultPermutation::id();
    c.bench_function("permutation__is_id", |b| b.iter(|| id.is_id()));
}

/// Benchmarking for inverting of elements.
fn inverse(c: &mut Criterion) {
    use stabchain::perm::algos::inv;
    let mut group = c.benchmark_group("permutation__inverse");
    for i in RANGE_OF_VALUES.iter() {
        group.bench_with_input(BenchmarkId::new("default", i), i, |b, i| {
            let perm = random_permutation(*i);
            b.iter(|| inv(&perm))
        });
    }
    group.finish();
}

criterion_group!(
    permutation,
    random_instantiation,
    inverse_of_product,
    identity_check,
    inverse,
    exponentiation,
    exponentiation_small_exponent,
    order_efficiency,
);
