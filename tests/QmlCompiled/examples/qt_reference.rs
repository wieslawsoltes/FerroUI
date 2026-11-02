#[path = "../reference/fixture.rs"]
mod fixture;

fn main() {
    fixture::run(|mode, case, result, _| println!("{mode}\t{case}\t{result}"));
}
