//! Catálogo estático de artefatos baixados sob demanda (ADR-009).

#[derive(Debug, Clone)]
pub struct ModelFile {
    pub url: &'static str,
    pub filename: &'static str,
}

#[derive(Debug, Clone)]
pub struct ModelDefinition {
    pub name: &'static str,
    pub version: &'static str,
    pub size_mb_estimate: u32,
    pub files: &'static [ModelFile],
}

pub fn get_model(name: &str) -> Option<&'static ModelDefinition> {
    MODELS.iter().find(|m| m.name == name)
}

pub fn required_onboarding_models() -> &'static [&'static str] {
    &["whisper-tiny", "piper-en"]
}

const WHISPER_TINY: ModelDefinition = ModelDefinition {
    name: "whisper-tiny",
    version: "1.0.0",
    size_mb_estimate: 75,
    files: &[ModelFile {
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        filename: "ggml-tiny.bin",
    }],
};

const PIPER_EN: ModelDefinition = ModelDefinition {
    name: "piper-en",
    version: "1.0.0",
    size_mb_estimate: 63,
    files: &[
        ModelFile {
            url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx",
            filename: "en_US-lessac-medium.onnx",
        },
        ModelFile {
            url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json",
            filename: "en_US-lessac-medium.onnx.json",
        },
    ],
};

const PIPER_JA: ModelDefinition = ModelDefinition {
    name: "piper-ja",
    version: "1.0.0",
    size_mb_estimate: 63,
    files: &[
        ModelFile {
            url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/ja/ja_JP/natsuya/medium/ja_JP-natsuya-medium.onnx",
            filename: "ja_JP-natsuya-medium.onnx",
        },
        ModelFile {
            url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/ja/ja_JP/natsuya/medium/ja_JP-natsuya-medium.onnx.json",
            filename: "ja_JP-natsuya-medium.onnx.json",
        },
    ],
};

static MODELS: &[ModelDefinition] = &[WHISPER_TINY, PIPER_EN, PIPER_JA];
