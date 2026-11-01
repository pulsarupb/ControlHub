import json
from types import SimpleNamespace
import unittest
from unittest.mock import Mock

import numpy as np
from mesh_export import export_mesh


class MeshExportTests(unittest.TestCase):
    def mesh(self):
        return SimpleNamespace(
            update_mesh_from_chunklist=Mock(),
            vertices=np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [1, 1, 0], [np.nan, 0, 0]]),
            triangles=np.array([[0, 1, 2], [1, 3, 2], [0, 4, 1], [0, 1, 99]]),
            colors=np.array([[255, 0, 0]] * 5),
            uv=np.array([[0, 0], [1, 0], [0, 1], [1, 1], [0, 0]]),
            apply_texture=Mock(return_value=True),
            texture=SimpleNamespace(get_data=lambda: np.zeros((8, 8, 3), dtype=np.uint8)))

    def test_preserves_topology_and_filters_invalid_faces(self):
        result = export_mesh(self.mesh(), np, Mock())
        self.assertEqual(result["triangles"], [[0, 1, 2], [1, 3, 2]])
        self.assertEqual(len(result["vertices"]), 4)
        self.assertIsNone(result["texture"])
        self.assertFalse(result["truncated"])
        json.dumps(result, allow_nan=False)

    def test_preview_limit_keeps_valid_original_faces(self):
        result = export_mesh(self.mesh(), np, Mock(), face_limit=1)
        self.assertEqual(result["triangles"], [[0, 1, 2]])
        self.assertEqual(len(result["vertices"]), 3)
        self.assertTrue(result["truncated"])

    def test_texture_and_uv_export(self):
        cv2 = Mock()
        cv2.cvtColor.side_effect = lambda image, code: image
        cv2.imencode.return_value = (True, b"jpeg")
        result = export_mesh(self.mesh(), np, cv2, textured=True)
        self.assertEqual(result["texture"], "data:image/jpeg;base64,anBlZw==")
        self.assertEqual(len(result["uv"]), len(result["vertices"]))

    def test_texture_failure_is_explicit(self):
        mesh = self.mesh()
        mesh.apply_texture.return_value = False
        with self.assertRaisesRegex(RuntimeError, "baking failed"):
            export_mesh(mesh, np, Mock(), textured=True)


if __name__ == "__main__":
    unittest.main()
