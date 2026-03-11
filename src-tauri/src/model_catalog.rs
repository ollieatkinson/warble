use crate::constants::{
    NEMOTRON_DECODER_DOWNLOAD_URL, NEMOTRON_ENCODER_DATA_DOWNLOAD_URL,
    NEMOTRON_ENCODER_DOWNLOAD_URL, NEMOTRON_TOKENIZER_DOWNLOAD_URL,
    PARAKEET_CTC_CONFIG_DOWNLOAD_URL, PARAKEET_CTC_MODEL_DATA_DOWNLOAD_URL,
    PARAKEET_CTC_MODEL_DOWNLOAD_URL, PARAKEET_CTC_PREPROCESSOR_CONFIG_DOWNLOAD_URL,
    PARAKEET_CTC_SPECIAL_TOKENS_DOWNLOAD_URL, PARAKEET_CTC_TOKENIZER_CONFIG_DOWNLOAD_URL,
    PARAKEET_CTC_TOKENIZER_DOWNLOAD_URL, PARAKEET_DECODER_DOWNLOAD_URL,
    PARAKEET_ENCODER_DOWNLOAD_URL, PARAKEET_EOU_DECODER_DOWNLOAD_URL,
    PARAKEET_EOU_ENCODER_DOWNLOAD_URL, PARAKEET_EOU_TOKENIZER_DOWNLOAD_URL,
    PARAKEET_VOCAB_DOWNLOAD_URL,
};
use crate::parakeet;
use crate::state::TranscriptionModelKind;

pub(crate) struct CatalogDownloadFile {
    pub(crate) file_name: &'static str,
    pub(crate) download_url: &'static str,
}

pub(crate) struct CatalogDownloadSpec {
    pub(crate) model_id: &'static str,
    pub(crate) model_kind: TranscriptionModelKind,
    pub(crate) display_name: &'static str,
    pub(crate) activates_as_default: bool,
    pub(crate) files: &'static [CatalogDownloadFile],
    pub(crate) model_dir_name: &'static str,
}

const PARAKEET_FILES: &[CatalogDownloadFile] = &[
    CatalogDownloadFile {
        file_name: "encoder-model.int8.onnx",
        download_url: PARAKEET_ENCODER_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "decoder_joint-model.int8.onnx",
        download_url: PARAKEET_DECODER_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "vocab.txt",
        download_url: PARAKEET_VOCAB_DOWNLOAD_URL,
    },
];

const PARAKEET_EOU_FILES: &[CatalogDownloadFile] = &[
    CatalogDownloadFile {
        file_name: "encoder.onnx",
        download_url: PARAKEET_EOU_ENCODER_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "decoder_joint.onnx",
        download_url: PARAKEET_EOU_DECODER_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "tokenizer.json",
        download_url: PARAKEET_EOU_TOKENIZER_DOWNLOAD_URL,
    },
];

const PARAKEET_CTC_FILES: &[CatalogDownloadFile] = &[
    CatalogDownloadFile {
        file_name: "model_int8.onnx",
        download_url: PARAKEET_CTC_MODEL_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "model_int8.onnx_data",
        download_url: PARAKEET_CTC_MODEL_DATA_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "config.json",
        download_url: PARAKEET_CTC_CONFIG_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "preprocessor_config.json",
        download_url: PARAKEET_CTC_PREPROCESSOR_CONFIG_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "special_tokens_map.json",
        download_url: PARAKEET_CTC_SPECIAL_TOKENS_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "tokenizer.json",
        download_url: PARAKEET_CTC_TOKENIZER_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "tokenizer_config.json",
        download_url: PARAKEET_CTC_TOKENIZER_CONFIG_DOWNLOAD_URL,
    },
];

const NEMOTRON_FILES: &[CatalogDownloadFile] = &[
    CatalogDownloadFile {
        file_name: "encoder.onnx",
        download_url: NEMOTRON_ENCODER_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "encoder.onnx.data",
        download_url: NEMOTRON_ENCODER_DATA_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "decoder_joint.onnx",
        download_url: NEMOTRON_DECODER_DOWNLOAD_URL,
    },
    CatalogDownloadFile {
        file_name: "tokenizer.model",
        download_url: NEMOTRON_TOKENIZER_DOWNLOAD_URL,
    },
];

const CATALOG_DOWNLOAD_SPECS: &[CatalogDownloadSpec] = &[
    CatalogDownloadSpec {
        model_id: "parakeet",
        model_kind: TranscriptionModelKind::Parakeet,
        display_name: "Parakeet TDT",
        activates_as_default: true,
        files: PARAKEET_FILES,
        model_dir_name: parakeet::MODEL_ID,
    },
    CatalogDownloadSpec {
        model_id: "parakeet-eou",
        model_kind: TranscriptionModelKind::Parakeet,
        display_name: "Parakeet Realtime EOU",
        activates_as_default: false,
        files: PARAKEET_EOU_FILES,
        model_dir_name: "realtime_eou_120m-v1-onnx",
    },
    CatalogDownloadSpec {
        model_id: "parakeet-ctc",
        model_kind: TranscriptionModelKind::ParakeetCtc,
        display_name: "Parakeet CTC",
        activates_as_default: false,
        files: PARAKEET_CTC_FILES,
        model_dir_name: parakeet::CTC_MODEL_ID,
    },
    CatalogDownloadSpec {
        model_id: "nemotron-streaming",
        model_kind: TranscriptionModelKind::Parakeet,
        display_name: "Nemotron Streaming",
        activates_as_default: false,
        files: NEMOTRON_FILES,
        model_dir_name: "nemotron-speech-streaming-en-0.6b",
    },
];

pub(crate) fn catalog_download_spec(model_id: &str) -> Option<&'static CatalogDownloadSpec> {
    CATALOG_DOWNLOAD_SPECS
        .iter()
        .find(|spec| spec.model_id == model_id)
}
