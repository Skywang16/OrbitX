use std::fs;
use std::path::PathBuf;

use terminal_lib::filesystem::commands::{
    code_list_definition_names, fs_list_directory, CodeDefItem,
};
use terminal_lib::utils::ApiResponse;

fn has<S: AsRef<str>>(set: &std::collections::BTreeSet<String>, s: S) -> bool {
    set.contains(s.as_ref())
}

#[tokio::test]
async fn test_fs_list_directory_invalid_inputs() {
    // Non-existent path
    let resp = fs_list_directory("/path/does/not/exist".into(), false).await;
    let ApiResponse {
        code,
        data,
        message,
    } = resp.expect("api resp");
    assert_eq!(code, 500, "should be error for non-existent path");
    assert!(data.is_none());
    assert!(message.is_some());

    // Path is a file
    let tmp = tempfile::tempdir().expect("tempdir");
    let file = tmp.path().join("a.txt");
    fs::write(&file, "x").unwrap();
    let resp = fs_list_directory(file.display().to_string(), false).await;
    let ApiResponse {
        code,
        data,
        message,
    } = resp.expect("api resp");
    assert_eq!(code, 500, "should be error for file path");
    assert!(data.is_none());
    assert!(message.is_some());
}

#[tokio::test]
async fn test_code_list_definition_names_on_directory_non_recursive() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let top = root.join("top.ts");
    let nested_dir = root.join("sub");
    fs::create_dir_all(&nested_dir).unwrap();
    let nested = nested_dir.join("nested.ts");

    fs::write(&top, "function topLevel(){}\n").unwrap();
    fs::write(&nested, "function nested(){}\n").unwrap();

    let resp = code_list_definition_names(root.display().to_string()).await;
    let ApiResponse { code, data, .. } = resp.expect("api resp");
    assert_eq!(code, 200);
    let defs: Vec<CodeDefItem> = data.expect("data");

    // Should include top.ts, but not sub/nested.ts because non-recursive
    let files: std::collections::BTreeSet<_> = defs.iter().map(|d| d.file.clone()).collect();
    assert!(
        files.iter().any(|f| f.ends_with("top.ts")),
        "missing top.ts in {:?}",
        files
    );
    assert!(
        !files.iter().any(|f| f.ends_with("nested.ts")),
        "should not include nested.ts in {:?}",
        files
    );
}

#[tokio::test]
async fn test_code_list_definition_names_invalid_path() {
    let resp = code_list_definition_names("/no/such/path".into()).await;
    let ApiResponse {
        code,
        data,
        message,
    } = resp.expect("api resp");
    assert_eq!(code, 500);
    assert!(data.is_none());
    assert!(message.is_some());
}

#[tokio::test]
async fn test_fs_list_directory_non_recursive_and_recursive_with_gitignore() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    // Files & directories
    fs::write(root.join("a.txt"), "hello").unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    fs::write(root.join("b/c.txt"), "world").unwrap();

    // Ignored entries
    fs::write(root.join("ignored.txt"), "nope").unwrap();
    fs::create_dir_all(root.join("ignored_dir")).unwrap();
    fs::write(root.join("ignored_dir/keep.txt"), "nope").unwrap();

    // .gitignore rules
    fs::write(root.join(".gitignore"), "ignored.txt\nignored_dir/\n").unwrap();

    // Non-recursive
    let resp = fs_list_directory(root.display().to_string(), false).await;
    let ApiResponse { code, data, .. } = resp.expect("api resp");
    assert_eq!(code, 200, "non-recursive should succeed");
    let list = data.expect("data present");
    let set: std::collections::BTreeSet<String> = list.iter().cloned().collect();

    // Should contain top-level file/dir
    assert!(has(&set, "a.txt"), "should contain a.txt, got: {:?}", set);
    assert!(has(&set, "b/"), "should contain b/, got: {:?}", set);

    // Should not contain ignored entries at top-level
    assert!(
        !has(&set, "ignored.txt"),
        "should NOT contain ignored.txt, got: {:?}",
        set
    );
    assert!(
        !has(&set, "ignored_dir/"),
        "should NOT contain ignored_dir/, got: {:?}",
        set
    );

    // Recursive
    let resp = fs_list_directory(root.display().to_string(), true).await;
    let ApiResponse { code, data, .. } = resp.expect("api resp");
    assert_eq!(code, 200, "recursive should succeed");
    let list = data.expect("data present");
    let set: std::collections::BTreeSet<String> = list.iter().cloned().collect();

    // Should include nested file
    assert!(
        has(&set, "b/c.txt"),
        "should contain b/c.txt, got: {:?}",
        set
    );

    // Ignored subtree should not appear
    assert!(
        !has(&set, "ignored_dir/"),
        "should NOT contain ignored_dir/, got: {:?}",
        set
    );
    assert!(
        !has(&set, "ignored_dir/keep.txt"),
        "should NOT contain ignored_dir/keep.txt, got: {:?}",
        set
    );
}

#[tokio::test]
async fn test_code_list_definition_names_on_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let file = root.join("sample.ts");

    let content = r#"
export function foo() {}
class Bar {}
export interface Baz {}
export type T = { a: number };
export enum E { A }
export const arrow = () => {};
export default function defaultFn() {}
"#;

    fs::write(&file, content).unwrap();

    let resp = code_list_definition_names(file.display().to_string()).await;
    let ApiResponse { code, data, .. } = resp.expect("api resp");
    assert_eq!(code, 200, "code defs should succeed");
    let defs: Vec<CodeDefItem> = data.expect("data present");

    let kinds: std::collections::BTreeSet<_> = defs.iter().map(|d| d.kind.as_str()).collect();
    let names: std::collections::BTreeSet<_> = defs.iter().map(|d| d.name.as_str()).collect();

    // Expect presence of multiple definition kinds
    for k in [
        "function",     // foo
        "class",        // Bar
        "interface",    // Baz
        "type",         // T
        "enum",         // E
        "var-function", // arrow
        "default",      // defaultFn
    ] {
        assert!(kinds.contains(k), "missing kind: {} in {:?}", k, kinds);
    }

    // Expect some names
    for n in ["foo", "Bar", "Baz", "T", "E", "arrow", "defaultFn"] {
        assert!(names.contains(n), "missing name: {} in {:?}", n, names);
    }
}
