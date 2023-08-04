mod lib;
mod light;
mod world;

use crate::lib::run;

#[tokio::main]
async fn main() {
    pollster::block_on(run::run());
}
