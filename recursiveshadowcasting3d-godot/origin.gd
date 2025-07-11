extends MeshInstance3D

func _ready() -> void:
	(get_parent() as Display).set_origin_and_recompute(self.global_position)
