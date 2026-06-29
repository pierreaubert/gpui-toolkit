#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SceneFingerprints {
    pub geometry: u64,
    pub material: u64,
    pub camera: u64,
}
