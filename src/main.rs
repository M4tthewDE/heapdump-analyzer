// https://github.com/openjdk/jdk17/blob/4afbcaf55383ec2f5da53282a1547bac3d099e9d/src/hotspot/share/services/heapDumper.cpp#L62

use std::path::PathBuf;

mod parser;

fn main() {
    let path_arg = std::env::args().skip(1).next().unwrap();
    let path = PathBuf::from(path_arg);
    let heap = parser::parse(&path);

    dbg!(heap);
}
