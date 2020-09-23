use chrono::prelude::Local;
use dlib_face_recognition::*;
use image::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    error::Error,
    fs::{create_dir_all, read_to_string, File},
    path::Path,
};
use v4l::{buffer::Stream, io, prelude::*, Format, FourCC};

// Config struct to deserialize config.toml
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub core: Core,
    pub video: Video,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Core {
    pub detection_notice: bool,
    pub no_confirmation: bool,
    pub suppress_unknown: bool,
    pub suppress_timeout: bool,
    pub ignore_ssh: bool,
    pub ignore_closed_lid: bool,
    pub disabled: bool,
    pub use_cnn: bool,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Video {
    pub certainty: f64,
    pub timeout: u64,
    pub device: usize,
    pub max_height: u32,
}

// Model stuct for user face encoding
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Model {
    data: Vec<f64>,
    pub label: String,
    pub id: usize,
    pub time: i64,
}

// Main app structure
pub struct App<'a> {
    // Dlib
    cnn_detector: FaceDetectorCnn,
    detector: FaceDetector,
    encoder: FaceEncoderNetwork,
    landmarks: LandmarkPredictor,

    // Video capture
    stream: Option<io::mmap::Stream<'a>>,
    fmt: Option<Format>,

    config: Config,
    models: Vec<Model>,
    user: String,
    base_path: String,
}

impl App<'_> {
    pub fn new<P: AsRef<Path>, T: Into<String> + std::fmt::Display>(base_path: P, user: T) -> Self {
        let dlib_model_path = base_path.as_ref().join("dlib_models");
        let config_file_path = base_path.as_ref().join("config.toml");
        let content = read_to_string(&config_file_path).expect("Failed to open config file");
        let config: Config = toml::from_str(&content).expect("Failed to parse config file");
        let detector = FaceDetector::new();
        let cnn_detector =
            FaceDetectorCnn::new(dlib_model_path.join("mmod_human_face_detector.dat")).unwrap();
        let landmarks =
            LandmarkPredictor::new(dlib_model_path.join("shape_predictor_5_face_landmarks.dat"))
                .unwrap();
        let encoder = FaceEncoderNetwork::new(
            dlib_model_path.join("dlib_face_recognition_resnet_model_v1.dat"),
        )
        .unwrap();
        let model_path = base_path.as_ref().join("models");
        create_dir_all(&model_path).expect("Error creating models folder");
        let json_file_path = model_path.join(&format!("{}.dat", user));
        let models: Vec<Model> = match File::open(&json_file_path) {
            Ok(f) => serde_json::from_reader(f).expect("s"),
            Err(_) => {
                let models: Vec<Model> = Vec::new();
                let file = File::create(json_file_path).expect("Error creating initial model file");
                serde_json::to_writer(&file, &models).expect("Error writing into initial model");
                models
            }
        };
        Self {
            detector,
            cnn_detector,
            encoder,
            landmarks,
            stream: None,
            fmt: None,
            config,
            models,
            user: user.to_string(),
            base_path: base_path.as_ref().to_string_lossy().to_string(),
        }
    }

    // Start video capture
    pub fn start_capture(&mut self) {
        let mut dev = CaptureDevice::new(self.config.video.device).expect("Failed to open device");
        let mut fmt = dev.format().expect("Failed to read format");
        fmt.fourcc = FourCC::new(b"RGB3");
        dev.set_format(&fmt).expect("Failed to write format");
        self.stream =
            Some(MmapStream::with_buffers(&mut dev, 1).expect("Failed to create buffer stream"));
        self.fmt = Some(fmt)
    }

    // Processes next frame available for face encodings
    pub fn process_next_frame(&mut self) -> Option<Vec<FaceEncoding>> {
        if self.stream.is_none() {
            return None;
        }
        match self.stream.as_mut().unwrap().next() {
            Ok(buffer) => {
                let img: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_raw(
                    self.fmt.unwrap().width,
                    self.fmt.unwrap().height,
                    buffer.data(),
                )
                .unwrap();
                let width = (self.fmt.unwrap().width * self.config.video.max_height
                    / self.fmt.unwrap().height) as usize;
                let matrix = ImageMatrix::from_image(&img)
                    .resize(width, self.config.video.max_height as usize);
                let face_locations = match self.config.core.use_cnn {
                    true => self.cnn_detector.face_locations(&matrix),
                    false => self.detector.face_locations(&matrix),
                };
                let encodings: Vec<FaceEncoding> = face_locations
                    .iter()
                    .map(|r| {
                        let landmarks = self.landmarks.face_landmarks(&matrix, &r);
                        self.encoder
                            .get_face_encodings(&matrix, &[landmarks], 0)
                            .first()
                            .unwrap()
                            .clone()
                    })
                    .collect();
                if encodings.is_empty() {
                    return None;
                }
                Some(encodings)
            }
            Err(_) => None,
        }
    }

    pub fn push_model(&mut self, model: Vec<f64>, label: String) {
        self.models.push(Model {
            data: model,
            id: self.models.len(),
            label,
            time: Local::now().timestamp(),
        });
    }

    pub fn find_model(&mut self, id: usize) -> Option<usize> {
        self.models.iter().position(|x| x.id == id)
    }

    pub fn remove_model(&mut self, index: usize) {
        self.models.remove(index);
    }

    pub fn clear_models(&mut self) {
        self.models = Vec::new();
    }

    pub fn save_model(&mut self) -> Result<(), Box<dyn Error>> {
        let json_file_path = Path::new(&self.base_path)
            .join("models")
            .join(&format!("{}.dat", self.user));
        let file = File::create(json_file_path)?;
        serde_json::to_writer(&file, &self.models)?;
        Ok(())
    }

    pub fn models(&mut self) -> Vec<Model> {
        self.models.clone()
    }

    pub fn config(&mut self) -> Config {
        self.config.clone()
    }

    pub fn identify(&mut self, encoding: FaceEncoding) -> bool {
        self.models.iter().any(|x| {
            let x = FaceEncoding::new_from_vec(x.data.clone());
            let distance = encoding.distance(&x);
            distance < self.config.video.certainty
        })
    }
}
