//! The left-handed view and projection matrices of the Direct3D demo
//! (`Matrix4x4.CreateLookAtLeftHanded`, `Matrix4x4.CreatePerspectiveFieldOfViewLeftHanded` of
//! .NET), against the values .NET gives for the arguments of the demo.

use crate::d3d_demo::{create_look_at_left_handed, create_perspective_field_of_view_left_handed};
use ferroui_base::numerics::{Matrix4x4, Vector3, Vector4};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn the_view_of_the_demo_looks_down_the_z_axis_from_five_units_before_the_origin() {
    let view = create_look_at_left_handed(Vector3::new(0.0, 0.0, -5.0), Vector3::new(0.0, 0.0, 0.0), Vector3::UNIT_Y);
    // .NET: identity rotation, the translation (0, 0, 5).
    let expected = Matrix4x4::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 5.0, 1.0);
    assert_eq!(view, expected);
}

#[test]
fn the_projection_maps_the_near_and_far_planes_to_zero_and_one() {
    let near = 0.1f32;
    let far = 100.0f32;
    let proj = create_perspective_field_of_view_left_handed(std::f32::consts::PI / 4.0, 4.0 / 3.0, near, far);
    // .NET: m33 = far / (far - near), m34 = 1, m43 = -near * far / (far - near).
    assert!(close(proj.m33, far / (far - near)));
    assert!(close(proj.m34, 1.0));
    assert!(close(proj.m43, -near * far / (far - near)));
    let height = 1.0 / (std::f32::consts::PI / 8.0).tan();
    assert!(close(proj.m22, height));
    assert!(close(proj.m11, height / (4.0 / 3.0)));

    // A point on the near plane has the depth 0, one on the far plane the depth 1 (row vectors,
    // as in .NET).
    for (z, depth) in [(near, 0.0f32), (far, 1.0)] {
        let p = Vector4::new(0.0, 0.0, z, 1.0);
        let clip_z = p.z * proj.m33 + p.w * proj.m43;
        let clip_w = p.z * proj.m34 + p.w * proj.m44;
        assert!(close(clip_z / clip_w, depth), "z {z}: {}", clip_z / clip_w);
    }
}

#[test]
#[should_panic]
fn a_field_of_view_of_pi_is_refused() {
    create_perspective_field_of_view_left_handed(std::f32::consts::PI, 1.0, 0.1, 100.0);
}
