use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn refuses_nonempty_output_without_touching_sentinel() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let out =
        std::env::temp_dir().join(format!("workload-cli-test-{}-{unique}", std::process::id()));
    fs::create_dir_all(&out).unwrap();
    let sentinel = out.join("keep.txt");
    fs::write(&sentinel, "preserve me\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_workload-fixtures"))
        .args(["all", "--out"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("output directory is not empty"));
    assert_eq!(fs::read_to_string(&sentinel).unwrap(), "preserve me\n");
    assert_eq!(fs::read_dir(&out).unwrap().count(), 1);
    fs::remove_dir_all(out).unwrap();
}
