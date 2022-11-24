mod lib;

use crate::lib::run;

fn main() {
    pollster::block_on(run::run());
}
