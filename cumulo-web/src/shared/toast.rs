#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toast {
    SaveFailedInvalid,
    SaveFailedStorage,
    CategoryIdTaken,
    RenameFailed,
}
