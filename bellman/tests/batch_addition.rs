use std::time::Instant;

use pairing::{bn256::{Fq, Bn256}, CurveAffine, ff::Field, Engine, CurveProjective, bls12_381::Bls12};
use rand::{self, Rand};

fn naive<G: CurveAffine>(a: Vec<G>, b: Vec<G>) -> Vec<G::Projective> {
    let mut result = vec![];
    for (a, b) in a.iter().zip(b.iter()) {
        let mut c = a.into_projective();
        c.add_assign_mixed(b);
        result.push(c);
    }
    result
}

#[test]
fn test_batch_addition_bn256() {
    const SAMPLES: usize = 1 << 8;

    let rng = &mut rand::thread_rng();
    let mut a = (0..SAMPLES).map(|_| <Bn256 as Engine>::G1::rand(rng).into_affine()).collect::<Vec<_>>();
    let b = (0..SAMPLES).map(|_| <Bn256 as Engine>::G1::rand(rng).into_affine()).collect::<Vec<_>>();
    let mut scratch_space = vec![a[0].get_x(); SAMPLES];

    let n = Instant::now();
    let naive = naive(a.clone(), b.clone());
    println!("Naive: {}", n.elapsed().as_nanos());

    let ba = Instant::now();
    CurveAffine::batch_addition_assign(&mut a, &b, &mut scratch_space);
    println!("Batch: {}", ba.elapsed().as_nanos());
    
    let naive: Vec<_> = naive.into_iter().map(|p| p.into_affine()).collect();
    assert!(naive == a);
}

#[test]
fn test_batch_addition_bls12() {
    const SAMPLES: usize = 1 << 8;

    let rng = &mut rand::thread_rng();
    let mut a = (0..SAMPLES).map(|_| <Bn256 as Engine>::G1::rand(rng).into_affine()).collect::<Vec<_>>();
    let b = (0..SAMPLES).map(|_| <Bn256 as Engine>::G1::rand(rng).into_affine()).collect::<Vec<_>>();
    let mut scratch_space = vec![a[0].get_x(); SAMPLES];

    let n = Instant::now();
    let naive = naive(a.clone(), b.clone());
    println!("Naive: {}", n.elapsed().as_nanos());

    let ba = Instant::now();
    CurveAffine::batch_addition_assign(&mut a, &b, &mut scratch_space);
    println!("Batch: {}", ba.elapsed().as_nanos());
    
    let naive: Vec<_> = naive.into_iter().map(|p| p.into_affine()).collect();
    assert!(naive == a);
}