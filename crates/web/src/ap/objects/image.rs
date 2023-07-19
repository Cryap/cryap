#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageObject {
    #[serde(rename = "type")]
    kind: ImageType,
    pub(crate) url: Url,
}
