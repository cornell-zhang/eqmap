use std::rc::Rc;

use eqmap::netlist::PrimitiveCell;
use eqmap::timing::{expand_n_nodes, get_critical_path, get_critical_paths};
use eqmap::verilog::PrimitiveType;
use safety_net::{NetRef, Netlist};

fn and_gate() -> PrimitiveCell {
    PrimitiveCell::new(PrimitiveType::AND, None)
}

fn reg_cell() -> PrimitiveCell {
    PrimitiveCell::new(PrimitiveType::FDRE, None)
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
    NetRef<PrimitiveCell>,
    NetRef<PrimitiveCell>,
    NetRef<PrimitiveCell>,
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

    (netlist, root, left, right)
}

struct TwoOutputNetlist {
    netlist: Rc<Netlist<PrimitiveCell>>,
    first_root: NetRef<PrimitiveCell>,
    first_leaf: NetRef<PrimitiveCell>,
    second_root: NetRef<PrimitiveCell>,
    second_leaf: NetRef<PrimitiveCell>,
}

// a ──┐
//     ├── [AND first_leaf] ──┐
// b ──┘                     │
//                           ├── [AND first_root] ── y0
// c ────────────────────────┘

// d ──┐
//     ├── [AND second_leaf] ─┐
// e ──┘                     │
//                           ├── [AND second_root] ── y1
// f ────────────────────────┘
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
        first_root,
        first_leaf,
        second_root,
        second_leaf,
    }
}

// a ──┐
//     ├── [AND first] ──┐
// b ──┘                │
//                      ├── [AND second] ──┐
// c ───────────────────┘                  │
//                                         ├── [AND third] ── y
// d ──────────────────────────────────────┘
fn single_chain_netlist() -> (
    Rc<Netlist<PrimitiveCell>>,
    NetRef<PrimitiveCell>,
    NetRef<PrimitiveCell>,
    NetRef<PrimitiveCell>,
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

    (netlist, first, second, third)
}

#[test]
fn critical_path_uses_one_max_depth_branch() {
    let (netlist, root, left, _right) = reconvergent_netlist();

    let path = get_critical_path(&netlist).unwrap();

    assert_eq!(path, vec![root, left]);
}

#[test]
fn expansion_adds_neighboring_fanin_nodes() {
    let (netlist, _root, _left, right) = reconvergent_netlist();
    let path = get_critical_path(&netlist).unwrap();

    let unexpanded = expand_n_nodes(&path, 0).unwrap();
    let expanded = expand_n_nodes(&path, 1).unwrap();

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

    let paths = get_critical_paths(&netlist, 2).unwrap();

    assert_eq!(paths.len(), 2);
    assert!(
        paths
            .iter()
            .any(|critical_path| critical_path.path == vec![first_root.clone(), first_leaf.clone()])
    );
    assert!(
        paths
            .iter()
            .any(|critical_path| critical_path.path
                == vec![second_root.clone(), second_leaf.clone()])
    );
}

#[test]
fn requesting_zero_critical_paths_returns_empty_result() {
    let (netlist, _root, _left, _right) = reconvergent_netlist();

    let paths = get_critical_paths(&netlist, 0).unwrap();

    assert!(paths.is_empty());
}

#[test]
fn critical_paths_use_timing_endpoints_not_internal_chain_nodes() {
    let (netlist, first, second, third) = single_chain_netlist();

    let paths = get_critical_paths(&netlist, 3).unwrap();

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].endpoint, third);
    assert_eq!(paths[0].depth, 3);
    assert_eq!(paths[0].path, vec![third, second, first]);
}

// a ──┐
//     ├── [AND before_a] ──┐
// b ──┘                   │
//                         ├── [AND before_b] ── D
// c ──────────────────────┘                     │
//                                               ▼
//                                       [FDRE register]
//                                       C  <- clk
//                                       CE <- ce
//                                       R  <- rst
//                                               │ Q
//                                               ▼
// d ─────────────────────────────────────┐
//                                        ├── [AND after] ── y
// reg.Q ─────────────────────────────────┘
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

    let path = get_critical_path(&netlist).unwrap();

    assert_eq!(path, vec![before_b, before_a]);
    assert!(!path.contains(&reg));
}

#[test]
fn ill_formed_netlist_has_no_critical_path() {
    let netlist = Netlist::new("ill_formed".to_string());

    let gate = netlist.insert_gate_disconnected(and_gate(), "and".into());
    gate.expose_with_name("y".into());

    assert!(get_critical_path(&netlist).is_err());
}
