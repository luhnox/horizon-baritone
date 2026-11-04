fn baritone_unload(world: &World, x: i32, z: i32) {
    if let Some(chunk) = world.chunks.get(&(x, z)) {
        baritone_unload::push(x, z, chunk.clone());
    }
}
