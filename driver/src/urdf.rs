use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Serialize;

#[derive(Serialize)]
struct UrdfModel {
    name: String,
    materials: Vec<UrdfMaterial>,
    links: Vec<UrdfLink>,
    joints: Vec<UrdfJoint>,
}

#[derive(Serialize)]
struct UrdfMaterial {
    name: String,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

#[derive(Serialize)]
struct UrdfLink {
    name: String,
    visuals: Vec<UrdfVisual>,
}

#[derive(Serialize)]
struct UrdfVisual {
    origin: UrdfOrigin,
    geometry: UrdfGeometry,
    material_name: Option<String>,
}

#[derive(Serialize)]
struct UrdfOrigin {
    xyz: [f32; 3],
    rpy: [f32; 3],
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum UrdfGeometry {
    #[serde(rename = "box")]
    Box { size: [f32; 3] },
    #[serde(rename = "cylinder")]
    Cylinder { radius: f32, length: f32 },
    #[serde(rename = "sphere")]
    Sphere { radius: f32 },
}

#[derive(Serialize)]
struct UrdfJoint {
    name: String,
    parent: String,
    child: String,
    origin: UrdfOrigin,
}

pub(crate) fn parse_urdf_to_json(xml: &str) -> Result<String, String> {
    let model = parse_urdf(xml)?;
    serde_json::to_string(&model).map_err(|e| format!("JSON serialization failed: {e}"))
}

fn parse_urdf(xml: &str) -> Result<UrdfModel, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut name = String::new();
    let mut materials = Vec::new();
    let mut links = Vec::new();
    let mut joints = Vec::new();

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "robot" => {
                        name = get_attr_str(e, "name");
                    }
                    "material" => {
                        let mat_name = get_attr_str(e, "name");
                        let mut mr = 0.5;
                        let mut mg = 0.5;
                        let mut mb = 0.5;
                        let mut ma = 1.0;
                        // Read child elements
                        loop {
                            match reader.read_event_into(&mut buf) {
                                Ok(Event::Start(ref child)) => {
                                    if child.name().as_ref() == b"color" {
                                        parse_color_attrs(child, &mut mr, &mut mg, &mut mb, &mut ma);
                                        let _ = reader.read_event_into(&mut buf); // consume End(color)
                                    } else {
                                        skip_element(&mut reader, &mut buf);
                                    }
                                }
                                Ok(Event::Empty(ref child)) => {
                                    if child.name().as_ref() == b"color" {
                                        parse_color_attrs(child, &mut mr, &mut mg, &mut mb, &mut ma);
                                    }
                                }
                                Ok(Event::End(ref end)) if end.name().as_ref() == b"material" => break,
                                Ok(Event::Eof) => break,
                                _ => {}
                            }
                        }
                        materials.push(UrdfMaterial {
                            name: mat_name,
                            r: mr, g: mg, b: mb, a: ma,
                        });
                    }
                    "link" => {
                        let link_name = get_attr_str(e, "name");
                        let mut visuals = Vec::new();
                        loop {
                            match reader.read_event_into(&mut buf) {
                                Ok(Event::Start(ref inner)) | Ok(Event::Empty(ref inner)) => {
                                    match inner.name().as_ref() {
                                        b"visual" => {
                                            if let Some(v) = parse_visual(&mut reader, &mut buf) {
                                                visuals.push(v);
                                            }
                                        }
                                        b"inertial" | b"collision" => {
                                            skip_element(&mut reader, &mut buf);
                                        }
                                        _ => {}
                                    }
                                }
                                Ok(Event::End(ref inner)) if inner.name().as_ref() == b"link" => break,
                                Ok(Event::Eof) => break,
                                _ => {}
                            }
                        }
                        links.push(UrdfLink { name: link_name, visuals });
                    }
                    "joint" => {
                        let joint_name = get_attr_str(e, "name");
                        let mut parent = String::new();
                        let mut child = String::new();
                        let mut origin = UrdfOrigin { xyz: [0.0, 0.0, 0.0], rpy: [0.0, 0.0, 0.0] };
                        loop {
                            match reader.read_event_into(&mut buf) {
                                Ok(Event::Start(ref inner)) | Ok(Event::Empty(ref inner)) => {
                                    match inner.name().as_ref() {
                                        b"parent" => parent = get_attr_str(inner, "link"),
                                        b"child" => child = get_attr_str(inner, "link"),
                                        b"origin" => {
                                            let xyz_s = get_attr_str(inner, "xyz");
                                            if !xyz_s.is_empty() {
                                                let p = parse_float_array(&xyz_s, 3);
                                                origin.xyz = [p[0], p[1], p[2]];
                                            }
                                            let rpy_s = get_attr_str(inner, "rpy");
                                            if !rpy_s.is_empty() {
                                                let p = parse_float_array(&rpy_s, 3);
                                                origin.rpy = [p[0], p[1], p[2]];
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Ok(Event::End(ref inner)) if inner.name().as_ref() == b"joint" => break,
                                Ok(Event::Eof) => break,
                                _ => {}
                            }
                        }
                        joints.push(UrdfJoint {
                            name: joint_name,
                            parent,
                            child,
                            origin,
                        });
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
    }

    Ok(UrdfModel { name, materials, links, joints })
}

fn parse_visual(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Option<UrdfVisual> {
    let mut origin = UrdfOrigin { xyz: [0.0, 0.0, 0.0], rpy: [0.0, 0.0, 0.0] };
    let mut geometry: Option<UrdfGeometry> = None;
    let mut material_name: Option<String> = None;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                match e.name().as_ref() {
                    b"origin" => {
                        let xyz_s = get_attr_str(e, "xyz");
                        if !xyz_s.is_empty() {
                            let p = parse_float_array(&xyz_s, 3);
                            origin.xyz = [p[0], p[1], p[2]];
                        }
                        let rpy_s = get_attr_str(e, "rpy");
                        if !rpy_s.is_empty() {
                            let p = parse_float_array(&rpy_s, 3);
                            origin.rpy = [p[0], p[1], p[2]];
                        }
                    }
                    b"geometry" => {
                        geometry = parse_geometry(reader, buf);
                    }
                    b"material" => {
                        material_name = Some(get_attr_str(e, "name"));
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"visual" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
    }

    Some(UrdfVisual {
        origin,
        geometry: geometry?,
        material_name,
    })
}

fn parse_geometry(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Option<UrdfGeometry> {
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                return match tag.as_str() {
                    "box" => {
                        let size_s = get_attr_str(e, "size");
                        let size = if !size_s.is_empty() {
                            let p = parse_float_array(&size_s, 3);
                            [p[0], p[1], p[2]]
                        } else {
                            [0.0, 0.0, 0.0]
                        };
                        Some(UrdfGeometry::Box { size })
                    }
                    "cylinder" => {
                        let radius = get_attr_str(e, "radius").parse().unwrap_or(0.0);
                        let length = get_attr_str(e, "length").parse().unwrap_or(0.0);
                        Some(UrdfGeometry::Cylinder { radius, length })
                    }
                    "sphere" => {
                        let radius = get_attr_str(e, "radius").parse().unwrap_or(0.0);
                        Some(UrdfGeometry::Sphere { radius })
                    }
                    _ => None,
                };
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"geometry" => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
    }
    None
}

fn parse_color_attrs(e: &quick_xml::events::BytesStart, r: &mut f32, g: &mut f32, b: &mut f32, a: &mut f32) {
    let rgba_s = get_attr_str(e, "rgba");
    let rgba = parse_float_array(&rgba_s, 4);
    if rgba.len() >= 4 {
        *r = rgba[0]; *g = rgba[1]; *b = rgba[2]; *a = rgba[3];
    }
}

fn skip_element(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) {
    let mut depth = 1;
    while depth > 0 {
        match reader.read_event_into(buf) {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => depth -= 1,
            Ok(Event::Eof) => break,
            _ => {}
        }
    }
}

fn get_attr_str(e: &quick_xml::events::BytesStart, key: &str) -> String {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == key.as_bytes())
        .map(|a| String::from_utf8_lossy(&a.value).to_string())
        .unwrap_or_default()
}

fn parse_float_array(s: &str, count: usize) -> Vec<f32> {
    let mut result: Vec<f32> = s
        .split_whitespace()
        .filter_map(|v| v.parse::<f32>().ok())
        .collect();
    result.resize(count, 0.0);
    result
}
