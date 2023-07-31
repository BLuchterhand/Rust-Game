use std::collections::HashMap;

use cgmath::Point3;

use crate::world::Chunk;


struct Utils{
    chunk_distance: i32,
    chunk_size: cgmath::Vector2<u32>,
}

impl Utils {
    pub fn new(chunk_distance: i32, chunk_size: cgmath::Vector2<u32>) -> Self {
        Self {
            chunk_distance,
            chunk_size,
        }
    }

    // pub fn get_chunk_by_coords(
    //     &mut self,
    //     chunks: HashMap<String, Chunk>, 
    //     position: Point3<f32>,
    // ) {
    //     // Get the x and z coords of the chunk identifier
    //     let x_coord = (position.x as i32 / self.chunk_size.x as i32) * self.chunk_size.x as i32;
    //     let z_coord = (position.z as i32 / self.chunk_size.y as i32) * self.chunk_size.y as i32;

    //     let chunk_key = format!("{}_{}", x_coord, z_coord);
    //     match chunks.get(&chunk_key) {
    //         Some(&chunk) => println!("FOUND: {:?}", chunk.mesh.vertex_buffer),
    //         _ => println!("NOT FOUND"),
    //     }
    
    // }
}