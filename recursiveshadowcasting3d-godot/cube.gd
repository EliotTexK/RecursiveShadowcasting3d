extends MeshInstance3D

@export var val = 4.0

func _enter_tree() -> void:
	(self.get_parent() as Display).set_occluded(Vector3i(self.position))
