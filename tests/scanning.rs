use gdnative_project_utils::*;

#[test]
fn scanning() {
    let res = scan_crate("tests/project_stub").expect("Scanning should work");

    assert_eq!(res.len(), 3);
    assert!(res.contains("Test"));
    assert!(res.contains("MoreTest"));
    assert!(res.contains("EvenMoreTest"));
}
