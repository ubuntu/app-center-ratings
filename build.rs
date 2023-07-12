fn main() -> Result<(), Box<dyn std::error::Error>> {
    let descriptor_set_path = format!(
        "{}/ratings_descriptor.bin",
        std::env::var("OUT_DIR").unwrap()
    );

    let files = &[
        "proto/ratings_features_app.proto",
        "proto/ratings_features_chart.proto",
        "proto/ratings_features_user.proto",
    ];

    tonic_build::configure()
        .build_server(true)
        .file_descriptor_set_path(&descriptor_set_path)
        .compile(files, &["proto"])?;

    Ok(())
}
