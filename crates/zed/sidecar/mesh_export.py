"""Bounded, indexed SDK mesh export. Never triangulate a sampled point cloud."""
import base64
import json


def export_mesh(mesh, np, cv2, textured=False, face_limit=20000):
    mesh.update_mesh_from_chunklist()
    texture = None
    if textured:
        if not mesh.apply_texture():
            raise RuntimeError("ZED texture baking failed; try a smaller scan")
        # apply_texture defaults to RGB, whereas OpenCV's encoder expects BGR.
        image = np.asarray(mesh.texture.get_data())
        if image.size:
            image = cv2.cvtColor(image, cv2.COLOR_RGB2BGR)
            h, w = image.shape[:2]
            scale = min(1.0, 1024 / max(h, w))
            if scale < 1:
                image = cv2.resize(image, (max(1, int(w * scale)), max(1, int(h * scale))))
            ok, encoded = cv2.imencode(".jpg", image, [cv2.IMWRITE_JPEG_QUALITY, 80])
            if ok:
                texture = "data:image/jpeg;base64," + base64.b64encode(encoded).decode("ascii")
    vertices = np.asarray(mesh.vertices, dtype=np.float32).reshape(-1, 3)
    faces = np.asarray(mesh.triangles, dtype=np.int64).reshape(-1, 3)
    if not len(vertices) or not len(faces):
        raise RuntimeError("No mesh faces available yet; scan more surfaces first")
    faces = faces[((faces >= 0) & (faces < len(vertices))).all(axis=1)]
    faces = faces[np.isfinite(vertices[faces]).all(axis=(1, 2))]
    total = len(faces)
    # Keep connected original faces, not every Nth vertex (which destroys topology).
    faces = faces[:face_limit]
    if not len(faces):
        raise RuntimeError("No valid mesh faces available")
    used, indices = np.unique(faces.reshape(-1), return_inverse=True)
    colors = np.asarray(mesh.colors, dtype=np.uint8)
    if colors.ndim != 2 or len(colors) != len(vertices) or colors.shape[1] < 3:
        colors = np.full((len(vertices), 3), 128, dtype=np.uint8)
    uv = np.asarray(mesh.uv, dtype=np.float32) if texture else np.empty((0, 2))
    has_uv = uv.shape == (len(vertices), 2) and np.isfinite(uv).all()
    result = {"vertices": vertices[used].astype(np.float64).round(4).tolist(),
            "triangles": indices.reshape(-1, 3).tolist(),
            "colors": colors[used, :3].tolist(),
            "uv": uv[used].astype(np.float64).round(6).tolist() if has_uv else [],
            "texture": texture if has_uv else None, "truncated": total > face_limit}
    # Leave headroom under the sidecar's 8 MiB JSON-line transport limit.
    if len(json.dumps(result, allow_nan=False, separators=(",", ":")).encode()) > 7 * 1024 * 1024:
        raise RuntimeError("Mesh preview exceeds the transport budget; try a smaller scan")
    return result
