mod lib;
mod light;
mod world;

use crate::lib::run;

fn main() {
    pollster::block_on(run::run());
}
