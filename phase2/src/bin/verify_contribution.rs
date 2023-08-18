extern crate exitcode;
extern crate fawkes_crypto_phase2;
extern crate libzeropool;

use std::fs::OpenOptions;

use fawkes_crypto_phase2::parameters::*;
use libzeropool::{
    circuit::{
        delegated_deposit::{
            check_delegated_deposit_batch, CDelegatedDepositBatchPub, CDelegatedDepositBatchSec,
        },
        tree::{tree_update, CTreePub, CTreeSec},
        tx::{c_transfer, CTransferPub, CTransferSec},
    },
    fawkes_crypto::{
        backend::bellman_groth16::{
            engines::{Bn256, Engine},
            BellmanCS,
        },
        circuit::cs::{BuildCS, CS},
        core::signal::Signal,
        engines::bn256::Fr,
    },
    POOL_PARAMS,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        println!("Usage: \n<transfer/tree_update/delegated_deposit> <in_old_params.params> <in_new_params.params> <path/to/phase1radix>");
        std::process::exit(exitcode::USAGE);
    }
    let circuit_name = &args[1];
    let old_params_filename = &args[2];
    let new_params_filename = &args[3];
    let radix_directory = &args[4];

    let disallow_points_at_infinity = false;

    let old_reader = OpenOptions::new()
        .read(true)
        .open(old_params_filename)
        .expect("unable to open old params");
    let old_params = MPCParameters::read(old_reader, disallow_points_at_infinity, true)
        .expect("unable to read old params");

    let new_reader = OpenOptions::new()
        .read(true)
        .open(new_params_filename)
        .expect("unable to open new params");
    let new_params = MPCParameters::read(new_reader, disallow_points_at_infinity, true)
        .expect("unable to read new params");

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
        "delegated_deposit" => {
            let signal_pub = CDelegatedDepositBatchPub::alloc(rcs, None);
            signal_pub.inputize();
            let signal_sec = CDelegatedDepositBatchSec::alloc(rcs, None);

            delegated_deposit(signal_pub, signal_sec);
        }
        _ => panic!("Wrong cicruit parameter"),
    };

    let bcs = BellmanCS::<Bn256, BuildCS<<Bn256 as Engine>::Fr>>::new(rcs.clone());

    println!("Checking contribution {}...", new_params_filename);
    let contribution = verify_contribution(&old_params, &new_params).expect("should verify");

    let should_filter_points_at_infinity = true;
    let verification_result = new_params
        .verify(bcs, should_filter_points_at_infinity, radix_directory)
        .unwrap();
    assert!(contains_contribution(&verification_result, &contribution));
    println!("Contribution {} verified.", new_params_filename);
}

fn tx_circuit<C: CS<Fr = Fr>>(public: CTransferPub<C>, secret: CTransferSec<C>) {
    c_transfer(&public, &secret, &*POOL_PARAMS);
}

fn delegated_deposit<C: CS<Fr = Fr>>(
    public: CDelegatedDepositBatchPub<C>,
    secret: CDelegatedDepositBatchSec<C>,
) {
    check_delegated_deposit_batch(&public, &secret, &*POOL_PARAMS);
}
