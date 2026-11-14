use super::*;

#[test]
fn file_index_lists_nested_files_without_loading_the_tree() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir_all(dir.path().join("docs/deep")).expect("dirs");
    fs::write(dir.path().join("README.md"), "# readme").expect("file");
    fs::write(dir.path().join("docs/deep/guide.md"), "# guide").expect("file");
    let workspace = Workspace::open(dir.path()).expect("open");
    let files = workspace.file_index(100);
    assert!(files.iter().any(|path| path.ends_with("README.md")));
    assert!(files.iter().any(|path| path.ends_with("guide.md")));
}

#[test]
fn refreshing_the_root_preserves_expansion() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("src")).expect("src");
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").expect("file");
    let mut workspace = Workspace::open(dir.path()).expect("open");
    workspace.expand(&dir.path().join("src"));
    workspace.refresh_dir(dir.path());
    let src = workspace.root_entry().find(&dir.path().join("src")).expect("src");
    assert!(src.expanded);
    assert_eq!(src.children.as_ref().map(Vec::len), Some(1));
}

#[test]
fn lazy_tree_filters_and_expands() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::create_dir(dir.path().join("src")).expect("src");
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").expect("file");
    fs::write(dir.path().join(".hidden"), "hidden").expect("hidden");
    let mut workspace = Workspace::open(dir.path()).expect("open");
    assert_eq!(workspace.root_entry().children.as_ref().map(Vec::len), Some(1));
    let src = workspace.root().join("src");
    workspace.expand(&src);
    assert_eq!(
        workspace
            .root_entry()
            .find(&src)
            .and_then(|entry| entry.children.as_ref())
            .map(Vec::len),
        Some(1)
    );
    workspace.set_show_hidden(true);
    assert!(workspace.root_entry().find(&workspace.root().join(".hidden")).is_some());
}

#[test]
fn gitignore_and_hidden_files_are_filtered() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join(".gitignore"), "ignored.md\nbuild/\n").expect("gitignore");
    fs::write(dir.path().join("kept.md"), "kept").expect("kept");
    fs::write(dir.path().join("ignored.md"), "ignored").expect("ignored");
    fs::create_dir(dir.path().join("build")).expect("build");
    let mut workspace = Workspace::open(dir.path()).expect("open");
    let names: Vec<&str> = workspace
        .root_entry()
        .children
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(names.contains(&"kept.md"));
    assert!(!names.contains(&"ignored.md"));
    assert!(!names.contains(&"build"));
    assert!(!names.contains(&".gitignore"));
    workspace.set_show_hidden(true);
    assert!(workspace.root_entry().find(&dir.path().join(".gitignore")).is_some());
    workspace.set_respect_gitignore(false);
    assert!(workspace.root_entry().find(&dir.path().join("ignored.md")).is_some());
}

#[test]
fn watcher_reports_changed_paths_or_falls_back_to_polling() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut workspace = Workspace::open(dir.path()).expect("open");
    let path = dir.path().join("watched.md");
    fs::write(&path, "one").expect("write");
    if workspace.watcher_warning().is_some() {
        workspace.refresh_dir(dir.path());
        assert!(workspace.root_entry().find(&path).is_some());
        return;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut reported = false;
    while Instant::now() < deadline {
        workspace.poll();
        if workspace.take_changed_paths().contains(&path) {
            reported = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(reported, "the watcher did not report the new file");
    assert!(workspace.root_entry().find(&path).is_some());
}

#[test]
fn fifty_thousand_files_load_lazily() {
    let dir = tempfile::tempdir().expect("temp dir");
    for index in 0..50_000 {
        std::fs::File::create(dir.path().join(format!("file{index:05}.md"))).expect("file");
    }
    let nested = dir.path().join("nested");
    fs::create_dir(&nested).expect("nested");
    fs::write(nested.join("deep.md"), "deep").expect("deep");
    let start = Instant::now();
    let mut workspace = Workspace::open(dir.path()).expect("open");
    let elapsed = start.elapsed();
    assert_eq!(workspace.root_entry().children.as_ref().map(Vec::len), Some(50_001));
    let nested_entry = workspace.root_entry().find(&nested).expect("nested entry");
    assert!(nested_entry.children.is_none(), "children load only on expand");
    workspace.expand(&nested);
    assert_eq!(
        workspace
            .root_entry()
            .find(&nested)
            .and_then(|entry| entry.children.as_ref())
            .map(Vec::len),
        Some(1)
    );
    eprintln!("50k workspace first level loaded in {elapsed:?}");
}

#[test]
fn refreshes_created_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace = Workspace::open(dir.path()).expect("open");
    assert_eq!(workspace.root_entry().children.as_ref().map(Vec::len), Some(0));
    fs::write(dir.path().join("new.md"), "# new\n").expect("write");
    let mut workspace = workspace;
    let root = workspace.root().to_path_buf();
    workspace.refresh_dir(&root);
    assert_eq!(workspace.root_entry().children.as_ref().map(Vec::len), Some(1));
}
