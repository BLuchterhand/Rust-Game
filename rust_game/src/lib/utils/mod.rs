
// struct VertexData {
//   position: [f32; 3],
//   normal: [f32; 3],
// }

// Loading vertex pos!!!!
 // let vertex_count = data.len() / 8 / std::mem::size_of::<f32>(); // 2 attributes (position and normal)
// for i in 0..vertex_count {
//     let vertex_offset = i * std::mem::size_of::<VertexData>();

//     let position_bytes = &data[vertex_offset..vertex_offset + 3 * std::mem::size_of::<f32>()];
//     let result: Vec<f32> = bytemuck::cast_slice(&position_bytes).to_vec();
//     new_chunks.insert(chunk_key.clone(), RawBufferData {
//         vertex_data: result,
//     });

//     if chunk_key == "0_0"{
//         println!("HERE");
//         println!("{:?}", chunk_key);
//     }
// }