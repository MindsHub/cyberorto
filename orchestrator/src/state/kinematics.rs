use definitions::Position;

pub fn joint_to_world(x: f32, y: f32, z: f32, arm_length: f32) -> (f32, f32, f32) {
    (
        x - arm_length * y.sin(),
        y - arm_length * y.cos(),
        z,
    )
}

pub fn joint_to_world_pos(pos: &Position, arm_length: f32) -> Position {
    let v = joint_to_world(pos.x, pos.y, pos.z, arm_length);
    Position { x: v.0, y: v.1, z: v.2 }
}

/*fn world_to_joint(x: f32, y: f32, z: f32) -> (f32, f32, f32) {

}*/