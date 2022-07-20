extern crate exitcode;
extern crate fawkes_crypto;
extern crate fawkes_crypto_phase2;
extern crate libzeropool;
extern crate rand;
extern crate bellman_ce;

use libzeropool::{
    circuit::tree::{tree_update, CTreePub, CTreeSec},
    circuit::tx::{c_transfer, CTransferPub, CTransferSec},
    POOL_PARAMS,
};
use std::fs::File;
use bellman_ce::pairing::CurveAffine;

use fawkes_crypto::{
    backend::bellman_groth16::{
        engines::{Bn256, Engine},
        BellmanCS,
    },
    circuit::cs::BuildCS,
    core::signal::Signal,
};
use fawkes_crypto_phase2::parameters::MPCParameters;
use fawkes_crypto::backend::bellman_groth16::Parameters;
use fawkes_crypto::circuit::cs::CS;
use fawkes_crypto::engines::bn256::Fr;

fn tx_circuit<C:CS<Fr=Fr>>(public: CTransferPub<C>, secret: CTransferSec<C>) {
    c_transfer(&public, &secret, &*POOL_PARAMS);
}

/*
pub fn prepare_parameters<E: Engine, Pub: Signal<BuildCS<E::Fr>>, Sec: Signal<BuildCS<E::Fr>>, C: Fn(Pub, Sec)>(
    circuit: C,
) -> Parameters<E> {
    let ref rcs = BuildCS::rc_new();
    let signal_pub = Pub::alloc(rcs, None);
    signal_pub.inputize();
    let signal_sec = Sec::alloc(rcs, None);

    circuit(signal_pub, signal_sec);

    let bcs = BellmanCS::<E, BuildCS<E::Fr>>::new(rcs.clone());

    let ref mut rng = OsRng::new();
    let bp = bellman::groth16::generate_random_parameters(bcs, rng).unwrap();
    let cs=rcs.borrow();

    let num_gates = cs.gates.len();

    let mut buf = std::io::Cursor::new(vec![]);
    let mut c = brotli::CompressorWriter::new(&mut buf, 4096, 9, 22);
    for g in cs.gates.iter() {
        c.write_all(&g.try_to_vec().unwrap()).unwrap();
    }

    c.flush().unwrap();
    drop(c);

    Parameters(bp, num_gates as u32, buf.into_inner(), cs.const_tracker.clone())
}

 */

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: \n<transfer/tree_update> <out_params.params> <path/to/phase1radix>");
        std::process::exit(exitcode::USAGE);
    }
    let circuit_name = &args[1];
    let params_filename = &args[2];
    let radix_directory = &args[3];

    let ref rcs = BuildCS::rc_new();

    match circuit_name.as_str() {
        "tree_update" => {
            let signal_pub = CTreePub::alloc(rcs, None);
            signal_pub.inputize();
            let signal_sec = CTreeSec::alloc(rcs, None);

            tree_update(&signal_pub, &signal_sec, &*POOL_PARAMS);
        }
        "transfer" => {
            let signal_pub = CTransferPub::alloc(rcs, None);
            signal_pub.inputize();
            let signal_sec = CTransferSec::alloc(rcs, None);

            tx_circuit(signal_pub, signal_sec);
        }
        _ => panic!("Wrong cicruit parameter"),
    };

    let bcs = BellmanCS::<Bn256, BuildCS<<Bn256 as Engine>::Fr>>::new(rcs.clone());

    let should_filter_points_at_infinity = true;

    // Import the circuit and create the initial parameters using phase 1
    println!("Creating initial parameters for {}...", circuit_name);
    let params =
        MPCParameters::new(bcs, should_filter_points_at_infinity, radix_directory).unwrap();

    println!("Writing MPC parameters to {}.", params_filename);
    let mut f = File::create(params_filename).unwrap();
    params.write(&mut f).expect("unable to write params");
}
