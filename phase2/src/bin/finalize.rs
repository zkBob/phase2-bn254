extern crate bellman_ce;
extern crate exitcode;
extern crate fawkes_crypto;
extern crate fawkes_crypto_phase2;
extern crate libzeropool;
extern crate rand;

use libzeropool::{
    circuit::tree::{tree_update, CTreePub, CTreeSec},
    circuit::tx::{c_transfer, CTransferPub, CTransferSec},
    POOL_PARAMS,
};
use std::fs::File;

use fawkes_crypto::circuit::cs::CS;
use fawkes_crypto::engines::bn256::Fr;
use fawkes_crypto::{
    backend::bellman_groth16::engines::Bn256,
    circuit::cs::BuildCS,
    core::signal::Signal,
};
use fawkes_crypto_phase2::parameters::MPCParameters;

fn tx_circuit<C: CS<Fr = Fr>>(public: CTransferPub<C>, secret: CTransferSec<C>) {
    c_transfer(&public, &secret, &*POOL_PARAMS);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: \n<transfer/tree_update> <mpc_params> <out_fawkes_params>");
        std::process::exit(exitcode::USAGE);
    }
    let circuit_name = &args[1];
    let mpc_params_filename = &args[2];
    let params_filename = &args[3];

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

    let should_filter_points_at_infinity = true;

    println!("Creating fawkes compatible parameters for {}...", circuit_name);    

    let mpc_params: MPCParameters = MPCParameters::read(
        std::fs::File::open(mpc_params_filename).unwrap(),
        should_filter_points_at_infinity,
        true,
    )
    .unwrap();
    let params = mpc_params.get_params().clone();

    let mut result_params =
        fawkes_crypto::backend::bellman_groth16::setup::setup::<Bn256, _, _, _>(tx_circuit);
    result_params.0 = params;

    println!("Writing initial parameters to {}.", params_filename);
    let mut f = File::create(params_filename).unwrap();
    result_params.write(&mut f).expect("unable to write params");
}
