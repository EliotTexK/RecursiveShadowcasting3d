use godot::{
    classes::{MeshInstance3D, StandardMaterial3D, TubeTrailMesh},
    prelude::*,
};

#[derive(GodotClass)]
#[class(init, base=MeshInstance3D)]
pub struct DebugLine3D {
    base: Base<MeshInstance3D>,
}

#[godot_api]
impl DebugLine3D {
    pub fn new(
        start: Vector3,
        end: Vector3,
        scene_prefab: &Gd<PackedScene>,
        color: Color,
    ) -> Gd<Self> {
        let mut new = scene_prefab
            .try_instantiate_as::<Self>()
            .expect("scene_prefab should be of type DebugLine3D");

        // Transform the line segment going from (-0.5, 0, 0) to (0.5, 0, 0)
        // such that it starts at start and ends at end
        let target_vector = end - start;
        let target_length = target_vector.length();
        let target_direction = (target_vector / target_length).normalized();

        // Change the line segment's length such that it matches the length of (end - start)
        let mut trail_mesh = new
            .get_mesh()
            .expect("Should have mesh here")
            .try_cast::<TubeTrailMesh>()
            .expect("Should be TubeTrailMesh here");
        trail_mesh.set_section_length(target_length / 4.0);

        // Rotate the segment such that it points in the same direction as (end - start)
        if target_direction != Vector3::RIGHT && target_direction != Vector3::LEFT {
            let axis = Vector3::RIGHT.cross(target_direction).normalized();
            let angle = (Vector3::RIGHT.dot(target_direction)).acos();
            new.rotate(axis, angle);
        }

        // Bring to center of (end - start)
        new.set_position((end + start) / 2.0);

        // Set color
        trail_mesh
            .get_material()
            .expect("Should have material here")
            .try_cast::<StandardMaterial3D>()
            .expect("Should be StandardMaterial3d")
            .set_albedo(color);

        new
    }
}
