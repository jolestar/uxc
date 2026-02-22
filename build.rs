fn main() {
    // Compile gRPC reflection proto
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["src/adapters/grpc/reflection.proto"],
            &["src/adapters/grpc"],
        )
        .expect("Failed to compile gRPC reflection proto");
}
