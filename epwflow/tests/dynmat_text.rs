use epwflow::qe_data::{
    dynmat_text,
    xml::{self, Geometry},
};
fn geometry() -> Geometry {
    Geometry {
        alat: 2.0,
        lattice: [[2., 0., 0.], [-0.4, 2., 0.], [0., 0., 3.]],
        reciprocal: [[1., 0.2, 0.], [0., 1., 0.], [0., 0., 2. / 3.]],
        atoms: vec![("Si".into(), [0.; 3])],
        fft: [2; 3],
    }
}
fn input() -> String {
    let mut s=String::from("Dynamical matrix file\nexample\n1 1 0 2 0 0 0 0 0\nBasis vectors\n1 0 0\n-.2 1 0\n0 0 1.5\n1 'Si' 25598.3672898282\n1 1 0 0 0\n");
    for q in ["0 0 0", ".5 .1 0"] {
        s.push_str(&format!("Dynamical  Matrix in cartesian axes\nq = ( {q} )\n1 1\n1 2 3 4 5 6\n7 8 9 10 11 12\n13 14 15 16 17 18\n"));
    }
    for m in 1..=3 {
        s.push_str(&format!(
            "freq ( {m}) = {m} [THz] = {} [cm-1]\n( 1 0 0 1 0 0 )\n",
            m * 33
        ));
    }
    s
}
#[test]
fn all_stars_and_fortran_complex_order_are_preserved() {
    let (s, q) = dynmat_text::convert(&input(), &geometry()).unwrap();
    assert_eq!(q, vec![[0.; 3], [0.5, 0.1, 0.]]);
    let d = roxmltree::Document::parse(&s).unwrap();
    let r = d.root_element();
    assert_eq!(
        xml::numbers(xml::text(r, "PHI.1.1").unwrap()).unwrap(),
        vec![1., 2., 7., 8., 13., 14., 3., 4., 9., 10., 15., 16., 5., 6., 11., 12., 17., 18.]
    );
    assert!((xml::numbers(xml::text(r, "MASS.1").unwrap()).unwrap()[0] - 28.0855).abs() < 1e-6);
    assert_eq!(
        xml::numbers(xml::text(r, "DISPLACEMENT.3").unwrap()).unwrap(),
        vec![1., 0., 0., 1., 0., 0.]
    );
    assert!(s.contains("<PHI.1.1>\n"));
}
#[test]
fn truncated_and_nonfinite_and_mismatched_dyn_are_rejected() {
    for raw in [
        input().replace("13 14 15 16 17 18", "13 14 15"),
        input().replace("1 2 3 4 5 6", "NaN 2 3 4 5 6"),
        input().replace("1 'Si'", "1 'P'"),
        input().replace("freq ( 2)", "freq ( 3)"),
        input().replace("( 1 0 0 1 0 0 )", "( 1 0 )"),
    ] {
        assert!(dynmat_text::convert(&raw, &geometry()).is_err());
    }
}
#[test]
fn polar_text_fails_instead_of_dropping_born_charges() {
    assert!(dynmat_text::convert(&(input() + "\nDielectric Tensor"), &geometry()).is_err());
}
#[test]
#[ignore = "需要本地已复制 SiP2 save；仅解析，不运行科学程序"]
fn real_sip2_all_text_stars_convert_without_loss() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/SiP2");
    let g =
        Geometry::read(&root.join("1.qe-scf/snapshot/Si8P16.save/data-file-schema.xml")).unwrap();
    let mut stars = 0;
    for n in 1..=16 {
        let raw =
            std::fs::read_to_string(root.join(format!("external-phonon/save/Si8P16.dyn_q{n}")))
                .unwrap();
        let (s, qs) = dynmat_text::convert(&raw, &g).unwrap();
        let doc = roxmltree::Document::parse(&s).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|x| x.has_tag_name("FREQUENCIES_THZ_CMM1"))
                .count(),
            1
        );
        assert_eq!(
            doc.descendants()
                .filter(|x| x.is_element() && x.tag_name().name().starts_with("OMEGA."))
                .count(),
            72
        );
        stars += qs.len();
    }
    assert_eq!(stars, 36);
}
#[test]
fn polar_tensors_keep_matrix_order() {
    let raw=input()+"\nDielectric Tensor:\n\n1 2 3\n4 5 6\n7 8 9\n\nEffective Charges E-U:\n\natom # 1\n1 2 3\n4 5 6\n7 8 9\n";
    let (s, _) = dynmat_text::convert(&raw, &geometry()).unwrap();
    let d = roxmltree::Document::parse(&s).unwrap();
    assert_eq!(
        xml::numbers(xml::text(d.root_element(), "EPSILON").unwrap()).unwrap(),
        vec![1., 4., 7., 2., 5., 8., 3., 6., 9.]
    );
    assert_eq!(
        xml::numbers(xml::text(d.root_element(), "Z_AT_.1").unwrap()).unwrap(),
        vec![1., 4., 7., 2., 5., 8., 3., 6., 9.]
    );
}
#[test]
fn nonzero_bravais_is_reconstructed_and_wrong_lattice_is_rejected() {
    let mut g = geometry();
    g.lattice = [[2., 0., 0.], [0., 2., 0.], [0., 0., 3.]];
    g.reciprocal = [[1., 0., 0.], [0., 1., 0.], [0., 0., 2. / 3.]];
    let raw = input().replace(
        "1 1 0 2 0 0 0 0 0\nBasis vectors\n1 0 0\n-.2 1 0\n0 0 1.5",
        "1 1 6 2 0 1.5 0 0 0",
    );
    assert!(dynmat_text::convert(&raw, &g).is_ok());
    assert!(dynmat_text::convert(&raw.replace("6 2 0 1.5", "6 2 0 2.0"), &g).is_err());
}
