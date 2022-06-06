use crate::pairing::{
    CurveAffine,
    CurveProjective,
    Engine
};

use crate::pairing::ff::{
    PrimeField,
    Field,
    PrimeFieldRepr,
    ScalarEngine};

use crate::multicore::Waiter;

use std::sync::Arc;
use super::source::*;
use super::worker::Worker;

use super::SynthesisError;

#[cfg(feature = "multicore")]
use rayon::prelude::*;

#[cfg(not(feature = "multicore"))]
use crate::multicore::FakeParallelIterator;

fn multiexp_inner<Q, D, G, S>(
    bases: S,
    density_map: D,
    exponents: Arc<Vec<<G::Scalar as PrimeField>::Repr>>,
    c: u32,
) -> Result<<G as CurveAffine>::Projective, SynthesisError>
    where for<'a> &'a Q: QueryDensity,
          D: Send + Sync + 'static + Clone + AsRef<Q>,
          G: CurveAffine,
          S: SourceBuilder<G>,
{
    // Perform this region of the multiexp
    let this = move |
        bases: S,
        density_map: D,
        exponents: Arc<Vec<<G::Scalar as PrimeField>::Repr>>,
        chunk: usize
    | -> Result<_, SynthesisError> {
        // Accumulate the result
        let mut acc = G::Projective::zero();

        // Build a source for the bases
        let mut bases = bases.new();

        // Create space for the buckets
        let mut buckets = vec![G::Projective::zero(); (1 << c) - 1];

        // only the first round uses this
        let handle_trivial = chunk == 0;

        //let zero = <G::Engine as ScalarEngine>::Fr::zero().into_repr();
        let one = <G::Engine as ScalarEngine>::Fr::one().into_repr();

        // Sort the bases into buckets
        for (&exp, density) in exponents.iter().zip(density_map.as_ref().iter()) {
            if density {
                if exp.is_zero() {
                    bases.skip(1)?;
                } else if exp == one {
                    if handle_trivial {
                        bases.add_assign_mixed(&mut acc)?;
                    } else {
                        bases.skip(1)?;
                    }
                } else {
                    // Place multiplication into the bucket: Separate s * P as
                    // (s/2^c) * P + (s mod 2^c) P
                    // First multiplication is c bits less, so one can do it,
                    // sum results from different buckets and double it c times,
                    // then add with (s mod 2^c) P parts
                    let mut exp = exp;
                    let skip = (chunk as u32) * c;
                    exp.shr(skip);
                    let exp = exp.as_ref()[0] % (1 << c);

                    if exp != 0 {
                        bases.add_assign_mixed(&mut buckets[(exp - 1) as usize])?;
                    } else {
                        bases.skip(1)?;
                    }
                }
            }
        }

        // Summation by parts
        // e.g. 3a + 2b + 1c = a +
        //                    (a) + b +
        //                    ((a) + b) + c
        let mut running_sum = G::Projective::zero();
        for exp in buckets.into_iter().rev() {
            running_sum.add_assign(&exp);
            acc.add_assign(&running_sum);
        }

        Ok(acc)
    };

    // compute c-bit width sums in parallel
    let parts = (0..G::Scalar::NUM_BITS)
        .into_par_iter()
        .step_by(c as usize)
        .enumerate()
        .map(|(chunk_idx, _)| this(bases.clone(), density_map.clone(), exponents.clone(), chunk_idx))
        .collect::<Vec<Result<_, _>>>();

    // final summation
    parts
        .into_iter()
        .rev()
        .try_fold(G::Projective::zero(), |acc, part| {
            part.map(|part| {
                let mut partial_sum = (0..c).fold(acc, |mut acc, _| {
                    acc.double();
                    acc
                });
                partial_sum.add_assign(&part);
                partial_sum
            })
        })
}

/// Perform multi-exponentiation. The caller is responsible for ensuring the
/// query size is the same as the number of exponents.
pub fn multiexp<Q, D, G, S>(
    pool: &Worker,
    bases: S,
    density_map: D,
    exponents: Arc<Vec<<<G::Engine as ScalarEngine>::Fr as PrimeField>::Repr>>
) -> Waiter<Result<<G as CurveAffine>::Projective, SynthesisError>>
    where
            for<'a> &'a Q: QueryDensity,
            D: Send + Sync + 'static + Clone + AsRef<Q>,
            G: CurveAffine,
            S: SourceBuilder<G>,
{
    let c = if exponents.len() < 32 {
        3u32
    } else {
        (f64::from(exponents.len() as u32)).ln().ceil() as u32
    };

    if let Some(query_size) = density_map.as_ref().get_query_size() {
        // If the density map has a known query size, it should not be
        // inconsistent with the number of exponents.

        assert!(query_size == exponents.len());
    }

    pool.compute(move || multiexp_inner(bases, density_map, exponents, c))
}

#[allow(dead_code)] // it's used inside tests
fn naive_multiexp<G: CurveAffine>(
    bases: Arc<Vec<G>>,
    exponents: Arc<Vec<<G::Scalar as PrimeField>::Repr>>
) -> G::Projective
{
    assert_eq!(bases.len(), exponents.len());

    let mut acc = G::Projective::zero();

    for (base, exp) in bases.iter().zip(exponents.iter()) {
        acc.add_assign(&base.mul(*exp));
    }

    acc
}

#[test]
fn test_with_bls12() {
    use rand::{self, Rand};
    use crate::pairing::bls12_381::Bls12;

    const SAMPLES: usize = 1 << 14;

    let rng = &mut rand::thread_rng();
    let v = Arc::new((0..SAMPLES).map(|_| <Bls12 as ScalarEngine>::Fr::rand(rng).into_repr()).collect::<Vec<_>>());
    let g = Arc::new((0..SAMPLES).map(|_| <Bls12 as Engine>::G1::rand(rng).into_affine()).collect::<Vec<_>>());

    let naive = naive_multiexp(g.clone(), v.clone());

    let pool = Worker::new();

    let fast = multiexp(
        &pool,
        (g, 0),
        FullDensity,
        v
    ).wait().unwrap();

    assert_eq!(naive, fast);
}

#[test]
fn test_speed_with_bn256() {
    use rand::{self, Rand};
    use crate::pairing::bn256::Bn256;
    use num_cpus;

    let cpus = num_cpus::get();
    const SAMPLES: usize = 1 << 22;

    let rng = &mut rand::thread_rng();
    let v = Arc::new((0..SAMPLES).map(|_| <Bn256 as ScalarEngine>::Fr::rand(rng).into_repr()).collect::<Vec<_>>());
    let g = Arc::new((0..SAMPLES).map(|_| <Bn256 as Engine>::G1::rand(rng).into_affine()).collect::<Vec<_>>());

    let pool = Worker::new();

    let start = std::time::Instant::now();

    let _fast = multiexp(
        &pool,
        (g, 0),
        FullDensity,
        v
    ).wait().unwrap();


    let duration_ns = start.elapsed().as_nanos() as f64;
    println!("Elapsed {} ns for {} samples", duration_ns, SAMPLES);
    let time_per_sample = duration_ns/(SAMPLES as f64);
    println!("Tested on {} samples on {} CPUs with {} ns per multiplication", SAMPLES, cpus, time_per_sample);
}

#[test]
fn test_bench_sparse_multiexp() {
    use rand::{XorShiftRng, SeedableRng, Rand, Rng};
    use crate::pairing::bn256::Bn256;
    use num_cpus;

    const SAMPLES: usize = 1 << 22;
    let rng = &mut XorShiftRng::from_seed([0x3dbe6259, 0x8d313d76, 0x3237db17, 0xe5bc0654]);

    let v = (0..SAMPLES).map(|_| <Bn256 as ScalarEngine>::Fr::rand(rng).into_repr()).collect::<Vec<_>>();
    let g = (0..SAMPLES).map(|_| <Bn256 as Engine>::G1::rand(rng).into_affine()).collect::<Vec<_>>();

    println!("Done generating test points and scalars");

    let pool = Worker::new();
    let start = std::time::Instant::now();

    let _sparse = multiexp(
        &pool,
        (Arc::new(g), 0),
        FullDensity,
        Arc::new(v)
    ).wait().unwrap();

    let duration_ns = start.elapsed().as_nanos() as f64;
    println!("{} ms for sparse for {} samples", duration_ns/1000.0f64, SAMPLES);
}

#[test]
fn test_with_ones() {
    use rand::{self, Rand};
    use crate::pairing::bls12_381::Bls12;

    let rng = &mut rand::thread_rng();
    let v = Arc::new(vec![
        <Bls12 as ScalarEngine>::Fr::rand(rng).into_repr(),
        <Bls12 as ScalarEngine>::Fr::rand(rng).into_repr(),
        <Bls12 as ScalarEngine>::Fr::rand(rng).into_repr(),
        <Bls12 as ScalarEngine>::Fr::rand(rng).into_repr(),
        <Bls12 as ScalarEngine>::Fr::rand(rng).into_repr(),
    ]);
    let g = Arc::new(vec![
        <Bls12 as Engine>::G1::one().into_affine(),
        <Bls12 as Engine>::G1::one().into_affine(),
        <Bls12 as Engine>::G1::one().into_affine(),
        <Bls12 as Engine>::G1::one().into_affine(),
        <Bls12 as Engine>::G1::rand(rng).into_affine(),
    ]);
    let naive = naive_multiexp(g.clone(), v.clone());

    let pool = Worker::new();
    let fast = multiexp(
        &pool,
        (g, 0),
        FullDensity,
        v
    ).wait().unwrap();

    assert_eq!(naive, fast);
}
