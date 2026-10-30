"""Small parameter adapters extracted from ZED-Test, without UI or GNSS ownership."""


def init_parameters(sl, cfg):
    p = sl.InitParameters(
        camera_resolution=getattr(sl.RESOLUTION, cfg["camera_resolution"]),
        camera_fps=int(cfg["camera_fps"]),
        depth_mode=getattr(sl.DEPTH_MODE, cfg["depth_mode"]),
        depth_minimum_distance=cfg["depth_minimum_distance"],
        depth_maximum_distance=cfg["depth_maximum_distance"],
        coordinate_units=sl.UNIT.METER,
        coordinate_system=sl.COORDINATE_SYSTEM.RIGHT_HANDED_Z_UP,
    )
    p.camera_image_flip = getattr(sl.FLIP_MODE, cfg["camera_image_flip"])
    for key in ("camera_disable_self_calib", "depth_stabilization", "enable_image_enhancement", "sensors_required"):
        setattr(p, key, cfg[key])
    return p


def tracking_parameters(sl, cfg):
    p = sl.PositionalTrackingParameters()
    for key, value in cfg.items():
        setattr(p, key, getattr(sl.POSITIONAL_TRACKING_MODE, value) if key == "mode" else value)
    return p


def mapping_parameters(sl, cfg):
    p = sl.SpatialMappingParameters(
        map_type=getattr(sl.SPATIAL_MAP_TYPE, cfg["map_type"]),
        resolution=getattr(sl.MAPPING_RESOLUTION, cfg["resolution"]),
        max_memory_usage=cfg["max_memory_usage"], save_texture=cfg["save_texture"],
        use_chunk_only=cfg["use_chunk_only"], reverse_vertex_order=cfg["reverse_vertex_order"],
    )
    if cfg["range_meter"] != -1:
        p.set_range(float(cfg["range_meter"]))
    p.stability_counter = int(cfg["stability_counter"])
    return p
