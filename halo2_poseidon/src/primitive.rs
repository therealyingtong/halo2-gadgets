//! The Poseidon algebraic hash function.

use std::array;
use std::fmt;
use std::iter;
use std::marker::PhantomData;

use pasta_curves::arithmetic::FieldExt;

pub(crate) mod fp;
#[allow(dead_code)]
pub(crate) mod fq;
pub(crate) mod grain;
pub(crate) mod mds;

#[cfg(test)]
pub(crate) mod test_vectors;

mod p128pow5t3;
pub use p128pow5t3::P128Pow5T3;

use grain::SboxType;

/// The type used to hold permutation state.
pub(crate) type State<F, const T: usize> = [F; T];

/// The type used to hold duplex sponge state.
pub(crate) type SpongeState<F, const RATE: usize> = [Option<F>; RATE];

/// The type used to hold the MDS matrix and its inverse.
pub(crate) type Mds<F, const T: usize> = [[F; T]; T];

/// A specification for a Poseidon permutation.
pub trait Spec<F: FieldExt, const T: usize, const RATE: usize> {
    /// The number of full rounds for this specification.
    ///
    /// This must be an even number.
    fn full_rounds() -> usize;

    /// The number of partial rounds for this specification.
    fn partial_rounds() -> usize;

    /// The S-box for this specification.
    fn sbox(val: F) -> F;

    /// Side-loaded index of the first correct and secure MDS that will be generated by
    /// the reference implementation.
    ///
    /// This is used by the default implementation of [`Spec::constants`]. If you are
    /// hard-coding the constants, you may leave this unimplemented.
    fn secure_mds(&self) -> usize;

    /// Generates `(round_constants, mds, mds^-1)` corresponding to this specification.
    fn constants(&self) -> (Vec<[F; T]>, Mds<F, T>, Mds<F, T>) {
        let r_f = Self::full_rounds();
        let r_p = Self::partial_rounds();

        let mut grain = grain::Grain::new(SboxType::Pow, T as u16, r_f as u16, r_p as u16);

        let round_constants = (0..(r_f + r_p))
            .map(|_| {
                let mut rc_row = [F::zero(); T];
                for (rc, value) in rc_row
                    .iter_mut()
                    .zip((0..T).map(|_| grain.next_field_element()))
                {
                    *rc = value;
                }
                rc_row
            })
            .collect();

        let (mds, mds_inv) = mds::generate_mds::<F, T>(&mut grain, self.secure_mds());

        (round_constants, mds, mds_inv)
    }
}

/// Runs the Poseidon permutation on the given state.
pub(crate) fn permute<F: FieldExt, S: Spec<F, T, RATE>, const T: usize, const RATE: usize>(
    state: &mut State<F, T>,
    mds: &Mds<F, T>,
    round_constants: &[[F; T]],
) {
    let r_f = S::full_rounds() / 2;
    let r_p = S::partial_rounds();

    let apply_mds = |state: &mut State<F, T>| {
        let mut new_state = [F::zero(); T];
        // Matrix multiplication
        #[allow(clippy::needless_range_loop)]
        for i in 0..T {
            for j in 0..T {
                new_state[i] += mds[i][j] * state[j];
            }
        }
        *state = new_state;
    };

    let full_round = |state: &mut State<F, T>, rcs: &[F; T]| {
        for (word, rc) in state.iter_mut().zip(rcs.iter()) {
            *word = S::sbox(*word + rc);
        }
        apply_mds(state);
    };

    let part_round = |state: &mut State<F, T>, rcs: &[F; T]| {
        for (word, rc) in state.iter_mut().zip(rcs.iter()) {
            *word += rc;
        }
        // In a partial round, the S-box is only applied to the first state word.
        state[0] = S::sbox(state[0]);
        apply_mds(state);
    };

    iter::empty()
        .chain(iter::repeat(&full_round as &dyn Fn(&mut State<F, T>, &[F; T])).take(r_f))
        .chain(iter::repeat(&part_round as &dyn Fn(&mut State<F, T>, &[F; T])).take(r_p))
        .chain(iter::repeat(&full_round as &dyn Fn(&mut State<F, T>, &[F; T])).take(r_f))
        .zip(round_constants.iter())
        .fold(state, |state, (round, rcs)| {
            round(state, rcs);
            state
        });
}

fn poseidon_duplex<F: FieldExt, S: Spec<F, T, RATE>, const T: usize, const RATE: usize>(
    state: &mut State<F, T>,
    input: &SpongeState<F, RATE>,
    pad_and_add: &dyn Fn(&mut State<F, T>, &SpongeState<F, RATE>),
    mds_matrix: &Mds<F, T>,
    round_constants: &[[F; T]],
) -> SpongeState<F, RATE> {
    pad_and_add(state, input);

    permute::<F, S, T, RATE>(state, mds_matrix, round_constants);

    let mut output = [None; RATE];
    for (word, value) in output.iter_mut().zip(state.iter()) {
        *word = Some(*value);
    }
    output
}

#[derive(Debug)]
pub(crate) enum Sponge<F, const RATE: usize> {
    Absorbing(SpongeState<F, RATE>),
    Squeezing(SpongeState<F, RATE>),
}

impl<F: Copy, const RATE: usize> Sponge<F, RATE> {
    pub(crate) fn absorb(val: F) -> Self {
        let mut input = [None; RATE];
        input[0] = Some(val);
        Sponge::Absorbing(input)
    }
}

/// A Poseidon duplex sponge.
pub struct Duplex<F: FieldExt, S: Spec<F, T, RATE>, const T: usize, const RATE: usize> {
    sponge: Sponge<F, RATE>,
    state: State<F, T>,
    pad_and_add: Box<dyn Fn(&mut State<F, T>, &SpongeState<F, RATE>)>,
    mds_matrix: Mds<F, T>,
    round_constants: Vec<[F; T]>,
    _marker: PhantomData<S>,
}

impl<F: FieldExt, S: Spec<F, T, RATE>, const T: usize, const RATE: usize> fmt::Debug
    for Duplex<F, S, T, RATE>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Duplex")
            .field("width", &T)
            .field("rate", &RATE)
            .field("R_F", &S::full_rounds())
            .field("R_P", &S::partial_rounds())
            .field("sponge", &self.sponge)
            .field("state", &self.state)
            .field("mds_matrix", &self.mds_matrix)
            .field("round_constants", &self.round_constants)
            .finish()
    }
}

impl<F: FieldExt, S: Spec<F, T, RATE>, const T: usize, const RATE: usize> Duplex<F, S, T, RATE> {
    /// Constructs a new duplex sponge for the given Poseidon specification.
    pub fn new(
        spec: S,
        initial_capacity_element: F,
        pad_and_add: Box<dyn Fn(&mut State<F, T>, &SpongeState<F, RATE>)>,
    ) -> Self {
        let (round_constants, mds_matrix, _) = spec.constants();

        let input = [None; RATE];
        let mut state = [F::zero(); T];
        state[RATE] = initial_capacity_element;

        Duplex {
            sponge: Sponge::Absorbing(input),
            state,
            pad_and_add,
            mds_matrix,
            round_constants,
            _marker: PhantomData::default(),
        }
    }

    /// Absorbs an element into the sponge.
    pub fn absorb(&mut self, value: F) {
        match self.sponge {
            Sponge::Absorbing(ref mut input) => {
                for entry in input.iter_mut() {
                    if entry.is_none() {
                        *entry = Some(value);
                        return;
                    }
                }

                // We've already absorbed as many elements as we can
                let _ = poseidon_duplex::<F, S, T, RATE>(
                    &mut self.state,
                    input,
                    &self.pad_and_add,
                    &self.mds_matrix,
                    &self.round_constants,
                );
                self.sponge = Sponge::absorb(value);
            }
            Sponge::Squeezing(_) => {
                // Drop the remaining output elements
                self.sponge = Sponge::absorb(value);
            }
        }
    }

    /// Squeezes an element from the sponge.
    pub fn squeeze(&mut self) -> F {
        loop {
            match self.sponge {
                Sponge::Absorbing(ref input) => {
                    self.sponge = Sponge::Squeezing(poseidon_duplex::<F, S, T, RATE>(
                        &mut self.state,
                        input,
                        &self.pad_and_add,
                        &self.mds_matrix,
                        &self.round_constants,
                    ));
                }
                Sponge::Squeezing(ref mut output) => {
                    for entry in output.iter_mut() {
                        if let Some(e) = entry.take() {
                            return e;
                        }
                    }

                    // We've already squeezed out all available elements
                    self.sponge = Sponge::Absorbing([None; RATE]);
                }
            }
        }
    }
}

/// A domain in which a Poseidon hash function is being used.
pub trait Domain<F: FieldExt, const T: usize, const RATE: usize>: Copy + fmt::Debug {
    /// The initial capacity element, encoding this domain.
    fn initial_capacity_element(&self) -> F;

    /// The padding that will be added to each state word by [`Domain::pad_and_add`].
    fn padding(&self) -> SpongeState<F, RATE>;

    /// Returns a function that will update the given state with the given input to a
    /// duplex permutation round, applying padding according to this domain specification.
    fn pad_and_add(&self) -> Box<dyn Fn(&mut State<F, T>, &SpongeState<F, RATE>)>;
}

/// A Poseidon hash function used with constant input length.
///
/// Domain specified in section 4.2 of https://eprint.iacr.org/2019/458.pdf
#[derive(Clone, Copy, Debug)]
pub struct ConstantLength<const L: usize>;

impl<F: FieldExt, const T: usize, const RATE: usize, const L: usize> Domain<F, T, RATE>
    for ConstantLength<L>
{
    fn initial_capacity_element(&self) -> F {
        // Capacity value is $length \cdot 2^64 + (o-1)$ where o is the output length.
        // We hard-code an output length of 1.
        F::from_u128((L as u128) << 64)
    }

    fn padding(&self) -> SpongeState<F, RATE> {
        // For constant-input-length hashing, padding consists of the field elements being
        // zero.
        let mut padding = [None; RATE];
        for word in padding.iter_mut().skip(L) {
            *word = Some(F::zero());
        }
        padding
    }

    fn pad_and_add(&self) -> Box<dyn Fn(&mut State<F, T>, &SpongeState<F, RATE>)> {
        Box::new(|state, input| {
            // `Iterator::zip` short-circuits when one iterator completes, so this will only
            // mutate the rate portion of the state.
            for (word, value) in state.iter_mut().zip(input.iter()) {
                // For constant-input-length hashing, padding consists of the field
                // elements being zero, so we don't add anything to the state.
                if let Some(value) = value {
                    *word += value;
                }
            }
        })
    }
}

/// A Poseidon hash function, built around a duplex sponge.
pub struct Hash<
    F: FieldExt,
    S: Spec<F, T, RATE>,
    D: Domain<F, T, RATE>,
    const T: usize,
    const RATE: usize,
> {
    duplex: Duplex<F, S, T, RATE>,
    domain: D,
}

impl<
        F: FieldExt,
        S: Spec<F, T, RATE>,
        D: Domain<F, T, RATE>,
        const T: usize,
        const RATE: usize,
    > fmt::Debug for Hash<F, S, D, T, RATE>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hash")
            .field("width", &T)
            .field("rate", &RATE)
            .field("R_F", &S::full_rounds())
            .field("R_P", &S::partial_rounds())
            .field("domain", &self.domain)
            .finish()
    }
}

impl<
        F: FieldExt,
        S: Spec<F, T, RATE>,
        D: Domain<F, T, RATE>,
        const T: usize,
        const RATE: usize,
    > Hash<F, S, D, T, RATE>
{
    /// Initializes a new hasher.
    pub fn init(spec: S, domain: D) -> Self {
        Hash {
            duplex: Duplex::new(
                spec,
                domain.initial_capacity_element(),
                domain.pad_and_add(),
            ),
            domain,
        }
    }
}

impl<F: FieldExt, S: Spec<F, T, RATE>, const T: usize, const RATE: usize, const L: usize>
    Hash<F, S, ConstantLength<L>, T, RATE>
{
    /// Hashes the given input.
    pub fn hash(mut self, message: [F; L]) -> F {
        for value in array::IntoIter::new(message) {
            self.duplex.absorb(value);
        }
        self.duplex.squeeze()
    }
}

#[cfg(test)]
mod tests {
    use pasta_curves::{arithmetic::FieldExt, pallas};

    use super::{permute, ConstantLength, Hash, P128Pow5T3 as OrchardNullifier, Spec};

    #[test]
    fn orchard_spec_equivalence() {
        let message = [pallas::Base::from_u64(6), pallas::Base::from_u64(42)];

        let (round_constants, mds, _) = OrchardNullifier.constants();

        let hasher = Hash::init(OrchardNullifier, ConstantLength);
        let result = hasher.hash(message);

        // The result should be equivalent to just directly applying the permutation and
        // taking the first state element as the output.
        let mut state = [message[0], message[1], pallas::Base::from_u128(2 << 64)];
        permute::<_, OrchardNullifier, 3, 2>(&mut state, &mds, &round_constants);
        assert_eq!(state[0], result);
    }
}
