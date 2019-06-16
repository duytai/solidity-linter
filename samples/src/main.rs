extern crate loader;
extern crate json;
extern crate core;
extern crate cfg;

use std::io::*;
use std::path::Path;
use core::{ Dictionary, State };
use cfg::{ ControlFlowGraph };
use loader::{
    Solidity,
    SolidityOption,
    SolidityOutputKind,
    SolidityOutput,
    SolidityASTOutput,
};

fn main() -> Result<()> {
    let home_dir = env!("CARGO_MANIFEST_DIR");
    let assets_dir = Path::new(home_dir).join("assets/"); 
    let contract_dir = assets_dir.join("contracts/");
    let option = SolidityOption {
        bin_dir: &assets_dir.join("bin/"),
        contract: &contract_dir.join("Math.sol"),
        kind: SolidityOutputKind::AST,
    };
    let solidity = Solidity::new(option);
    let solidity_output = solidity.compile()?;
    match solidity_output {
        SolidityOutput::AST(SolidityASTOutput { ast, sources }) => {
            let ast_json = json::parse(&ast).expect("Invalid json format");
            let dict = Dictionary::new(&ast_json, &sources);
            let mut control_flow = ControlFlowGraph::new(&dict);
            let State { vertices, edges, .. } = control_flow.start_at(19).unwrap();
        }
    }
    Ok(())
}
