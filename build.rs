fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc not found");
    std::env::set_var("PROTOC", protoc);
    prost_build::compile_protos(&["proto/faery_save.proto"], &["proto/"])
        .expect("Failed to compile proto files");
}
