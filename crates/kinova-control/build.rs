fn main() {
    let proto = std::fs::canonicalize("proto").expect("Kinova proto directory");
    let files = ["Frame.proto", "Session.proto", "Base.proto"].map(|file| proto.join(file));
    let include = [proto];

    let descriptors = protox::compile(files, include).expect("failed to parse Kinova protos");
    prost_build::compile_fds(descriptors).expect("failed to generate Kinova prost types");

    println!("cargo:rerun-if-changed=proto");
}
