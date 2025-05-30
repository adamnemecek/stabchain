//! Utility for generating random elements of a subgroup

use {
    super::Group,
    crate::perm::{
        impls::word::WordPermutation,
        DefaultPermutation,
        Permutation,
    },
    rand::{
        prelude::SliceRandom,
        rngs::ThreadRng,
        Rng,
    },
    std::cmp::max,
};

/// Calling random_element from this struct repetedly will generate random permutations in the subgroup
#[derive(Debug)]
pub struct RandPerm<P = DefaultPermutation, R = ThreadRng> {
    size: usize,
    rng: R,
    gen_elements: Vec<P>,
    accum: P,
}

impl<P, R> RandPerm<P, R>
where
    P: Permutation,
    R: Rng,
{
    /// Creates a rand perm, using a defined source of randomness
    pub fn new(min_size: usize, g: &Group<P>, initial_runs: usize, rng: R) -> Self {
        let mut gen_elements: Vec<_> = if !g.generators().is_empty() {
            g.generators().to_vec()
        } else {
            vec![P::id()]
        };
        let k = gen_elements.len();
        //Repeat elements if there aren't enough generators.
        for i in k..min_size {
            gen_elements.push(gen_elements[(i - k) % k].clone());
        }
        let accum = P::id();
        let size = max(min_size, k);
        let mut rand = Self {
            size,
            rng,
            gen_elements,
            accum,
        };
        // Inital randomisation.
        for _ in 0..initial_runs {
            rand.random_permutation();
        }
        rand
    }

    /// Generate a random permutation.
    /// ```
    /// use stabchain::perm::*;
    /// use stabchain::group::Group;
    /// use stabchain::group::random_perm::RandPerm;
    /// let generators = &[DefaultPermutation::from_images(&[1, 0]), DefaultPermutation::from_images(&[0, 2, 3, 1])];
    /// let mut rand_perm = RandPerm::from_generators(11, &Group::new(generators), 50);
    /// rand_perm.random_permutation();
    /// ```
    pub fn random_permutation(&mut self) -> P {
        let s = self.rng.gen_range(0..self.size);
        let mut t = s;
        // Generate another index that isn't equal to s.
        while t == s {
            t = self.rng.gen_range(0..self.size);
        }
        // Either take product or quotient.
        let e = if self.rng.gen::<bool>() { 1 } else { -1 };
        // Randomly determine order of operation.
        // The operation works by replacing a list entry with a product, and then accumulating with the stored permutation.
        if self.rng.gen::<bool>() {
            self.gen_elements[s] = self.gen_elements[s].multiply(&self.gen_elements[t].pow(e));
            self.accum = self.accum.multiply(&self.gen_elements[s]);
        } else {
            self.gen_elements[s] = self.gen_elements[t].pow(e).multiply(&self.gen_elements[s]);
            self.accum = self.gen_elements[s].multiply(&self.accum);
        }
        self.accum.clone()
    }
}

impl<P> RandPerm<P>
where
    P: Permutation,
{
    /// Construct and initialise a random permutation generator.
    /// ```
    /// use stabchain::perm::{Permutation, DefaultPermutation};
    /// use stabchain::group::Group;
    /// use stabchain::group::random_perm::RandPerm;
    /// let generators = &[DefaultPermutation::from_images(&[1, 0]), DefaultPermutation::from_images(&[0, 2, 3, 1])];
    /// let rand_perm = RandPerm::from_generators(11, &Group::new(generators), 50);
    /// ```
    pub fn from_generators(min_size: usize, g: &Group<P>, initial_runs: usize) -> Self {
        Self::new(min_size, g, initial_runs, rand::thread_rng())
    }
}

/// Perform a random walk of the cayley graph of a group.
pub fn random_cayley_walk<P, R>(g: &Group<P>, iters: usize, rng: &mut R) -> P
where
    P: Permutation,
    R: Rng,
{
    if g.generators.is_empty() {
        return P::id();
    }
    // Create a word permutation to reduce allocations.
    let mut p = WordPermutation::<P>::id_with_capacity(iters);
    // The multiply by iters random elements from the genset.
    for _ in 0..iters {
        let elem = g.generators.choose(rng).unwrap();
        let inv = elem.inv();
        p.multiply_mut(if rng.gen() { elem } else { &inv });
    }
    p.evaluate()
}

/// Perform a random walk of the cayley graph of a group, but optionally walking to neighbour vertex or staying put.
/// This is effectively a walk where we either multiply by identity or an element.
pub fn random_lazy_cayley_walk<P, R>(g: &Group<P>, iters: usize, rng: &mut R) -> P
where
    P: Permutation,
    R: Rng,
{
    if g.generators.is_empty() {
        return P::id();
    }
    // Create a word permutation to reduce allocations.
    let mut p = WordPermutation::<P>::id_with_capacity(iters);
    // The multiply by iters random elements from the genset.
    for _ in 0..iters {
        // Either multiply by random element or identity.
        if rng.gen() {
            let elem = g.generators.choose(rng).unwrap();
            p.multiply_mut(elem);
        }
    }
    p.evaluate()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    /// Test that only the indentity permutation is generated from an empty set of generators.
    fn empty_generators() {
        let id = DefaultPermutation::id();
        let mut rand_perm = RandPerm::from_generators(10, &Group::trivial(), 50);
        for _ in 0..50 {
            assert_eq!(id, rand_perm.random_permutation());
        }
    }
    #[test]
    /// Test that elements generated are in the subgroup generated by the generator
    fn closure_small() {
        let generator = DefaultPermutation::from_images(&[3, 0, 1, 2]);
        let elements = [generator.clone(), generator.pow(2), generator.pow(3), generator.pow(4)];
        let mut rand_perm = RandPerm::from_generators(10, &Group::new(&elements[..]), 50);
        for _ in 0..50 {
            assert!(elements.contains(&rand_perm.random_permutation()));
        }
    }

    #[test]
    /// Test that elements generated are in the subgroup for multiple generators.
    fn closure_larger_disjoint() {
        use crate::{
            group::Group,
            perm::export::CyclePermutation,
        };
        let g = Group::<DefaultPermutation>::new(&[
            CyclePermutation::single_cycle(&[1, 2, 4]).into(),
            CyclePermutation::single_cycle(&[3, 5, 8]).into(),
            CyclePermutation::single_cycle(&[7, 9]).into(),
        ]);
        let mut rand_perm = RandPerm::from_generators(10, &g, 50);
        // dbg!(&rand_perm);
        let chain = g.stabchain();
        for _ in 0..100 {
            let perm = rand_perm.random_permutation();
            assert!(chain.in_group(&perm));
        }
    }

    #[test]
    ///Test that random elements from the symmetric group are included. This is ignored as it is slow.
    fn closure_larger_symmetric() {
        use crate::group::Group;
        let g = Group::symmetric(20);
        let mut rand_perm = RandPerm::from_generators(10, &g, 1000);
        let chain = g.stabchain();
        for _ in 0..100 {
            let perm = rand_perm.random_permutation();
            assert!(chain.in_group(&perm));
        }
    }
}
