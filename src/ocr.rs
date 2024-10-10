use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

pub struct OcrClient {
    pub inner: OcrEngine,
}

impl OcrClient {
    pub fn new() -> anyhow::Result<OcrClient> {
        let engine = init_engine()?;

        tracing::info!("created OCR engine");
        Ok(OcrClient { inner: engine })
    }

    pub fn image_to_text(&self, img_source: ImageSource) -> anyhow::Result<String> {
        let ocr_input = self.inner.prepare_input(img_source)?;

        let word_rects = self.inner.detect_words(&ocr_input)?;
        let line_rects = self.inner.find_text_lines(&ocr_input, &word_rects);
        let line_texts = self.inner.recognize_text(&ocr_input, &line_rects)?;

        let lines = line_texts
            .iter()
            .flatten()
            .map(|x| {
                let mut s = x.to_string();
                s.push('\n');
                s
            })
            .filter(|l| l.len() > 1)
            .collect::<String>();

        Ok(lines)
    }
}

/// Returns a touple of two models: text detection, text recognition
fn load_models() -> anyhow::Result<(Model, Model)> {
    let text_detection = Model::load_file("./ocr/text-detection.rten")?;
    let text_recognition = Model::load_file("./ocr/text-recognition.rten")?;

    Ok((text_detection, text_recognition))
}

fn init_engine() -> anyhow::Result<OcrEngine> {
    let (detection, regonition) = load_models()?;
    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(regonition),
        ..Default::default()
    })?;

    Ok(engine)
}
