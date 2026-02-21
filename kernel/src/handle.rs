use crate::api::{ApiVersion, Compatibility, KERNEL_API_VERSION};

pub struct KernelHandle {
    _private: (),
}

impl KernelHandle {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn api_version(&self) -> ApiVersion {
        KERNEL_API_VERSION
    }

    pub fn check_compatibility(&self, _expected: ApiVersion) -> Compatibility {
        Compatibility
    }
}
