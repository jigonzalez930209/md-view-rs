#[derive(Debug)]
struct Level {
    depth: usize,
    name: &'static str,
}

fn main() {
    let level = Level { depth: 5, name: "level5" };
    println!("{level:?}");
}
