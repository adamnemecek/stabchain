use super::parameters::RandomAlgoParameters;

#[allow(deprecated)]
use crate::group::orbit::transversal::factored_transversal::{
    factored_transversal_complete_opt,
    representative_raw_as_word,
};
use {
    crate::{
        group::{
            orbit::abstraction::{
                FactoredTransversalResolver,
                TransversalResolver,
            },
            stabchain::{
                base::selectors::BaseSelector,
                order,
                Stabchain,
                StabchainRecord,
            },
            Group,
        },
        perm::{
            actions::SimpleApplication,
            Action,
            Permutation,
        },
        DetHashMap,
    },
    itertools::Itertools,
    rand::{
        rngs::ThreadRng,
        seq::{
            IteratorRandom,
            SliceRandom,
        },
        Rng,
    },
    std::{
        cell::RefCell,
        collections::VecDeque,
        fmt::Debug,
        iter::repeat_with,
    },
};

use tracing::{
    debug,
    trace,
};

// Helper struct, used to build the stabilizer chain

#[derive(Debug)]
pub struct StabchainBuilderRandom<P, S, A = SimpleApplication<P>, R = ThreadRng>
where
    A: Action<P, OrbitT = usize>,
    P: Permutation,
{
    current_pos: usize,
    chain: Vec<StabchainRecord<P, FactoredTransversalResolver<A>, A>>,
    selector: S,
    action: A,
    // The maximum degree of the permutations this group generates.
    n: usize,
    base: Vec<A::OrbitT>,
    rng: RefCell<R>,
    constants: super::parameters::Constants,
    //The chain is zero indexed, but this field is 1 indexed.
    //This is due to the end condition being when the chain is up to date below the index of the first record position, and this would be -1 with zero indexing.
    //For zero indexing this would have to be a signed type, which doesn't really seem worth it just to require one negative value at the end condition.
    up_to_date: usize,
}

#[allow(deprecated)]
impl<P, S, A, R> StabchainBuilderRandom<P, S, A, R>
where
    P: Permutation,
    S: BaseSelector<P, A::OrbitT>,
    A: Action<P, OrbitT = usize>,
    R: Rng,
{
    pub fn new(selector: S, action: A, params: RandomAlgoParameters<R>) -> Self {
        let (constants, random) = params.consts();

        Self {
            current_pos: 0,
            chain: Vec::new(),
            selector,
            action,
            n: 0,
            base: Vec::new(),
            constants,
            rng: RefCell::new(random),
            up_to_date: 1,
        }
    }

    fn current_chain(&self) -> impl Iterator<Item = &StabchainRecord<P, FactoredTransversalResolver<A>, A>> {
        self.chain.iter().skip(self.current_pos)
    }

    pub(super) fn build(self) -> Stabchain<P, FactoredTransversalResolver<A>, A> {
        Stabchain { chain: self.chain }
    }

    fn construct_strong_generating_set(&mut self, group: &Group<P>) {
        trace!(group = %group, "Constructing Strong Generating Set");
        //Edge case for trivial group.
        if group.generators().is_empty() {
            return;
        }
        //Find the largest moved point of any generator, i.e find which size of the symmetric group the generators form a subgroup of.
        // The minus 1 is to account for this being zero indexed, e.g S_4 moves points 0..3.
        self.n = group.symmetric_super_order() - 1;
        //Pick the initial moved point.
        let moved_point = group
            .generators()
            .iter()
            .map(|p| self.selector.moved_point(p, 0))
            .min()
            .unwrap();
        debug!(group = %group, moved_point = moved_point, "Adding initial record");
        //Create the top level record for this chain, and add it to the chain.
        //TODO check if you should add generators 1 by 1, in case there are redundant generators.
        let initial_record = StabchainRecord::new(
            moved_point,
            group.clone(),
            factored_transversal_complete_opt(group, moved_point, &self.action),
        );
        self.base.push(moved_point);
        self.chain.push(initial_record);
        self.sgc();
    }

    /// Generate a permutation that is with high probably a schrier generator for the current subgroup.
    fn random_schrier_generators_as_word(
        &self,
        subproducts: usize,
        coset_representatives: usize,
        gens: &[P],
    ) -> Vec<Vec<P>> {
        debug!(level = self.current_pos, "Random generation of Schrier Generators");
        //Sum of all "depths". In reality the transversal doesn't have a depth, so we use this as a upper bound.
        let t = self
            .chain
            .iter()
            .map(|record| record.transversal.len() + record.gens.generators().len())
            .sum::<usize>();
        let record = &self.chain[self.current_pos];
        //Create an iterator of subproducts w and w2
        let subproduct_w1_iter = repeat_with(|| random_subproduct_word_full(&mut *self.rng.borrow_mut(), gens));
        let subproduct_w2_iter = repeat_with(|| {
            let k = rand::Rng::gen_range(&mut *self.rng.borrow_mut(), 0..1 + gens.len() / 2);
            random_subproduct_word_subset(&mut *self.rng.borrow_mut(), gens, k)
        });
        //Iterleave the two iterators.
        let subproduct_iter: Vec<Vec<P>> = subproduct_w1_iter
            .interleave(subproduct_w2_iter)
            .take(2 * subproducts)
            .collect();
        //TODO check if precalculating all transversal elements would be faster.
        // Iterator of random coset representatives.
        let g_iter = repeat_with(|| {
            record
                .transversal
                .keys()
                .choose(&mut *self.rng.borrow_mut())
                .map(|point| {
                    representative_raw_as_word(&record.transversal, record.base, *point, &self.action).unwrap()
                })
                .expect("should be present")
        })
        .take(coset_representatives * t);
        //Create an iterator that contains combintations of each coset representative with each pair of subproducts.
        g_iter
            .flat_map(|g| {
                subproduct_iter.iter().map(move |w| {
                    let mut gw = g.clone();
                    gw.extend(w.clone());
                    gw
                })
            })
            .collect()
    }

    /// Check if adding a new element modifies the current layer of the chain.
    fn check_transversal_augmentation(&mut self, p: P) {
        debug!(level = self.current_pos, perm = %p, "Checking transversal augmentation with perm");
        let mut record = self.chain[self.current_pos].clone();
        //debug_assert!(record.gens.generators().contains(&p));
        // If this element is already a generator, then we can exit
        let mut to_check: VecDeque<_> = record.transversal.keys().cloned().collect();
        let mut new_transversal = DetHashMap::default();
        while let Some(orbit_element) = to_check.pop_back() {
            let new_image = self.action.apply(&p, orbit_element);

            // If we haven't seen this element.
            if !(record.transversal.contains_key(&new_image) || new_transversal.contains_key(&new_image)) {
                new_transversal.insert(new_image, p.inv());
            }
        }

        // We now want to check all the newly added elements
        let mut to_check: VecDeque<_> = new_transversal.keys().cloned().collect();

        // Update the record
        record.transversal.extend(new_transversal);

        // While we have orbit elements (and representatives to check)
        while let Some(orbit_element) = to_check.pop_back() {
            // Get the pair
            // For each generator (and p)
            for generator in std::iter::once(&p).chain(record.gens.generators()) {
                let new_image = self.action.apply(generator, orbit_element);
                // If we haven't already seen the image
                record.transversal.entry(new_image).or_insert_with(|| {
                    // Update and ask to check the new image
                    to_check.push_back(new_image);
                    generator.inv()
                });
            }
        }
        // Update the generators adding p if it isn't already present.
        if !record.gens.generators().contains(&p) {
            debug!("Adding perm to generating set");
            record.gens = std::iter::once(&p).chain(record.gens.generators()).cloned().collect();
        }
        // Store the updated record in the chain
        self.chain[self.current_pos] = record;
    }

    ///Check if the permutation augments the orbit at a level, resetting the position afterwards.
    fn check_transversal_augmentation_at_level(&mut self, level: usize, p: P) {
        let previous_pos = self.current_pos;
        self.current_pos = level;
        self.check_transversal_augmentation(p);
        self.current_pos = previous_pos;
    }

    fn sgc(&mut self) {
        trace!(level = self.current_pos, "Strong Generating Set Construction");
        let record = self.chain[self.current_pos].clone();
        //Number of base points than are in the current orbit.
        let b_star = self.base.iter().filter(|&b| record.transversal.contains_key(b)).count();
        let gens = self
            .current_chain()
            .flat_map(|record| record.gens.generators())
            .cloned()
            .collect::<Vec<P>>();
        //Random products of the form gw
        let mut random_gens = self.random_schrier_generators_as_word(self.constants.c1, self.constants.c2, &gens[..]);
        //Convert these into random schrier generators, by concatenating the resdiue of the inverse to it.
        random_gens.iter_mut().for_each(|gw| {
            //Get the residue of this word
            let (_, gw_bar) = residue_as_words_from_words(self.current_chain(), &gw.clone());
            //Append the inverse of the residue to the word, to get a schrier generator.
            gw.extend(gw_bar.iter().map(|p| p.inv()).rev());
        });
        //To see if all generators are discarded.
        let mut all_discarded = true;
        for h in random_gens {
            let (drop_out_level, h_residue) = residue_as_words_from_words(self.current_chain(), &h);
            if self.sifted(drop_out_level) {
                //Pick the points that should be evaluated. This is a heuristic to speed up run times.
                let evaluated_points: Vec<A::OrbitT> = if record.transversal.len() <= self.constants.orbit_bound {
                    //Evaluate on all points of the current orbit.
                    record.transversal.keys().cloned().collect()
                } else if self.base.len() <= self.constants.base_bound {
                    //Evaluate on BASE_BOUND randomly chosen points.
                    record
                        .transversal
                        .keys()
                        .choose_multiple(&mut *self.rng.borrow_mut(), self.constants.base_bound)
                        .into_iter()
                        .cloned()
                        .collect()
                } else {
                    //Evaluate on b_star randomly chosen points.
                    record
                        .transversal
                        .keys()
                        .choose_multiple(&mut *self.rng.borrow_mut(), b_star)
                        .into_iter()
                        .cloned()
                        .collect()
                };
                //If any point is not fixed by the residue, then we add the residue as a generator.
                if !self.is_trivial_residue(&h_residue, evaluated_points) {
                    //Not all permutations have been discarded
                    all_discarded = false;
                    let h_star = collapse_perm_word(&h_residue);
                    //Add a new base point, along with a new record for that base point.
                    let new_base_point = self.selector.moved_point(&h_star, self.current_pos);
                    //self.check_transversal_augmentation(h_star);
                    debug_assert!(!self.base.contains(&new_base_point));
                    debug!(perm = %h_star, moved_point = new_base_point, "Extending chain");
                    self.base.push(new_base_point);
                    //Fields for the new record.
                    let gens = Group::new(&[h_star]);
                    let transversal = factored_transversal_complete_opt(&gens, new_base_point, &self.action);
                    let record = StabchainRecord::new(new_base_point, gens, transversal);
                    self.chain.push(record);
                    //Now up to date beneath the newly added point.
                    self.up_to_date = self.base.len() + 1;
                }
            } else {
                //We have found a residue that has not sifted through, so we add a new base point with this point as a generator.
                let h_star = collapse_perm_word(&h_residue);
                //Find the position at which this didn't sift through.
                let j = self.current_pos + drop_out_level;
                debug!(perm = %h_star, level = j, "Permutation not sifting through");
                //Add as a generator and update the transversal.
                self.check_transversal_augmentation_at_level(j, h_star);
                //Consider the chain now up to date below level j + 1. The +1 is for 1 indexing.
                self.up_to_date = j + 1;
            }
        }
        if all_discarded {
            debug!(level = self.current_pos, "All generators discarded");
            //Really is setting this to i - 1, but as the position is zero indexed it would be doing (i - 1 + 1).
            self.up_to_date = self.current_pos;
        }
        //SGC terminates if it is up to date at position 0; otherwise moving onto the next layers. Stop if we have reached the bottom of the chain
        if self.up_to_date != 0 && self.current_pos != self.chain.len() - 1 {
            self.current_pos += 1;
            self.sgc();
        } else {
            self.sgt();
        }
    }

    /// Test that the current strong generating set is indeed a strong generating set, returning true if it (probably) is.
    fn sgt(&mut self) {
        trace!("Strong Generating Test");
        let original_position = self.current_pos;
        //Should be at the top of the chain, I think.
        self.current_pos = 0;
        let mut current_order = None;
        // If we know the order we can just check if the order is correct.
        if let Some(known_order) = self.constants.order.as_ref() {
            let current_order_val = order(self.chain.iter());
            if *known_order == current_order_val {
                // Call the sgc starting from the top level
                return;
            }
            // Save for later check
            current_order = Some(current_order_val);
        }
        //The union of the generator sets in the chain to this point.
        let gens = self
            .chain
            .iter()
            .flat_map(|record| record.gens.generators())
            .cloned()
            .collect::<Vec<P>>();
        //Create an iterator that first has the original generators, and then the random schrier generators.
        let products: Vec<Vec<P>> = self.chain[0]
            .gens
            .generators
            .iter()
            .map(|p| vec![p.clone()])
            .chain(self.random_schrier_generators_as_word(self.constants.c3, self.constants.c4, &gens[..]))
            .collect();
        //Sift the original generators, and all products of the form g*w_{1,2}.
        for p in products {
            //If we have found a non-trivial element, then invoke the sgc at the specified level.
            if let Some(sgc_invoke_level) = self.sgt_test(&p[..]) {
                debug!(level = sgc_invoke_level, "SGT failed");
                self.current_pos = sgc_invoke_level;
                self.sgc();
                break;
            };
        }
        // Make sure we don't exit without having the correct order.
        if let Some(known_order) = self.constants.order.as_ref() {
            if *known_order != current_order.unwrap() {
                // Call the sgc starting from the top level
                self.sgc();
            }
        }
        self.current_pos = original_position;
    }

    fn sgt_test(&mut self, p: &[P]) -> Option<usize> {
        let (drop_out_level, residue) = residue_as_words_from_words(self.current_chain(), p);
        //Check if this is a non-trivial residue. If it is then the output of the SGC is correct for this element.
        if !self.is_trivial_residue_all_points(&residue) {
            let invoke_level = self.current_pos + drop_out_level;
            let collapsed_residue = collapse_perm_word(&residue);
            //If this point sifted through but isn't trivial, then we need a new record and base point.
            if self.sifted(drop_out_level) {
                let moved_point = self.selector.moved_point(&collapsed_residue, self.current_pos);
                debug!(perm = %collapsed_residue, moved_point = moved_point, "Extending chain");
                let gens = Group::new(&[collapsed_residue]);
                let transversal = factored_transversal_complete_opt(&gens, moved_point, &self.action);
                let initial_record = StabchainRecord::new(moved_point, gens, transversal);
                self.base.push(moved_point);
                self.chain.push(initial_record);
                self.up_to_date = self.base.len() + 1;
            } else {
                //Otherwise add it to the generators at that level, and invoke the SGC at that level.
                self.check_transversal_augmentation_at_level(invoke_level, collapsed_residue);
                self.up_to_date = self.current_pos + drop_out_level + 1
            }
            Some(self.current_pos + drop_out_level)
        } else {
            None
        }
    }

    /// Wrapper function to check all points of the permutation domain.
    fn is_trivial_residue_all_points(&self, p_as_words: &[P]) -> bool {
        self.is_trivial_residue(p_as_words, 0..self.n)
    }

    /// Check if a residue acts trivially on a set of points.
    /// This is just done by checking that the given permutation fixes all given points.
    fn is_trivial_residue(&self, p_as_words: &[P], points: impl IntoIterator<Item = A::OrbitT>) -> bool {
        points
            .into_iter()
            .all(|x| apply_permutation_word(p_as_words, x, &self.action) == x)
    }

    //Utility function to check if a given drop out level is the bottom of the chain.
    fn sifted(&self, drop_out_level: usize) -> bool {
        self.current_pos + drop_out_level == self.chain.len()
    }
}

impl<P, S, A, R> crate::group::stabchain::builder::Builder<P, FactoredTransversalResolver<A>, A>
    for StabchainBuilderRandom<P, S, A, R>
where
    P: Permutation,
    A: Action<P, OrbitT = usize>,
    S: BaseSelector<P, A::OrbitT>,
    R: Rng + Debug,
{
    fn set_generators(&mut self, gens: &Group<P>) {
        self.construct_strong_generating_set(gens);
    }

    fn build(self) -> Stabchain<P, FactoredTransversalResolver<A>, A> {
        self.build()
    }
}

// Functions used for compatability reasons
/// Generate a word representation of a random subproduct of the given generators.
fn random_subproduct_word_subset<R, P>(rng: &mut R, gens: &[P], k: usize) -> Vec<P>
where
    P: Permutation,
    R: Rng,
{
    // TODO: CHeck 1 + k/2
    gens.choose_multiple(rng, k)
        .filter(|_| rng.gen::<bool>())
        .cloned()
        .collect::<Vec<P>>()
}

/// Generate random subproduct of the given generators.
fn random_subproduct_word_full<T, P>(rng: &mut T, gens: &[P]) -> Vec<P>
where
    P: Permutation,
    T: Rng,
{
    random_subproduct_word_subset(rng, gens, gens.len())
}

/// Apply a point to permutations stored as a word.
fn apply_permutation_word<'a, P, A>(perm_word: impl IntoIterator<Item = &'a P>, x: A::OrbitT, strat: &A) -> A::OrbitT
where
    P: 'a + Permutation,
    A: Action<P>,
{
    perm_word.into_iter().fold(x, |accum, p| strat.apply(p, accum))
}

/// Convert from a permutation stored as a word, into a single permutation.
fn collapse_perm_word<'a, P>(p: impl IntoIterator<Item = &'a P>) -> P
where
    P: 'a + Permutation,
{
    p.into_iter()
        .fold(Permutation::id(), |accum, perm| accum.multiply(perm))
}

/// Sift the permutation word through the chain, returning the residue it generates and the drop out level.
fn residue_as_words_from_words<'a, 'b, V, A, P>(
    it: impl IntoIterator<Item = &'a StabchainRecord<P, V, A>>,
    p: impl IntoIterator<Item = &'b P>,
) -> (usize, Vec<P>)
where
    V: 'a + TransversalResolver<P, A>,
    P: 'a + 'b + Permutation,
    A: 'a + Action<P>,
{
    //This permutation word will store the resulting residue.
    let mut g: Vec<P> = p.into_iter().cloned().collect();
    //This counts how many layers of the chain the permutation sifts through.
    let mut k = 0;
    let applicator = A::default();
    for record in it {
        let base = record.base.clone();
        let application = apply_permutation_word(&g, base.clone(), &applicator);

        //There is a missing point, so this permutation has not sifted through.
        if !record.transversal.contains_key(&application) {
            break;
        }
        //Already check the point is present, so there should be a representative.
        let representative = record
            .resolver()
            .representative(&record.transversal, base.clone(), application)
            .unwrap();
        g.push(representative.inv());
        k += 1;
    }
    (k, g)
}

mod tests {

    ///Test that applying a permutation as a word gives the same image as collapsing that permutation.
    #[test]
    fn test_apply_permutation_word() {
        use {
            super::{
                apply_permutation_word,
                collapse_perm_word,
            },
            crate::perm::{
                actions::SimpleApplication,
                export::CyclePermutation,
                impls::standard::StandardPermutation,
                Permutation,
            },
        };
        //Test an empty word.
        let empty_word = vec![];
        let strat = SimpleApplication::default();
        assert_eq!(3, apply_permutation_word(&empty_word, 3, &strat));
        let perm_word: Vec<StandardPermutation> = vec![
            CyclePermutation::single_cycle(&[1, 2, 4]).into(),
            CyclePermutation::single_cycle(&[3, 5, 8]).into(),
            CyclePermutation::single_cycle(&[7, 9]).into(),
            CyclePermutation::single_cycle(&[1, 5, 6, 9]).into(),
        ];
        let collapsed_word = collapse_perm_word(&perm_word);
        for i in 0..9 {
            assert_eq!(collapsed_word.apply(i), apply_permutation_word(&perm_word, i, &strat));
        }
    }
}
