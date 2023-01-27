extern crate exitcode;
extern crate fawkes_crypto;
extern crate fawkes_crypto_phase2;
extern crate libzeropool;
extern crate rand;

use std::fs::File;

use fawkes_crypto::backend::bellman_groth16::{engines::Bn256, verifier::VK};
use fawkes_crypto::backend::bellman_groth16::Parameters;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage: \n<in_params.params> <out_vk.json>");
        std::process::exit(exitcode::USAGE);
    }
    let params_filename = &args[1];
    let vk_filename = &args[2];

    println!("Exporting {}...", params_filename);

    let fp = std::path::Path::new(params_filename);
    let mut raw_parameters = std::fs::read(fp).unwrap();
    let mpc = Parameters::<Bn256>::read(&mut raw_parameters.as_slice(), true, true).unwrap();

    let vk = mpc.0.vk.clone();
    let vk: VK<Bn256> = VK::from_bellman(&vk);
    let vk_str = serde_json::to_string_pretty(&vk).unwrap();
    std::fs::write(vk_filename, &vk_str.into_bytes()).unwrap();
}
