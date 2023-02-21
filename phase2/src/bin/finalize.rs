extern crate exitcode;
extern crate fawkes_crypto_phase2;
extern crate libzeropool;
extern crate rand;

use fawkes_crypto_phase2::parameters::MPCParameters;
use libzeropool::{
    circuit::tree::{tree_update, CTreePub, CTreeSec},
    circuit::{
        delegated_deposit::{
            check_delegated_deposit_batch, CDelegatedDepositBatchPub, CDelegatedDepositBatchSec,
        },
        tx::{c_transfer, CTransferPub, CTransferSec},
    },
    fawkes_crypto::{
        backend::bellman_groth16::Parameters,
        backend::bellman_groth16::{engines::Bn256, setup::setup},
        circuit::cs::BuildCS,
        circuit::cs::CS,
        core::signal::Signal,
        engines::bn256::Fr,
    },
    POOL_PARAMS,
};
use std::fs::File;

fn tx_circuit<C: CS<Fr = Fr>>(public: CTransferPub<C>, secret: CTransferSec<C>) {
    c_transfer(&public, &secret, &*POOL_PARAMS);
}

fn tree_update_circuit<C: CS<Fr = Fr>>(public: CTreePub<C>, secret: CTreeSec<C>) {
    tree_update(&public, &secret, &*POOL_PARAMS);
}
fn delegated_deposit<C: CS<Fr = Fr>>(
    public: CDelegatedDepositBatchPub<C>,
    secret: CDelegatedDepositBatchSec<C>,
) {
    check_delegated_deposit_batch(&public, &secret, &*POOL_PARAMS);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: \n<transfer/tree_update/delegated_deposit> <mpc_params> <out_fawkes_params>");
        std::process::exit(exitcode::USAGE);
    }
    let circuit_name = &args[1];
    let mpc_params_filename = &args[2];
    let params_filename = &args[3];

    let ref rcs = BuildCS::rc_new();

    let mut result_params: Parameters<Bn256> = match circuit_name.as_str() {
        "tree_update" => {
            let signal_pub = CTreePub::alloc(rcs, None);
            signal_pub.inputize();
            let signal_sec = CTreeSec::alloc(rcs, None);

            tree_update_circuit(signal_pub, signal_sec);
            setup(tree_update_circuit)
        }
        "transfer" => {
            let signal_pub = CTransferPub::alloc(rcs, None);
            signal_pub.inputize();
            let signal_sec = CTransferSec::alloc(rcs, None);

            tx_circuit(signal_pub, signal_sec);
            setup(tx_circuit)
        }
        "delegated_deposit" => {
            let signal_pub = CDelegatedDepositBatchPub::alloc(rcs, None);
            signal_pub.inputize();
            let signal_sec = CDelegatedDepositBatchSec::alloc(rcs, None);

            delegated_deposit(signal_pub, signal_sec);
            setup(delegated_deposit)
        }
        _ => panic!("Wrong cicruit parameter"),
    };

    let should_filter_points_at_infinity = true;

    println!(
        "Creating fawkes compatible parameters for {}...",
        circuit_name
    );

    let mpc_params: MPCParameters = MPCParameters::read(
        std::fs::File::open(mpc_params_filename).unwrap(),
        should_filter_points_at_infinity,
        true,
    )
    .unwrap();
    let params = mpc_params.get_params().clone();

    result_params.0 = params;

    println!("Writing initial parameters to {}.", params_filename);
    let mut f = File::create(params_filename).unwrap();
    result_params.write(&mut f).expect("unable to write params");
}
