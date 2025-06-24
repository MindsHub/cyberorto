use std::f32::consts::{PI, TAU};

use definitions::{Parameters, Vec3};

const EPSILON: f32 = 1e-6; // 1 micrometer

pub fn joint_to_world(pos: &Vec3, params: &Parameters) -> Vec3 {
    Vec3 {
        x: pos.x - params.arm_length * pos.y.cos(),
        y: - params.arm_length * pos.y.sin(),
        z: pos.z,
    }
}

/// TODO implement any corrections due to tools
/// TODO maybe do calculations in f64 or f128 to avoid precision errors
pub fn world_to_joint(pos: &Vec3, params: &Parameters) -> Option<Vec3> {
    let mut angle = (- pos.y / params.arm_length).asin();
    if angle.is_nan() {
        return None;
    }

    if pos.x >= params.rail_length - params.arm_length {
        angle = PI - angle;
        if angle >= PI {
            angle -= TAU; // 2 pi
        }
    }

    let new_x = pos.x + params.arm_length * angle.cos();
    if new_x < -EPSILON || new_x > params.rail_length + EPSILON {
        return None;
    }

    Some(Vec3 {
        x: new_x,
        y: angle,
        z: pos.z
    })
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use definitions::{Parameters, Vec3};
    use rand::{rngs::StdRng, Rng, SeedableRng};

    use crate::state::kinematics::{joint_to_world, world_to_joint, EPSILON};

    const PARAMS_1_3: Parameters = Parameters { arm_length: 1.0, rail_length: 3.0 };

    const fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    /// Uniformly samples a point from the world space
    fn random_world_point(rng: &mut StdRng, params: &Parameters) -> Vec3 {
        let z = rng.random_range(-0.1..=1.0);
        loop {
            let x = rng.random_range(-params.arm_length..=(params.rail_length + params.arm_length));
            let y = rng.random_range(-params.arm_length..=params.arm_length);
            if x < 0.0 && (x*x + y*y) > params.arm_length*params.arm_length {
                continue;
            }
            if x > params.rail_length && ((x-params.rail_length)*(x-params.rail_length) + y*y) > params.arm_length*params.arm_length {
                continue;
            }
            return Vec3 { x, y, z };
        }
    }

    fn assert_vec3_roughly_eq(expected: &Vec3, actual: &Vec3, msg: &str) {
        assert!(
            (expected.x - actual.x).abs() <= EPSILON &&
            (expected.y - actual.y).abs() <= EPSILON &&
            (expected.z - actual.z).abs() <= EPSILON,
            "Expected {expected:?}, got {actual:?}, for {msg}"
        );
    }

    fn assert_world_to_joint_to_world(expected_world: &Vec3, params: &Parameters) {
        let Some(joint) = world_to_joint(expected_world, params) else {
            panic!("unexpected failed conversion from world space {expected_world:?} to joint space");
        };
        let actual_world = joint_to_world(&joint, params);
        assert_vec3_roughly_eq(expected_world, &actual_world, "world to joint to world");
    }

    fn assert_world_to_joint(world: &Vec3, expected_joint: &Vec3, params: &Parameters) {
        let Some(actual_joint) = world_to_joint(world, params) else {
            panic!("unexpected failed conversion from world space {world:?} to joint space");
        };
        assert_vec3_roughly_eq(expected_joint, &actual_joint, "world to joint");
    }

    fn assert_world_to_joint_invalid(world: &Vec3, params: &Parameters) {
        if let Some(joint) = world_to_joint(world, params) {
            panic!("Unexpected {joint:?} configuration generated from invalid world coordinates {world:?}");
        }
    }

    fn assert_joint_to_world(joint: &Vec3, expected_world: &Vec3, params: &Parameters) {
        let actual_world = joint_to_world(joint, params);
        assert_vec3_roughly_eq(expected_world, &actual_world, "joint to world");
    }

    #[test]
    fn world_to_joint_to_world() {
        // change z (no problem expected)
        assert_world_to_joint_to_world(&vec3( 0.0,  0.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.0,  0.0,  1.2), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.0,  0.0, -0.9), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.0,  0.0,  7e9), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.0,  0.0, -7e9), &PARAMS_1_3);

        // change x, normal rotation
        assert_world_to_joint_to_world(&vec3( 1.0,  0.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.5,  0.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3(-0.5,  0.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3(-1.0,  0.0,  0.0), &PARAMS_1_3);

        // change y
        assert_world_to_joint_to_world(&vec3( 0.0,  0.5,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.0,  1.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.0, -0.5,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.0, -1.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 0.0,  0.0,  0.0), &PARAMS_1_3);

        // change x, reverse rotation
        assert_world_to_joint_to_world(&vec3( 3.0,  0.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 2.5,  0.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 3.5,  0.0,  0.0), &PARAMS_1_3);
        assert_world_to_joint_to_world(&vec3( 4.0,  0.0,  0.0), &PARAMS_1_3);
    }

    #[test]
    fn world_to_joint_to_world_random() {
        let mut rng = StdRng::from_seed([42; 32]);
        for _ in 0..1000 {
            let world = random_world_point(&mut rng, &PARAMS_1_3);
            assert_world_to_joint_to_world(&world, &PARAMS_1_3);
        }
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn world_to_joint_test() {
        // at the rail ends
        assert_world_to_joint(&vec3( 0.0,  0.0, 0.0), &vec3(1.0, 0.0, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 0.0,  0.0, 0.0), &vec3(1.0, 0.0, 0.0), &PARAMS_1_3);

        // at other key limit points
        let pi2 = FRAC_PI_2;
        assert_world_to_joint(&vec3(-1.0,  0.0, 0.0), &vec3(0.0,  0.0, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 0.0,  1.0, 0.0), &vec3(0.0, -pi2, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 0.0, -1.0, 0.0), &vec3(0.0,  pi2, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 4.0,  0.0, 0.0), &vec3(3.0,  -PI, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 3.0,  1.0, 0.0), &vec3(3.0, -pi2, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 3.0, -1.0, 0.0), &vec3(3.0,  pi2, 0.0), &PARAMS_1_3);

        // around the reset end of the rail, normal rotation
        assert_world_to_joint(&vec3( 0.2,  0.2, 0.0), &vec3(1.1797959, -0.2013579, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3(-0.2,  0.2, 0.0), &vec3(0.7797959, -0.2013579, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 0.2, -0.2, 0.0), &vec3(1.1797959,  0.2013579, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3(-0.2, -0.2, 0.0), &vec3(0.7797959,  0.2013579, 0.0), &PARAMS_1_3);

        // around the middle end of the rail, normal rotation
        assert_world_to_joint(&vec3( 1.7,  0.2, 0.0), &vec3(2.6797959, -0.2013579, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 1.3,  0.2, 0.0), &vec3(2.2797959, -0.2013579, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 1.7, -0.2, 0.0), &vec3(2.6797959,  0.2013579, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 1.3, -0.2, 0.0), &vec3(2.2797959,  0.2013579, 0.0), &PARAMS_1_3);

        // around the other end of the rail, reverse rotation
        assert_world_to_joint(&vec3( 3.2,  0.2, 0.0), &vec3(2.2202041, -2.9402347, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 2.8,  0.2, 0.0), &vec3(1.8202041, -2.9402347, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 3.2, -0.2, 0.0), &vec3(2.2202041,  2.9402347, 0.0), &PARAMS_1_3);
        assert_world_to_joint(&vec3( 2.8, -0.2, 0.0), &vec3(1.8202041,  2.9402347, 0.0), &PARAMS_1_3);
    }

    #[test]
    fn world_to_joint_invalid() {
        // change x
        assert_world_to_joint_invalid(&vec3(-1.1,  0.0, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3( 4.1,  0.0, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3(-1e9,  0.0, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3( 1e9,  0.0, 0.0), &PARAMS_1_3);

        // change y
        assert_world_to_joint_invalid(&vec3( 0.0, -1.1, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3( 0.0,  1.1, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3( 0.0, -1e9, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3( 0.0,  1e9, 0.0), &PARAMS_1_3);

        // points in the angles
        assert_world_to_joint_invalid(&vec3(-0.8,  0.8, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3(-0.8, -0.8, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3( 3.8,  0.8, 0.0), &PARAMS_1_3);
        assert_world_to_joint_invalid(&vec3( 3.8, -0.8, 0.0), &PARAMS_1_3);
    }

    #[test]
    fn joint_to_world_test() {
        // at the rail ends
        assert_joint_to_world(&vec3(1.0, 0.0, 0.0), &vec3( 0.0,  0.0, 0.0), &PARAMS_1_3);
        assert_joint_to_world(&vec3(1.0, 0.0, 0.0), &vec3( 0.0,  0.0, 0.0), &PARAMS_1_3);

        // at other key limit points
        let pi2 = FRAC_PI_2;
        assert_joint_to_world(&vec3(0.0,  0.0, 0.0), &vec3(-1.0,  0.0, 0.0), &PARAMS_1_3);
        assert_joint_to_world(&vec3(0.0, -pi2, 0.0), &vec3( 0.0,  1.0, 0.0), &PARAMS_1_3);
        assert_joint_to_world(&vec3(0.0,  pi2, 0.0), &vec3( 0.0, -1.0, 0.0), &PARAMS_1_3);
        assert_joint_to_world(&vec3(3.0,  -PI, 0.0), &vec3( 4.0,  0.0, 0.0), &PARAMS_1_3);
        assert_joint_to_world(&vec3(3.0, -pi2, 0.0), &vec3( 3.0,  1.0, 0.0), &PARAMS_1_3);
        assert_joint_to_world(&vec3(3.0,  pi2, 0.0), &vec3( 3.0, -1.0, 0.0), &PARAMS_1_3);
    }
}
