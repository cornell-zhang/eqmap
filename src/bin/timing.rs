use clap::Parser;
use std::{io::Read, path::PathBuf};

use eqmap::driver::{SynthReport, SynthRequest, process_expression};
use eqmap::lut::LutLang;
use eqmap::netlist::{LogicMapper, PrimitiveCell};
use eqmap::rewrite::{all_static_rules, dyn_decompositions};
use eqmap::timing::{expand_n_nodes, get_critical_path};
use eqmap::verilog::sv_parse_wrapper;

use nl_compiler::from_vast_overrides;
use safety_net::DrivenNet;
use safety_net::Identifier;

#[derive(Parser, Debug)]
struct Args {
    input: PathBuf,
}

fn xilinx_overrides(id: &Identifier, cell: &PrimitiveCell) -> Option<PrimitiveCell> {
    if id.get_name() == "INV" {
        Some(
            cell.clone()
                .remap_input(0, "I".into())
                .remap_output(0, "O".into()),
        )
    } else {
        None
    }
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mut buf = String::new();
    std::fs::File::open(&args.input)?.read_to_string(&mut buf)?;

    let ast = sv_parse_wrapper(&buf, Some(args.input.clone())).map_err(std::io::Error::other)?;

    let netlist = from_vast_overrides(&ast, xilinx_overrides).map_err(std::io::Error::other)?;

    let mut mapper = netlist
        .get_analysis::<LogicMapper<LutLang, PrimitiveCell>>()
        .map_err(std::io::Error::other)?;

    let critical_path = get_critical_path(&netlist)?;
    let expanded_nodes = expand_n_nodes(&critical_path, 2)?;

    let roots: Vec<DrivenNet<PrimitiveCell>> = vec![DrivenNet::from(&critical_path[0])];

    let extracted_expr = mapper
        .insert_filtered(
            roots,
            move |d| expanded_nodes.contains(&d.clone().unwrap()),
            |_| true,
        )
        .map_err(std::io::Error::other)?;
    let mut extracted_regions = mapper.mappings();
    let region_mapping = extracted_regions.pop().unwrap();

    let mut rules = all_static_rules(false);
    rules.append(&mut dyn_decompositions(true));

    let req = SynthRequest::default()
        .with_rules(rules)
        .with_joint_limits(10, 48_000, 32)
        .with_min_depth();

    let result = process_expression::<_, _, SynthReport>(extracted_expr, req, false, false)?
        .with_name(netlist.get_name().as_str());

    drop(critical_path);
    let updated_region_mapping = region_mapping.with_expr(result.get_expr().to_owned());
    updated_region_mapping
        .rewrite(&netlist)
        .map_err(std::io::Error::other)?;

    println!("{netlist}");

    Ok(())
}
