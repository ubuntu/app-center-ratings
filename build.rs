use git2::Repository;
use std::path::Path;

fn init_proto() -> Result<(), Box<dyn std::error::Error>> {
    // Define the path to the output directory within the `src` folder
    let out_dir = Path::new("proto");
    std::fs::create_dir_all(out_dir)?;

    let descriptor_set_path = format!(
        "{}/ratings_descriptor.bin",
        std::env::var("OUT_DIR").unwrap()
    );

    let files = &[
        "proto/ratings_features_app.proto",
        "proto/ratings_features_chart.proto",
        "proto/ratings_features_user.proto",
        "proto/ratings_features_common.proto",
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
        .compile(files, &["proto"])?;

    Ok(())
}

fn include_build_info() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(std::env::current_dir()?)?;
    let head = repo.head()?;
    let branch = head
        .name()
        .unwrap()
        .strip_prefix("refs/heads/")
        .unwrap_or("no-branch");
    println!("cargo:rustc-env=GIT_BRANCH={}", branch);

    let commit_sha = repo.head()?.target().unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", commit_sha);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_proto()?;
    include_build_info()?;

    Ok(())
}
