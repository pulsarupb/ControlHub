# kinova-control

Native Kortex TCP/protobuf control, extracted from the sibling repository's
`crates/kinova-control`. This library has no web server, camera, ROS, language-model
or inference dependency. Vendor protocol definitions are retained under `proto/`.

Units: joint angles and orientations in degrees; Cartesian translations in meters.
IK returns a joint solution, not a collision-free path. Callers own enable/stop,
command freshness, workspace limits and operator authorization.
