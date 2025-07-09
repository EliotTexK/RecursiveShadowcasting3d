extends MeshInstance3D

func _enter_tree() -> void:
	(self.get_parent() as Display).set_occluded(Vector3i(self.global_position))
