fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                // "proto/pre_vote.proto",
                // "proto/request_vote.proto",
                // "proto/append_entry.proto",
                "proto/ruft.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
