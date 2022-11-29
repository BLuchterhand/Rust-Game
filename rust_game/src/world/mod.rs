use opensimplex_noise_rs::OpenSimplexNoise;

pub struct World {
  terrain: OpenSimplexNoise,
  terrain_scale: f64
}

impl World {
  pub fn new() -> Self {
    let terrain = OpenSimplexNoise::new(Some(883_279_212_983_182_319));
    let terrain_scale = 0.044;

    Self {
      terrain,
      terrain_scale
    }
  }

  pub fn get_terrain(&mut self, x: f64, z: f64) -> f64{
    let y = self.terrain.eval_2d(x * self.terrain_scale, z * self.terrain_scale); // generates value in range (-1, 1)
    y
  }
}