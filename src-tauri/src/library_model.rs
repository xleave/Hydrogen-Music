use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub(crate) dir_tree: Vec<Node>,
    pub(crate) loca_files_metadata: Vec<Node>,
    pub(crate) count: usize,
    pub(crate) truncated: bool,
    pub(crate) complete: bool,
    pub(crate) revision: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnchangedScanResult {
    unchanged: bool,
    count: usize,
    truncated: bool,
    complete: bool,
    revision: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ScanResponse {
    Full(ScanResult),
    Unchanged(UnchangedScanResult),
}

impl ScanResult {
    pub fn matches_revision(&self, known_revision: Option<&str>) -> bool {
        self.complete && known_revision == Some(self.revision.as_str())
    }

    pub fn into_response(self, known_revision: Option<&str>) -> ScanResponse {
        if self.matches_revision(known_revision) {
            ScanResponse::Unchanged(UnchangedScanResult {
                unchanged: true,
                count: self.count,
                truncated: self.truncated,
                complete: self.complete,
                revision: self.revision,
            })
        } else {
            ScanResponse::Full(self)
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Node {
    pub(crate) name: String,
    pub(crate) dir_path: String,
    #[serde(rename = "type")]
    pub(crate) node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) children: Option<Vec<Node>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) common: Option<CommonMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) format: Option<FormatMetadata>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommonMetadata {
    pub(crate) local_title: String,
    pub(crate) file_url: String,
    pub(crate) title: String,
    pub(crate) artists: Vec<String>,
    pub(crate) album: String,
    pub(crate) albumartist: Option<String>,
    pub(crate) date: Option<String>,
    pub(crate) genre: Vec<String>,
    pub(crate) year: Option<u32>,
    pub(crate) has_lyrics: bool,
    pub(crate) modified_at: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FormatMetadata {
    pub(crate) bitrate: Option<u32>,
    pub(crate) bits_per_sample: Option<u8>,
    pub(crate) container: String,
    pub(crate) duration: f64,
    pub(crate) sample_rate: Option<u32>,
}
