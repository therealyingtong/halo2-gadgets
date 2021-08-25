//! Gadgets for elliptic curve operations.

use std::fmt::Debug;

use halo2::{
    arithmetic::CurveAffine,
    circuit::{Chip, Layouter},
    plonk::Error,
};

use utilities::UtilitiesInstructions;

/// Window size for fixed-base scalar multiplication
pub const FIXED_BASE_WINDOW_SIZE: usize = 3;

/// $2^{`FIXED_BASE_WINDOW_SIZE`}$
pub const H: usize = 1 << FIXED_BASE_WINDOW_SIZE;

/// The set of circuit instructions required to use the ECC gadgets.
pub trait EccInstructions<C: CurveAffine>:
    Chip<C::Base> + UtilitiesInstructions<C::Base> + Clone + Debug + Eq
{
    /// Variable representing an element of the elliptic curve's base field, that
    /// is used as a scalar in variable-base scalar mul.
    ///
    /// It is not true in general that a scalar field element fits in a curve's
    /// base field, and in particular it is untrue for the Pallas curve, whose
    /// scalar field `Fq` is larger than its base field `Fp`.
    ///
    /// However, the only use of variable-base scalar mul in the Orchard protocol
    /// is in deriving diversified addresses `[ivk] g_d`,  and `ivk` is guaranteed
    /// to be in the base field of the curve. (See non-normative notes in
    /// https://zips.z.cash/protocol/nu5.pdf#orchardkeycomponents.)
    type ScalarVar: Clone + Debug;
    /// Variable representing a full-width element of the elliptic curve's
    /// scalar field, to be used for fixed-base scalar mul.
    type ScalarFixed: Clone + Debug;
    /// Variable representing a signed short element of the elliptic curve's
    /// scalar field, to be used for fixed-base scalar mul.
    ///
    /// A `ScalarFixedShort` must be in the range [-(2^64 - 1), 2^64 - 1].
    type ScalarFixedShort: Clone + Debug;
    /// Variable representing an elliptic curve point.
    type Point: Clone + Debug;
    /// Variable representing the affine short Weierstrass x-coordinate of an
    /// elliptic curve point.
    type X: Clone + Debug;
    /// Enumeration of the set of fixed bases to be used in scalar mul with a full-width scalar.
    type FixedPoints: FixedPoints<C>;

    /// Constrains point `a` to be equal in value to point `b`.
    fn constrain_equal(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        a: &Self::Point,
        b: &Self::Point,
    ) -> Result<(), Error>;

    /// Witnesses the given point as a private input to the circuit.
    /// This maps the identity to (0, 0) in affine coordinates.
    fn witness_point(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        value: Option<C>,
    ) -> Result<Self::Point, Error>;

    /// Copies a point given existing x- and y-coordinate variables,
    /// checking that the coordinates indeed belong to a valid point.
    /// This maps the identity to (0, 0) in affine coordinates.
    fn copy_point(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        x: Self::Var,
        y: Self::Var,
    ) -> Result<Self::Point, Error>;

    /// Extracts the x-coordinate of a point.
    fn extract_p(point: &Self::Point) -> &Self::X;

    /// Performs incomplete point addition, returning `a + b`.
    ///
    /// This returns an error in exceptional cases.
    fn add_incomplete(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        a: &Self::Point,
        b: &Self::Point,
    ) -> Result<Self::Point, Error>;

    /// Performs complete point addition, returning `a + b`.
    fn add(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        a: &Self::Point,
        b: &Self::Point,
    ) -> Result<Self::Point, Error>;

    /// Performs variable-base scalar multiplication, returning `[scalar] base`.
    /// Multiplication of the identity `[scalar] 𝒪 ` returns an error.
    fn mul(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        scalar: &Self::Var,
        base: &Self::Point,
    ) -> Result<(Self::Point, Self::ScalarVar), Error>;

    /// Performs fixed-base scalar multiplication using a full-width scalar, returning `[scalar] base`.
    fn mul_fixed(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        scalar: Option<C::Scalar>,
        base: &Self::FixedPoints,
    ) -> Result<(Self::Point, Self::ScalarFixed), Error>;

    /// Performs fixed-base scalar multiplication using a short signed scalar, returning
    /// `[magnitude * sign] base`.
    fn mul_fixed_short(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        magnitude_sign: (Self::Var, Self::Var),
        base: &Self::FixedPoints,
    ) -> Result<(Self::Point, Self::ScalarFixedShort), Error>;

    /// Performs fixed-base scalar multiplication using a base field element as the scalar.
    /// In the current implementation, this base field element must be output from another
    /// instruction.
    fn mul_fixed_base_field_elem(
        &self,
        layouter: &mut impl Layouter<C::Base>,
        base_field_elem: Self::Var,
        base: &Self::FixedPoints,
    ) -> Result<Self::Point, Error>;
}

/// Returns information about a fixed point.
pub trait FixedPoints<C: CurveAffine>: Debug + Eq + Clone {
    fn generator(&self) -> C;
    fn u(&self) -> Vec<[[u8; 32]; H]>;
    fn z(&self) -> Vec<u64>;
    fn lagrange_coeffs(&self) -> Vec<[C::Base; H]>;
}

/// An element of the given elliptic curve's base field, that is used as a scalar
/// in variable-base scalar mul.
///
/// It is not true in general that a scalar field element fits in a curve's
/// base field, and in particular it is untrue for the Pallas curve, whose
/// scalar field `Fq` is larger than its base field `Fp`.
///
/// However, the only use of variable-base scalar mul in the Orchard protocol
/// is in deriving diversified addresses `[ivk] g_d`,  and `ivk` is guaranteed
/// to be in the base field of the curve. (See non-normative notes in
/// https://zips.z.cash/protocol/nu5.pdf#orchardkeycomponents.)
#[derive(Debug)]
pub struct ScalarVar<C: CurveAffine, EccChip: EccInstructions<C>> {
    chip: EccChip,
    inner: EccChip::ScalarVar,
}

/// A full-width element of the given elliptic curve's scalar field, to be used for fixed-base scalar mul.
#[derive(Debug)]
pub struct ScalarFixed<C: CurveAffine, EccChip: EccInstructions<C>> {
    chip: EccChip,
    inner: EccChip::ScalarFixed,
}

/// A signed short element of the given elliptic curve's scalar field, to be used for fixed-base scalar mul.
#[derive(Debug)]
pub struct ScalarFixedShort<C: CurveAffine, EccChip: EccInstructions<C>> {
    chip: EccChip,
    inner: EccChip::ScalarFixedShort,
}

/// An elliptic curve point over the given curve.
#[derive(Copy, Clone, Debug)]
pub struct Point<C: CurveAffine, EccChip: EccInstructions<C>> {
    chip: EccChip,
    inner: EccChip::Point,
}

impl<C: CurveAffine, EccChip: EccInstructions<C>> Point<C, EccChip> {
    /// Constructs a new point with the given value.
    pub fn new(
        chip: EccChip,
        mut layouter: impl Layouter<C::Base>,
        value: Option<C>,
    ) -> Result<Self, Error> {
        let point = chip.witness_point(&mut layouter, value);
        point.map(|inner| Point { chip, inner })
    }

    /// Constructs a new point by copying in its coordinates as `x`, `y` cells.
    pub fn copy(
        chip: EccChip,
        mut layouter: impl Layouter<C::Base>,
        x: EccChip::Var,
        y: EccChip::Var,
    ) -> Result<Self, Error> {
        let point = chip.copy_point(&mut layouter, x, y);
        point.map(|inner| Point { chip, inner })
    }

    /// Constrains this point to be equal in value to another point.
    pub fn constrain_equal(
        &self,
        mut layouter: impl Layouter<C::Base>,
        other: &Self,
    ) -> Result<(), Error> {
        self.chip
            .constrain_equal(&mut layouter, &self.inner, &other.inner)
    }

    /// Returns the inner point.
    pub fn inner(&self) -> &EccChip::Point {
        &self.inner
    }

    /// Extracts the x-coordinate of a point.
    pub fn extract_p(&self) -> X<C, EccChip> {
        X::from_inner(self.chip.clone(), EccChip::extract_p(&self.inner).clone())
    }

    /// Wraps the given point (obtained directly from an instruction) in a gadget.
    pub fn from_inner(chip: EccChip, inner: EccChip::Point) -> Self {
        Point { chip, inner }
    }

    /// Returns `self + other` using complete addition.
    pub fn add(&self, mut layouter: impl Layouter<C::Base>, other: &Self) -> Result<Self, Error> {
        assert_eq!(self.chip, other.chip);
        self.chip
            .add(&mut layouter, &self.inner, &other.inner)
            .map(|inner| Point {
                chip: self.chip.clone(),
                inner,
            })
    }

    /// Returns `self + other` using incomplete addition.
    pub fn add_incomplete(
        &self,
        mut layouter: impl Layouter<C::Base>,
        other: &Self,
    ) -> Result<Self, Error> {
        assert_eq!(self.chip, other.chip);
        self.chip
            .add_incomplete(&mut layouter, &self.inner, &other.inner)
            .map(|inner| Point {
                chip: self.chip.clone(),
                inner,
            })
    }

    /// Returns `[by] self`.
    pub fn mul(
        &self,
        mut layouter: impl Layouter<C::Base>,
        by: &EccChip::Var,
    ) -> Result<(Self, ScalarVar<C, EccChip>), Error> {
        self.chip
            .mul(&mut layouter, by, &self.inner)
            .map(|(point, scalar)| {
                (
                    Point {
                        chip: self.chip.clone(),
                        inner: point,
                    },
                    ScalarVar {
                        chip: self.chip.clone(),
                        inner: scalar,
                    },
                )
            })
    }
}

/// The affine short Weierstrass x-coordinate of an elliptic curve point over the
/// given curve.
#[derive(Debug)]
pub struct X<C: CurveAffine, EccChip: EccInstructions<C>> {
    chip: EccChip,
    inner: EccChip::X,
}

impl<C: CurveAffine, EccChip: EccInstructions<C>> X<C, EccChip> {
    /// Wraps the given x-coordinate (obtained directly from an instruction) in a gadget.
    pub fn from_inner(chip: EccChip, inner: EccChip::X) -> Self {
        X { chip, inner }
    }

    /// Returns the inner x-coordinate.
    pub fn inner(&self) -> &EccChip::X {
        &self.inner
    }
}

/// A constant elliptic curve point over the given curve, for which window tables have
/// been provided to make scalar multiplication more efficient.
///
/// Used in scalar multiplication with full-width scalars.
#[derive(Clone, Debug)]
pub struct FixedPoint<C: CurveAffine, EccChip: EccInstructions<C>> {
    chip: EccChip,
    /// UNDO THIS pub.
    pub inner: EccChip::FixedPoints,
}

impl<C: CurveAffine, EccChip: EccInstructions<C>> FixedPoint<C, EccChip> {
    #[allow(clippy::type_complexity)]
    /// Returns `[by] self`.
    pub fn mul(
        &self,
        mut layouter: impl Layouter<C::Base>,
        by: Option<C::Scalar>,
    ) -> Result<(Point<C, EccChip>, ScalarFixed<C, EccChip>), Error> {
        self.chip
            .mul_fixed(&mut layouter, by, &self.inner)
            .map(|(point, scalar)| {
                (
                    Point {
                        chip: self.chip.clone(),
                        inner: point,
                    },
                    ScalarFixed {
                        chip: self.chip.clone(),
                        inner: scalar,
                    },
                )
            })
    }

    #[allow(clippy::type_complexity)]
    /// Returns `[by] self`.
    pub fn mul_base_field(
        &self,
        mut layouter: impl Layouter<C::Base>,
        by: EccChip::Var,
    ) -> Result<Point<C, EccChip>, Error> {
        self.chip
            .mul_fixed_base_field_elem(&mut layouter, by, &self.inner)
            .map(|inner| Point {
                chip: self.chip.clone(),
                inner,
            })
    }

    #[allow(clippy::type_complexity)]
    /// Returns `[by] self`.
    pub fn mul_short(
        &self,
        mut layouter: impl Layouter<C::Base>,
        magnitude_sign: (EccChip::Var, EccChip::Var),
    ) -> Result<(Point<C, EccChip>, ScalarFixedShort<C, EccChip>), Error> {
        self.chip
            .mul_fixed_short(&mut layouter, magnitude_sign, &self.inner)
            .map(|(point, scalar)| {
                (
                    Point {
                        chip: self.chip.clone(),
                        inner: point,
                    },
                    ScalarFixedShort {
                        chip: self.chip.clone(),
                        inner: scalar,
                    },
                )
            })
    }

    /// Wraps the given fixed base (obtained directly from an instruction) in a gadget.
    pub fn from_inner(chip: EccChip, inner: EccChip::FixedPoints) -> Self {
        FixedPoint { chip, inner }
    }
}

#[cfg(feature = "testing")]
pub mod testing {
    use crate::{
        chip::{EccChip, EccConfig},
        gadget::FixedPoints,
    };
    use utilities::lookup_range_check::LookupRangeCheckConfig;

    use halo2::{
        circuit::{Layouter, SimpleFloorPlanner},
        plonk::{Circuit, ConstraintSystem, Error},
    };
    use pasta_curves::pallas;

    use std::marker::PhantomData;

    pub struct MyCircuit<S: EccTest<F>, F: FixedPoints<pallas::Affine>>(pub PhantomData<(S, F)>);

    #[allow(non_snake_case)]
    impl<S: EccTest<F>, F: FixedPoints<pallas::Affine>> Circuit<pallas::Base> for MyCircuit<S, F> {
        type Config = EccConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            MyCircuit(PhantomData)
        }

        fn configure(meta: &mut ConstraintSystem<pallas::Base>) -> Self::Config {
            let advices = [
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
                meta.advice_column(),
            ];
            let lookup_table = meta.lookup_table_column();
            let lagrange_coeffs = [
                meta.fixed_column(),
                meta.fixed_column(),
                meta.fixed_column(),
                meta.fixed_column(),
                meta.fixed_column(),
                meta.fixed_column(),
                meta.fixed_column(),
                meta.fixed_column(),
            ];
            // Shared fixed column for loading constants
            let constants = meta.fixed_column();
            meta.enable_constant(constants);

            let range_check = LookupRangeCheckConfig::configure(meta, advices[9], lookup_table);
            EccChip::<F>::configure(meta, advices, lagrange_coeffs, range_check)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<pallas::Base>,
        ) -> Result<(), Error> {
            let chip = EccChip::construct(config.clone());

            // Load 10-bit lookup table. In the Action circuit, this will be
            // provided by the Sinsemilla chip.
            config.lookup_config.load(&mut layouter)?;

            S::test_add(chip.clone(), layouter.namespace(|| "addition"))?;
            S::test_add_incomplete(chip.clone(), layouter.namespace(|| "incomplete addition"))?;
            S::test_mul(
                chip.clone(),
                layouter.namespace(|| "variable-base scalar multiplication"),
            )?;
            S::test_mul_fixed(
                chip.clone(),
                layouter.namespace(|| "fixed-base scalar multiplication with full-width scalar"),
            )?;
            S::test_mul_fixed_short(
                chip.clone(),
                layouter.namespace(|| "fixed-base scalar multiplication with short signed scalar"),
            )?;
            S::test_mul_fixed_base_field(
                chip,
                layouter.namespace(|| "fixed-base scalar multiplication with base field element"),
            )?;

            Ok(())
        }
    }

    pub trait EccTest<F: FixedPoints<pallas::Affine>> {
        fn fixed_bases_full() -> Vec<F>;
        fn fixed_bases_short() -> Vec<F>;
        fn fixed_bases_base_field() -> Vec<F>;

        fn test_add(chip: EccChip<F>, layouter: impl Layouter<pallas::Base>) -> Result<(), Error> {
            crate::chip::add::tests::test_add(chip, layouter)
        }

        fn test_add_incomplete(
            chip: EccChip<F>,
            layouter: impl Layouter<pallas::Base>,
        ) -> Result<(), Error> {
            crate::chip::add_incomplete::tests::test_add_incomplete(chip, layouter)
        }

        fn test_mul(chip: EccChip<F>, layouter: impl Layouter<pallas::Base>) -> Result<(), Error> {
            crate::chip::mul::tests::test_mul(chip, layouter)
        }

        fn test_mul_fixed(
            chip: EccChip<F>,
            mut layouter: impl Layouter<pallas::Base>,
        ) -> Result<(), Error> {
            for base in Self::fixed_bases_full().into_iter() {
                crate::chip::mul_fixed::full_width::tests::test_mul_fixed(
                    base,
                    chip.clone(),
                    layouter.namespace(|| "full-width fixed-base scalar mul"),
                )?;
            }

            Ok(())
        }

        fn test_mul_fixed_short(
            chip: EccChip<F>,
            mut layouter: impl Layouter<pallas::Base>,
        ) -> Result<(), Error> {
            for base in Self::fixed_bases_short().into_iter() {
                crate::chip::mul_fixed::short::tests::test_mul_fixed_short(
                    base,
                    chip.clone(),
                    layouter.namespace(|| "full-width fixed-base scalar mul"),
                )?;
            }

            Ok(())
        }

        fn test_mul_fixed_base_field(
            chip: EccChip<F>,
            mut layouter: impl Layouter<pallas::Base>,
        ) -> Result<(), Error> {
            for base in Self::fixed_bases_base_field().into_iter() {
                crate::chip::mul_fixed::base_field_elem::tests::test_mul_fixed_base_field(
                    base,
                    chip.clone(),
                    layouter.namespace(|| "full-width fixed-base scalar mul"),
                )?;
            }

            Ok(())
        }
    }
}

#[cfg(feature = "testing")]
mod tests {
    use group::{Curve, Group};

    use pasta_curves::pallas;

    use crate::{
        chip::{compute_lagrange_coeffs, find_zs_and_us, NUM_WINDOWS, NUM_WINDOWS_SHORT},
        gadget::{FixedPoints, H},
    };
    use lazy_static::lazy_static;

    #[derive(Debug, Eq, PartialEq, Clone)]
    enum FixedBase {
        FullWidth,
        Short,
    }

    lazy_static! {
        static ref BASE: pallas::Affine = pallas::Point::generator().to_affine();
        static ref ZS_AND_US: Vec<(u64, [[u8; 32]; H])> =
            find_zs_and_us(*BASE, NUM_WINDOWS).unwrap();
        static ref ZS_AND_US_SHORT: Vec<(u64, [[u8; 32]; H])> =
            find_zs_and_us(*BASE, NUM_WINDOWS_SHORT).unwrap();
        static ref LAGRANGE_COEFFS: Vec<[pallas::Base; H]> =
            compute_lagrange_coeffs(*BASE, NUM_WINDOWS);
        static ref LAGRANGE_COEFFS_SHORT: Vec<[pallas::Base; H]> =
            compute_lagrange_coeffs(*BASE, NUM_WINDOWS_SHORT);
    }

    impl FixedPoints<pallas::Affine> for FixedBase {
        fn generator(&self) -> pallas::Affine {
            *BASE
        }

        fn u(&self) -> Vec<[[u8; 32]; H]> {
            match self {
                FixedBase::FullWidth => ZS_AND_US.iter().map(|(_, us)| *us).collect(),
                FixedBase::Short => ZS_AND_US_SHORT.iter().map(|(_, us)| *us).collect(),
            }
        }

        fn z(&self) -> Vec<u64> {
            match self {
                FixedBase::FullWidth => ZS_AND_US.iter().map(|(z, _)| *z).collect(),
                FixedBase::Short => ZS_AND_US_SHORT.iter().map(|(z, _)| *z).collect(),
            }
        }

        fn lagrange_coeffs(&self) -> Vec<[pallas::Base; H]> {
            match self {
                FixedBase::FullWidth => LAGRANGE_COEFFS.to_vec(),
                FixedBase::Short => LAGRANGE_COEFFS_SHORT.to_vec(),
            }
        }
    }

    struct Test;
    impl super::testing::EccTest<FixedBase> for Test {
        fn fixed_bases_full() -> Vec<FixedBase> {
            vec![FixedBase::FullWidth]
        }
        fn fixed_bases_short() -> Vec<FixedBase> {
            vec![FixedBase::Short]
        }
        fn fixed_bases_base_field() -> Vec<FixedBase> {
            vec![FixedBase::FullWidth]
        }
    }

    #[test]
    fn ecc_chip() {
        use halo2::dev::MockProver;

        let k = 13;
        let circuit = super::testing::MyCircuit::<Test, FixedBase>(std::marker::PhantomData);
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()))
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_ecc_chip() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("ecc-chip-layout.png", (1024, 7680)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Ecc Chip Layout", ("sans-serif", 60)).unwrap();

        let circuit = super::testing::MyCircuit::<Test, FixedBase>(std::marker::PhantomData);
        halo2::dev::CircuitLayout::default()
            .render(13, &circuit, &root)
            .unwrap();
    }
}
