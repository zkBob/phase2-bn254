extern crate exitcode;
extern crate fawkes_crypto_phase2;
extern crate libzeropool;
extern crate rand;

use libzeropool::{
    circuit::tree::{tree_update, CTreePub, CTreeSec},
    circuit::{tx::{c_transfer, CTransferPub, CTransferSec}, delegated_deposit::{CDelegatedDepositBatchPub, CDelegatedDepositBatchSec, check_delegated_deposit_batch}},
    POOL_PARAMS,
    fawkes_crypto::{
        engines::bn256::Fr,
        backend::bellman_groth16::{
            engines::{Bn256, Engine},
            BellmanCS
        },
        circuit::cs::{CS,BuildCS},
        core::signal::Signal,
        
    },
    
};
use std::fs::File;
use fawkes_crypto_phase2::parameters::MPCParameters;

fn tx_circuit<C:CS<Fr=Fr>>(public: CTransferPub<C>, secret: CTransferSec<C>) {
    c_transfer(&public, &secret, &*POOL_PARAMS);
}
fn delegated_deposit<C:CS<Fr=Fr>>(public: CDelegatedDepositBatchPub<C>, secret: CDelegatedDepositBatchSec<C>) {
    check_delegated_deposit_batch(&public, &secret, &*POOL_PARAMS);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!("Usage: \n<transfer/tree_update/delegated_deposit> <out_params.params> <path/to/phase1radix>");
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
        "delegated_deposit" => {
            let signal_pub = CDelegatedDepositBatchPub::alloc(rcs, None);
            signal_pub.inputize();
            let signal_sec = CDelegatedDepositBatchSec::alloc(rcs, None);

            delegated_deposit(signal_pub, signal_sec);
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
