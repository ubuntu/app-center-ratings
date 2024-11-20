use std::path::Path;

fn init_proto() -> Result<(), Box<dyn std::error::Error>> {
    // Define the path to the output directory within the `src` folder
    let out_dir = Path::new("src/proto");
    std::fs::create_dir_all(out_dir)?;

    let descriptor_set_path = format!(
        "{}/ratings_descriptor.bin",
        std::env::var("OUT_DIR").unwrap()
    );

    let files = &[
        "../../proto/ratings_features_app.proto",
        "../../proto/ratings_features_chart.proto",
        "../../proto/ratings_features_user.proto",
        "../../proto/ratings_features_common.proto",
    ];

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional") // needed to run on GHA because it's jammy
        .build_server(true)
        .file_descriptor_set_path(descriptor_set_path)
        .out_dir(out_dir)
        .type_attribute("Category", "#[derive(sqlx::Type, strum::EnumString)]")
        .type_attribute(
            "Category",
            r#"#[strum(serialize_all = "kebab_case", ascii_case_insensitive)]"#,
        )
        .compile(files, &["../../proto"])?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_proto()?;

    Ok(())
}
