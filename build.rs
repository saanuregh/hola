#[cfg(feature = "embed")]
fn download_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("pam_hola")
        .join("dlib_models")
}

#[cfg(feature = "embed")]
fn download_and_unzip(url: &str) {
    use bzip2::read::*;

    let filename = url
        .to_string()
        .split(r#"/"#)
        .collect::<Vec<&str>>()
        .last()
        .unwrap()
        .replace(".bz2", "");
    let path = download_path().join(&filename);

    if path.exists() {
        println!("Already got '{}'", path.display());
        return;
    }

    println!("Downloading '{}'...", url);

    let response = ureq::get(url).call();
    let mut decoded = BzDecoder::new(response.into_reader());
    let mut file = std::fs::File::create(&path).unwrap();
    std::io::copy(&mut decoded, &mut file).unwrap();
}

fn main() {
    #[cfg(feature = "embed")]
    {
        if !download_path().exists() {
            std::fs::create_dir(download_path()).unwrap();
        }

        download_and_unzip(
            "https://github.com/davisking/dlib-models/raw/master/mmod_human_face_detector.dat.bz2",
        );
        download_and_unzip(
            "https://github.com/davisking/dlib-models/raw/master/dlib_face_recognition_resnet_model_v1.dat.bz2",
        );
        download_and_unzip(
            "https://github.com/davisking/dlib-models/raw/master/shape_predictor_5_face_landmarks.dat.bz2",
        );
    }
}
