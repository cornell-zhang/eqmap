use std::rc::Rc;

use eqmap::driver::CircuitLang;
use eqmap::lut::LutLang;
use eqmap::netlist::{LogicMapper, PrimitiveCell};
use eqmap::timing::get_critical_paths;
use eqmap::verilog::sv_parse_wrapper;
use nl_compiler::from_vast;
use safety_net::graph::CombDepthInfo;
use safety_net::{DrivenNet, Netlist};
use safety_pass::CellType;

fn and_gate() -> PrimitiveCell {
    PrimitiveCell::new(CellType::AND, None)
}

fn reg_cell() -> PrimitiveCell {
    PrimitiveCell::new(CellType::FDRE, None)
}

fn timing_analysis(
    netlist: &Rc<Netlist<PrimitiveCell>>,
) -> safety_net::graph::CombDepthInfo<'_, PrimitiveCell> {
    netlist.get_analysis::<CombDepthInfo<_>>().unwrap()
}

// Visual representation
// a ──┐
//     ├── [AND left] ──┐
// b ──┘                │
//                      ├── [AND root] ── y
// c ──┐                │
//     ├── [AND right] ─┘
// d ──┘
fn reconvergent_netlist() -> (
    Rc<Netlist<PrimitiveCell>>,
    DrivenNet<PrimitiveCell>,
    DrivenNet<PrimitiveCell>,
    DrivenNet<PrimitiveCell>,
) {
    let netlist = Netlist::new("reconvergent".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let d = netlist.insert_input("d".into());

    let left = netlist
        .insert_gate(and_gate(), "left".into(), &[a, b])
        .unwrap();
    let right = netlist
        .insert_gate(and_gate(), "right".into(), &[c, d])
        .unwrap();
    let root = netlist
        .insert_gate(
            and_gate(),
            "root".into(),
            &[left.get_output(0), right.get_output(0)],
        )
        .unwrap();
    root.clone().expose_with_name("y".into());

    (
        netlist,
        root.get_output(0),
        left.get_output(0),
        right.get_output(0),
    )
}

struct TwoOutputNetlist {
    netlist: Rc<Netlist<PrimitiveCell>>,
    first_root: DrivenNet<PrimitiveCell>,
    first_leaf: DrivenNet<PrimitiveCell>,
    second_root: DrivenNet<PrimitiveCell>,
    second_leaf: DrivenNet<PrimitiveCell>,
}

fn two_output_netlist() -> TwoOutputNetlist {
    let netlist = Netlist::new("two_output".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let d = netlist.insert_input("d".into());
    let e = netlist.insert_input("e".into());
    let f = netlist.insert_input("f".into());

    let first_leaf = netlist
        .insert_gate(and_gate(), "first_leaf".into(), &[a, b])
        .unwrap();
    let first_root = netlist
        .insert_gate(
            and_gate(),
            "first_root".into(),
            &[first_leaf.get_output(0), c],
        )
        .unwrap();
    first_root.clone().expose_with_name("y0".into());

    let second_leaf = netlist
        .insert_gate(and_gate(), "second_leaf".into(), &[d, e])
        .unwrap();
    let second_root = netlist
        .insert_gate(
            and_gate(),
            "second_root".into(),
            &[second_leaf.get_output(0), f],
        )
        .unwrap();
    second_root.clone().expose_with_name("y1".into());

    TwoOutputNetlist {
        netlist,
        first_root: first_root.get_output(0),
        first_leaf: first_leaf.get_output(0),
        second_root: second_root.get_output(0),
        second_leaf: second_leaf.get_output(0),
    }
}

fn single_chain_netlist() -> (
    Rc<Netlist<PrimitiveCell>>,
    DrivenNet<PrimitiveCell>,
    DrivenNet<PrimitiveCell>,
    DrivenNet<PrimitiveCell>,
) {
    let netlist = Netlist::new("single_chain".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let d = netlist.insert_input("d".into());

    let first = netlist
        .insert_gate(and_gate(), "first".into(), &[a, b])
        .unwrap();
    let second = netlist
        .insert_gate(and_gate(), "second".into(), &[first.get_output(0), c])
        .unwrap();
    let third = netlist
        .insert_gate(and_gate(), "third".into(), &[second.get_output(0), d])
        .unwrap();
    third.clone().expose_with_name("y".into());

    (
        netlist,
        first.get_output(0),
        second.get_output(0),
        third.get_output(0),
    )
}

#[test]
fn critical_path_uses_one_max_depth_branch() {
    let (netlist, root, left, _right) = reconvergent_netlist();
    let analysis = timing_analysis(&netlist);

    let path = get_critical_paths(&analysis).next().unwrap();

    assert_eq!(path.endpoint(), root);
    assert_eq!(path.path(), &[root, left]);
}

#[test]
fn expansion_adds_neighboring_fanin_nodes() {
    let (netlist, _root, _left, right) = reconvergent_netlist();
    let analysis = timing_analysis(&netlist);
    let path = get_critical_paths(&analysis).next().unwrap();

    let unexpanded = path.expand_n_nodes(0);
    let expanded = path.expand_n_nodes(1);

    assert!(!unexpanded.contains(&right));
    assert!(expanded.contains(&right));
}

#[test]
fn gets_multiple_critical_paths() {
    let TwoOutputNetlist {
        netlist,
        first_root,
        first_leaf,
        second_root,
        second_leaf,
    } = two_output_netlist();
    let analysis = timing_analysis(&netlist);

    let paths = get_critical_paths(&analysis).take(2).collect::<Vec<_>>();

    assert_eq!(paths.len(), 2);
    assert!(
        paths
            .iter()
            .any(|path| path.path() == [first_root.clone(), first_leaf.clone()])
    );
    assert!(
        paths
            .iter()
            .any(|path| path.path() == [second_root.clone(), second_leaf.clone()])
    );
}

#[test]
fn critical_paths_use_timing_endpoints_not_internal_chain_nodes() {
    let (netlist, first, second, third) = single_chain_netlist();
    let analysis = timing_analysis(&netlist);

    let paths = get_critical_paths(&analysis).collect::<Vec<_>>();

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].endpoint(), third);
    assert_eq!(paths[0].depth(), 3);
    assert_eq!(paths[0].path(), &[third, second, first]);
}

#[test]
fn critical_path_stops_at_register_boundary() {
    let netlist = Netlist::new("registered".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let d = netlist.insert_input("d".into());
    let clk = netlist.insert_input("clk".into());
    let ce = netlist.insert_input("ce".into());
    let rst = netlist.insert_input("rst".into());

    let before_a = netlist
        .insert_gate(and_gate(), "before_a".into(), &[a, b])
        .unwrap();
    let before_b = netlist
        .insert_gate(and_gate(), "before_b".into(), &[before_a.get_output(0), c])
        .unwrap();

    let reg = netlist.insert_gate_disconnected(reg_cell(), "reg".into());
    reg.find_input(&"D".into())
        .unwrap()
        .connect(before_b.get_output(0));
    reg.find_input(&"C".into()).unwrap().connect(clk);
    reg.find_input(&"CE".into()).unwrap().connect(ce);
    reg.find_input(&"R".into()).unwrap().connect(rst);

    let after = netlist
        .insert_gate(and_gate(), "after".into(), &[reg.get_output(0), d])
        .unwrap();
    after.expose_with_name("y".into());

    let analysis = timing_analysis(&netlist);
    let path = get_critical_paths(&analysis).next().unwrap();

    assert_eq!(
        path.path(),
        &[before_b.get_output(0), before_a.get_output(0)]
    );
    assert!(!path.path().contains(&reg.get_output(0)));
}

#[test]
fn insert_delay_paths_maps_only_the_critical_region() {
    let (netlist, root, left, right) = reconvergent_netlist();
    let mut mapper = netlist
        .get_analysis::<LogicMapper<LutLang, PrimitiveCell>>()
        .unwrap();

    mapper.insert_delay_paths(1, 0).unwrap();

    let mappings = mapper.mappings();
    assert_eq!(mappings.len(), 1);

    let mapping = &mappings[0];
    let roots = mapping.root_nets().collect::<Vec<_>>();
    let expr = mapping.get_expr();
    let vars = expr
        .iter()
        .filter_map(|node| node.get_var().map(|sym| sym.to_string()))
        .collect::<Vec<_>>();

    assert_eq!(roots, vec![root.clone()]);
    assert!(vars.contains(&right.get_identifier().to_string()));
    assert!(!vars.contains(&root.get_identifier().to_string()));
    assert!(!vars.contains(&left.get_identifier().to_string()));
}

// Visual representation
// a ──┐
//     ├─[c1]─┐
// b ──┘      ├─[c2]─┐
// c ─────────┘      ├─[c3]─┐
// d ────────────────┘      ├─[c4]─┐
// e ───────────────────────┘      │
//                                 ├─[root]─ y
// f ──┐                           │
//     ├─[s1]─┐                    │
// g ──┘      ├─[s2]─┐             │
// h ─────────┘      ├─[s3]────────┘
// i ────────────────┘
#[test]
fn delay_path_expansion_from_verilog_stops_at_requested_depth() {
    let verilog = r#"
module timing_branch (
    a, b, c, d, e, f, g, h, i, y
);
  input a;
  input b;
  input c;
  input d;
  input e;
  input f;
  input g;
  input h;
  input i;
  output y;
  wire a;
  wire b;
  wire c;
  wire d;
  wire e;
  wire f;
  wire g;
  wire h;
  wire i;
  wire y;
  wire c1_out;
  wire c2_out;
  wire c3_out;
  wire c4_out;
  wire s1_out;
  wire s2_out;
  wire s3_out;

  AND c1 (.A(a),      .B(b),      .Y(c1_out));
  AND c2 (.A(c1_out), .B(c),      .Y(c2_out));
  AND c3 (.A(c2_out), .B(d),      .Y(c3_out));
  AND c4 (.A(c3_out), .B(e),      .Y(c4_out));

  AND s1 (.A(f),      .B(g),      .Y(s1_out));
  AND s2 (.A(s1_out), .B(h),      .Y(s2_out));
  AND s3 (.A(s2_out), .B(i),      .Y(s3_out));

  AND root (.A(c4_out), .B(s3_out), .Y(y));
endmodule
"#;

    let ast = sv_parse_wrapper(verilog, None).unwrap();
    let netlist = from_vast::<PrimitiveCell>(&ast).unwrap();
    let analysis = timing_analysis(&netlist);
    let critical_path = get_critical_paths(&analysis).next().unwrap();
    let path_names = critical_path
        .path()
        .iter()
        .map(|net| net.get_identifier().to_string())
        .collect::<Vec<_>>();

    assert_eq!(path_names, ["y", "c4_out", "c3_out", "c2_out", "c1_out"]);

    let mut mapper = netlist
        .get_analysis::<LogicMapper<LutLang, PrimitiveCell>>()
        .unwrap();
    mapper.insert_delay_paths(1, 2).unwrap();

    let mappings = mapper.mappings();
    assert_eq!(mappings.len(), 1);

    let mapping = &mappings[0];
    let root_names = mapping
        .root_nets()
        .map(|net| net.get_identifier().to_string())
        .collect::<Vec<_>>();
    let vars = mapping
        .get_expr()
        .iter()
        .filter_map(|node| node.get_var().map(|symbol| symbol.to_string()))
        .collect::<Vec<_>>();

    assert_eq!(root_names, ["y"]);
    assert!(vars.contains(&"s1_out".to_string()));
    assert!(!vars.contains(&"s2_out".to_string()));
    assert!(!vars.contains(&"s3_out".to_string()));
}
