use definitions::{Parameters, Vec3};

pub fn joint_to_world(pos: &Vec3, params: &Parameters) -> Vec3 {
    Vec3 {
        x: pos.x - params.arm_length * pos.y.sin(),
        y: - params.arm_length * pos.y.cos(),
        z: pos.z,
    }
}

/// TODO implement any corrections due to tools
pub fn world_to_joint(pos: &Vec3, params: &Parameters) -> (f32, f32, f32) {
    let mut angle = (pos.x / params.arm_length).asin();

    let mut x = pos.x;
    if pos.x < params.rail_length - params.arm_length {
        x += params.arm_length * angle.cos();
        angle = angle.to_degrees();
    } else {
        x += -params.arm_length * angle.cos();
        angle = (std::f32::consts::PI - angle).to_degrees();
        if angle >= 180.0 {
            angle -= 360.0;
        }
    }

    (x, angle, pos.z)
}
